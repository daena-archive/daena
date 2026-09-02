// Candidate plan builders and validators.
use super::*;

pub fn build_import_candidate_plan(
    input: ImportCandidatePlanBuild,
    overrides: &ImportMappingOverrides,
) -> Result<ImportCandidatePlan, CoreError> {
    let ImportCandidatePlanBuild {
        session_id,
        importer,
        source,
        captured_content_generation,
        current_content_generation,
        manifest_fingerprint,
        objects,
        unsupported_count,
        diagnostics,
    } = input;
    if session_id.trim().is_empty() || manifest_fingerprint.trim().is_empty() {
        return Err(CoreError::Validation(
            "candidate plan requires a session and manifest fingerprint".into(),
        ));
    }
    let mut seen_ids = BTreeSet::new();
    let mut candidate_objects = Vec::with_capacity(objects.len());
    let mut unresolved_decision_count = 0;
    for object in &objects {
        if !seen_ids.insert(&object.id) {
            return Err(CoreError::Validation(format!(
                "duplicate staged object id in candidate plan: {}",
                object.id
            )));
        }
        validate_source_path(&object.source_path)?;
        let mapping = resolve_import_mapping(object, overrides);
        let mut issues = Vec::new();
        if mapping.entity_type.is_none() {
            issues.push(ImportCandidateIssue {
                code: "entity_type_required".into(),
                message: "Choose an enabled entity type for this item.".into(),
                source_path: Some(object.source_path.clone()),
                object_id: Some(object.id.clone()),
            });
            unresolved_decision_count += 1;
        }
        candidate_objects.push(ImportCandidateObject {
            staged_object_id: object.id.clone(),
            source_id: object.source_id.clone(),
            source_path: object.source_path.clone(),
            title: object.title.clone(),
            decision: "create".into(),
            mapping,
            issues,
        });
    }
    let mut issues = Vec::new();
    if captured_content_generation != current_content_generation {
        issues.push(ImportCandidateIssue {
            code: "project_generation_changed".into(),
            message: "The project changed after analysis; analyze the source again before commit."
                .into(),
            source_path: None,
            object_id: None,
        });
    }
    let mut plan = ImportCandidatePlan {
        schema_version: IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION,
        plan_id: String::new(),
        session_id,
        importer,
        source,
        captured_content_generation,
        current_content_generation,
        manifest_fingerprint,
        objects: candidate_objects,
        unsupported_count,
        diagnostics,
        issues,
        unresolved_decision_count,
    };
    let bytes = serde_json::to_vec(&plan).map_err(|error| {
        CoreError::Validation(format!("candidate plan serialization failed: {error}"))
    })?;
    plan.plan_id = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(plan)
}

pub fn validate_import_candidate_plan(
    input: ImportValidationBuild,
) -> Result<ImportValidationOutcome, CoreError> {
    let ImportValidationBuild {
        candidate,
        staged_objects,
        staged_assets,
        staged_unsupported,
        catalog,
        decisions,
        existing_targets,
        duplicate_targets,
    } = input;
    let staged = staged_objects
        .into_iter()
        .map(|object| (object.id.clone(), object))
        .collect::<BTreeMap<_, _>>();
    let source_diagnostics = candidate.diagnostics.clone();
    let mut issues = Vec::new();
    if candidate.captured_content_generation != candidate.current_content_generation {
        issues.push(validation_issue(
            ImportValidationSeverity::Error,
            "project_generation_changed",
            "The project changed after analysis. Analyze the source again.",
            None,
            None,
            None,
        ));
    }
    if candidate.manifest_fingerprint != catalog.fingerprint {
        issues.push(validation_issue(
            ImportValidationSeverity::Error,
            "manifest_changed",
            "Enabled schema contributions changed. Review the mappings again.",
            None,
            None,
            None,
        ));
    }
    for diagnostic in &candidate.diagnostics {
        let severity = match diagnostic.severity {
            ImportDiagnosticSeverity::Warning => ImportValidationSeverity::Warning,
            ImportDiagnosticSeverity::Fatal | ImportDiagnosticSeverity::Error => {
                ImportValidationSeverity::Error
            }
        };
        issues.push(validation_issue(
            severity,
            &diagnostic.code,
            &diagnostic.message,
            diagnostic.source_path.clone(),
            diagnostic.object_id.clone(),
            None,
        ));
    }
    if candidate.unsupported_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "unsupported_source_data",
            &format!(
                "{} unsupported source item(s) will not be imported.",
                candidate.unsupported_count
            ),
            None,
            None,
            None,
        ));
    }
    let candidate_ids = candidate
        .objects
        .iter()
        .map(|object| object.staged_object_id.as_str())
        .collect::<BTreeSet<_>>();
    for decision_id in decisions.keys() {
        if !candidate_ids.contains(decision_id.as_str()) {
            return Err(CoreError::Validation(format!(
                "decision references unknown staged object: {decision_id}"
            )));
        }
    }
    let mut validated = Vec::with_capacity(candidate.objects.len());
    let mut unmapped_field_count = 0_usize;
    let mut unmapped_field_objects = 0_usize;
    let mut unmapped_source_link_count = 0_usize;
    for candidate_object in &candidate.objects {
        let Some(object) = staged.get(&candidate_object.staged_object_id) else {
            return Err(CoreError::Validation(format!(
                "candidate object is missing from staged data: {}",
                candidate_object.staged_object_id
            )));
        };
        let decision = decisions
            .get(&object.id)
            .cloned()
            .unwrap_or(ImportObjectDecision::Create);
        let mut entity_type = None;
        let mut fields = Vec::new();
        match &decision {
            ImportObjectDecision::Skip => {}
            ImportObjectDecision::MapToExisting {
                entity_id,
                expected_revision,
            } => match existing_targets.get(entity_id) {
                Some(target) if target.revision == *expected_revision => {
                    entity_type = target.entity_type.clone();
                }
                Some(_) => issues.push(validation_issue(
                    ImportValidationSeverity::Error,
                    "existing_target_revision_changed",
                    "The selected existing entity changed. Select it again.",
                    Some(object.source_path.clone()),
                    Some(object.id.clone()),
                    Some(entity_id.clone()),
                )),
                None => issues.push(validation_issue(
                    ImportValidationSeverity::Error,
                    "existing_target_missing",
                    "The selected existing entity no longer exists.",
                    Some(object.source_path.clone()),
                    Some(object.id.clone()),
                    Some(entity_id.clone()),
                )),
            },
            ImportObjectDecision::Create => {
                if !decisions.contains_key(&object.id) {
                    if let Some(existing) = duplicate_targets
                        .get(&object.id)
                        .and_then(|targets| targets.first())
                    {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Error,
                            "duplicate_source_identity",
                            "This source was imported before. Choose create, skip, or map to existing explicitly.",
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            Some(existing.clone()),
                        ));
                    }
                }
                let selected_type = candidate_object
                    .mapping
                    .entity_type
                    .as_deref()
                    .filter(|value| catalog.entity_types.contains(*value));
                match selected_type {
                    Some(selected) => entity_type = Some(selected.into()),
                    None => issues.push(validation_issue(
                        ImportValidationSeverity::Error,
                        "entity_type_unavailable",
                        "Choose an entity type contributed by an enabled plugin.",
                        Some(object.source_path.clone()),
                        Some(object.id.clone()),
                        None,
                    )),
                }
                let mut mapped_target_ids = BTreeSet::new();
                for (source_key, target_id) in &candidate_object.mapping.field_mappings {
                    let Some(value) = object.fields.get(source_key) else {
                        // Folder/global mappings mean "map this key when present". Wiki
                        // infoboxes and Obsidian frontmatter are intentionally sparse.
                        continue;
                    };
                    let Some(target) = catalog.fields.get(target_id) else {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Error,
                            "target_field_unavailable",
                            &format!("The mapped field {target_id} is not available."),
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            None,
                        ));
                        continue;
                    };
                    if let Some(selected) = entity_type.as_deref() {
                        if !target.entity_types.is_empty()
                            && !target.entity_types.contains(selected)
                        {
                            issues.push(validation_issue(
                                ImportValidationSeverity::Error,
                                "target_field_scope_mismatch",
                                &format!("The field {target_id} does not apply to {selected}."),
                                Some(object.source_path.clone()),
                                Some(object.id.clone()),
                                None,
                            ));
                            continue;
                        }
                    }
                    if !import_field_value_matches(target, value) {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Error,
                            "target_field_value_invalid",
                            &format!(
                                "The source value for {source_key} does not match the schema for {target_id}."
                            ),
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            None,
                        ));
                        continue;
                    }
                    mapped_target_ids.insert(target_id.clone());
                    fields.push(ValidatedImportField {
                        source_key: source_key.clone(),
                        namespace: target.namespace.clone(),
                        key: target.key.clone(),
                        value: value.clone(),
                    });
                }
                if let Some(selected) = entity_type.as_deref() {
                    for (target_id, target) in &catalog.fields {
                        if target.required
                            && (target.entity_types.is_empty()
                                || target.entity_types.contains(selected))
                            && !mapped_target_ids.contains(target_id)
                        {
                            issues.push(validation_issue(
                                ImportValidationSeverity::Error,
                                "required_target_field_missing",
                                &format!(
                                    "The required field {target_id} has no mapped source value."
                                ),
                                Some(object.source_path.clone()),
                                Some(object.id.clone()),
                                None,
                            ));
                        }
                    }
                }
                let object_unmapped = object
                    .fields
                    .keys()
                    .filter(|source_key| {
                        !candidate_object
                            .mapping
                            .field_mappings
                            .contains_key(*source_key)
                    })
                    .count();
                if object_unmapped > 0 {
                    unmapped_field_count += object_unmapped;
                    unmapped_field_objects += 1;
                }
                unmapped_source_link_count += object
                    .links
                    .iter()
                    .filter(|link| {
                        !candidate_object
                            .mapping
                            .relationship_mappings
                            .contains_key(staged_link_kind_key(&link.kind))
                    })
                    .count();
            }
        }
        let unmapped_fields = object
            .fields
            .iter()
            .filter(|(source_key, _)| {
                !matches!(&decision, ImportObjectDecision::Create)
                    || !candidate_object
                        .mapping
                        .field_mappings
                        .contains_key(*source_key)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        validated.push(ValidatedImportObject {
            staged_object_id: object.id.clone(),
            source_id: object.source_id.clone(),
            source_path: object.source_path.clone(),
            content_hash: object.content_hash.clone(),
            title: object.title.clone(),
            entity_type,
            document: object.body.clone(),
            fields,
            source_context: ValidatedImportSourceContext {
                source_kind: object.source_kind.clone(),
                parent_source_path: object.parent_source_path.clone(),
                tags: object.tags.clone(),
                aliases: object.aliases.clone(),
                metadata: object.metadata.clone(),
                unmapped_fields,
                links: object.links.clone(),
            },
            decision,
        });
    }
    if unmapped_field_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "unmapped_source_fields_preserved",
            &format!(
                "{unmapped_field_count} unmapped source field(s) across {unmapped_field_objects} item(s) will remain in import source metadata."
            ),
            None,
            None,
            None,
        ));
    }
    if unmapped_source_link_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "source_links_preserved",
            &format!(
                "{unmapped_source_link_count} unmapped source link(s) will remain in the document and import source metadata; only explicitly mapped, resolved links become relationships."
            ),
            None,
            None,
            None,
        ));
    }

    let decisions_by_object = validated
        .iter()
        .map(|object| (object.staged_object_id.as_str(), &object.decision))
        .collect::<BTreeMap<_, _>>();
    let mut relationship_keys = BTreeSet::new();
    let mut validated_relationships = Vec::new();
    let mut unresolved_mapped_link_count = 0_usize;
    let mut skipped_target_link_count = 0_usize;
    for candidate_object in &candidate.objects {
        let object = staged
            .get(&candidate_object.staged_object_id)
            .expect("candidate staged object was checked above");
        if matches!(
            decisions_by_object.get(object.id.as_str()),
            Some(ImportObjectDecision::Skip)
        ) {
            continue;
        }
        for (source_kind, relationship_type) in &candidate_object.mapping.relationship_mappings {
            if !catalog.relationship_types.contains(relationship_type) {
                issues.push(validation_issue(
                    ImportValidationSeverity::Error,
                    "target_relationship_unavailable",
                    &format!("The mapped relationship type {relationship_type} is not available."),
                    Some(object.source_path.clone()),
                    Some(object.id.clone()),
                    None,
                ));
                continue;
            }
            for link in object
                .links
                .iter()
                .filter(|link| staged_link_kind_key(&link.kind) == source_kind)
            {
                if link.resolution != StagedLinkResolution::Resolved {
                    unresolved_mapped_link_count += 1;
                    continue;
                }
                let target_id = link
                    .resolved_object_id
                    .as_deref()
                    .expect("resolved links were checked by staged import validation");
                if matches!(
                    decisions_by_object.get(target_id),
                    Some(ImportObjectDecision::Skip)
                ) {
                    skipped_target_link_count += 1;
                    continue;
                }
                if !decisions_by_object.contains_key(target_id) {
                    return Err(CoreError::Validation(format!(
                        "resolved import relationship target is missing: {target_id}"
                    )));
                }
                let key = (
                    object.id.clone(),
                    target_id.to_owned(),
                    relationship_type.clone(),
                );
                if relationship_keys.insert(key.clone()) {
                    validated_relationships.push(ValidatedImportRelationship {
                        source_staged_object_id: key.0,
                        target_staged_object_id: key.1,
                        relationship_type: key.2,
                        source_kind: source_kind.clone(),
                        source_target: link.target.clone(),
                    });
                }
            }
        }
    }
    if unresolved_mapped_link_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "mapped_links_unresolved",
            &format!(
                "{unresolved_mapped_link_count} mapped link(s) were ambiguous, missing, or external and will not become relationships."
            ),
            None,
            None,
            None,
        ));
    }
    if skipped_target_link_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "mapped_link_targets_skipped",
            &format!(
                "{skipped_target_link_count} mapped link(s) target skipped items and will not become relationships."
            ),
            None,
            None,
            None,
        ));
    }
    let has_errors = issues
        .iter()
        .any(|issue| issue.severity == ImportValidationSeverity::Error);
    if has_errors {
        return Ok(ImportValidationOutcome { plan: None, issues });
    }
    let mut validated_assets = Vec::new();
    for asset in staged_assets {
        let Some(owner_id) = asset.owner_object_id.as_deref() else {
            issues.push(validation_issue(
                ImportValidationSeverity::Warning,
                "unreferenced_asset_skipped",
                "This unreferenced asset will not be imported.",
                Some(asset.source_path),
                None,
                None,
            ));
            continue;
        };
        if matches!(
            decisions_by_object.get(owner_id),
            Some(ImportObjectDecision::Skip)
        ) {
            continue;
        }
        if !decisions_by_object.contains_key(owner_id) {
            issues.push(validation_issue(
                ImportValidationSeverity::Error,
                "asset_owner_unavailable",
                "The entity selected for this asset is not available.",
                Some(asset.source_path),
                Some(owner_id.into()),
                None,
            ));
            continue;
        }
        let Some(content_hash) = asset.content_hash else {
            issues.push(validation_issue(
                ImportValidationSeverity::Error,
                "asset_hash_missing",
                "The asset did not produce a content hash during analysis.",
                Some(asset.source_path),
                Some(owner_id.into()),
                None,
            ));
            continue;
        };
        let Some(mime_type) = asset.mime_type else {
            issues.push(validation_issue(
                ImportValidationSeverity::Error,
                "asset_mime_type_missing",
                "The asset did not produce a media type during analysis.",
                Some(asset.source_path),
                Some(owner_id.into()),
                None,
            ));
            continue;
        };
        validated_assets.push(ValidatedImportAsset {
            staged_asset_id: asset.id,
            owner_staged_object_id: owner_id.into(),
            source_path: asset.source_path,
            filename: asset.filename,
            content_hash,
            size: asset.size,
            mime_type,
        });
    }
    let has_errors = issues
        .iter()
        .any(|issue| issue.severity == ImportValidationSeverity::Error);
    if has_errors {
        return Ok(ImportValidationOutcome { plan: None, issues });
    }
    let warnings = issues.clone();
    let mut plan = ValidatedImportPlan {
        schema_version: VALIDATED_IMPORT_PLAN_SCHEMA_VERSION,
        plan_id: String::new(),
        candidate_plan_id: candidate.plan_id,
        session_id: candidate.session_id,
        importer: candidate.importer,
        source: candidate.source,
        content_generation: candidate.current_content_generation,
        manifest_fingerprint: catalog.fingerprint,
        objects: validated,
        relationships: validated_relationships,
        assets: validated_assets,
        unsupported: staged_unsupported,
        diagnostics: source_diagnostics,
        warnings,
    };
    let bytes =
        serde_json::to_vec(&plan).map_err(|error| CoreError::Serialization(error.to_string()))?;
    plan.plan_id = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(ImportValidationOutcome {
        plan: Some(plan),
        issues,
    })
}

pub(super) fn staged_link_kind_key(kind: &StagedLinkKind) -> &'static str {
    match kind {
        StagedLinkKind::Internal => "internal",
        StagedLinkKind::External => "external",
        StagedLinkKind::Embed => "embed",
    }
}

pub(super) fn validation_issue(
    severity: ImportValidationSeverity,
    code: &str,
    message: &str,
    source_path: Option<String>,
    object_id: Option<String>,
    existing_entity_id: Option<String>,
) -> ImportValidationIssue {
    ImportValidationIssue {
        severity,
        code: code.into(),
        message: message.into(),
        source_path,
        object_id,
        existing_entity_id,
    }
}

pub(super) fn resolve_import_mapping(
    object: &StagedObject,
    overrides: &ImportMappingOverrides,
) -> ImportCandidateMapping {
    let mut resolved = ImportCandidateMapping {
        entity_type: None,
        field_mappings: BTreeMap::new(),
        relationship_mappings: BTreeMap::new(),
    };
    apply_mapping_decision(&mut resolved, &overrides.global);
    for category in object.tags.iter().collect::<BTreeSet<_>>() {
        if let Some(decision) = overrides.categories.get(category) {
            apply_mapping_decision(&mut resolved, decision);
        }
    }
    let segments = object.source_path.split('/').collect::<Vec<_>>();
    for end in 1..segments.len() {
        let folder = segments[..end].join("/");
        if let Some(decision) = overrides.folders.get(&folder) {
            apply_mapping_decision(&mut resolved, decision);
        }
    }
    if let Some(decision) = overrides.items.get(&object.id) {
        apply_mapping_decision(&mut resolved, decision);
    }
    resolved
}

pub(super) fn import_field_value_matches(
    target: &ImportFieldTarget,
    value: &serde_json::Value,
) -> bool {
    if value.is_null() {
        return !target.required;
    }
    match target.field_type.as_str() {
        "text" => value.is_string(),
        "date" => value.is_string() || value.is_object(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "enum" if target.multiple => value.as_array().is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| target.options.contains(value))
                })
        }),
        "enum" => value
            .as_str()
            .is_some_and(|value| target.options.contains(value)),
        "oneof" => {
            target
                .one_of
                .iter()
                .filter(|variant| import_field_variant_matches(variant, value))
                .count()
                == 1
        }
        _ => false,
    }
}

pub(super) fn import_field_variant_matches(
    variant: &ImportFieldVariant,
    value: &serde_json::Value,
) -> bool {
    match variant.field_type.as_str() {
        "text" => value.is_string(),
        "date" => value.is_string() || value.is_object(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "enum" => value
            .as_str()
            .is_some_and(|value| variant.options.contains(value)),
        _ => false,
    }
}

pub(super) fn apply_mapping_decision(
    target: &mut ImportCandidateMapping,
    decision: &ImportMappingDecision,
) {
    if let Some(entity_type) = decision
        .entity_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        target.entity_type = Some(entity_type.into());
    }
    for (source, destination) in &decision.field_mappings {
        if !source.trim().is_empty() && !destination.trim().is_empty() {
            target
                .field_mappings
                .insert(source.clone(), destination.clone());
        }
    }
    for (source, destination) in &decision.relationship_mappings {
        if !source.trim().is_empty() && !destination.trim().is_empty() {
            target
                .relationship_mappings
                .insert(source.clone(), destination.clone());
        }
    }
}

impl StagedImport {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != STAGED_IMPORT_SCHEMA_VERSION {
            return Err(CoreError::Validation(format!(
                "unsupported staged import schema version: {}",
                self.schema_version
            )));
        }
        if self.importer.id.trim().is_empty() || self.importer.version.trim().is_empty() {
            return Err(CoreError::Validation(
                "staged import importer id and version are required".into(),
            ));
        }
        if self.importer.name.trim().is_empty()
            || self.source.id.trim().is_empty()
            || self.source.display_name.trim().is_empty()
        {
            return Err(CoreError::Validation(
                "staged import importer name, source id, and source display name are required"
                    .into(),
            ));
        }

        let mut object_ids = BTreeSet::new();
        let mut source_ids = BTreeSet::new();
        for object in &self.objects {
            if object.id.trim().is_empty() || !object_ids.insert(object.id.as_str()) {
                return Err(CoreError::Validation(
                    "staged import object ids must be non-empty and unique".into(),
                ));
            }
            if object.source_id.trim().is_empty() || !source_ids.insert(object.source_id.as_str()) {
                return Err(CoreError::Validation(
                    "staged import source ids must be non-empty and unique".into(),
                ));
            }
            if object.title.trim().is_empty() {
                return Err(CoreError::Validation(
                    "staged import object title cannot be empty".into(),
                ));
            }
            if object.source_kind.trim().is_empty() || object.content_hash.trim().is_empty() {
                return Err(CoreError::Validation(
                    "staged import objects require a source kind and content hash".into(),
                ));
            }
            validate_source_path(&object.source_path)?;
            if let Some(parent) = &object.parent_source_path {
                validate_source_path(parent)?;
            }
            if let Some(body) = &object.body {
                if body.format.trim().is_empty() {
                    return Err(CoreError::Validation(
                        "staged import document format cannot be empty".into(),
                    ));
                }
            }
            validate_non_empty_unique_values("tag", &object.tags)?;
            validate_non_empty_unique_values("alias", &object.aliases)?;
            if object.fields.keys().any(|key| key.trim().is_empty())
                || object.metadata.keys().any(|key| key.trim().is_empty())
                || object
                    .raw_source_data
                    .keys()
                    .any(|key| key.trim().is_empty())
            {
                return Err(CoreError::Validation(
                    "staged import field and metadata keys cannot be empty".into(),
                ));
            }
            for hint in &object.mapping_hints {
                if hint
                    .source_key
                    .as_deref()
                    .is_some_and(|source_key| source_key.trim().is_empty())
                {
                    return Err(CoreError::Validation(
                        "staged import mapping hint source key cannot be empty".into(),
                    ));
                }
                if hint.confidence.is_some_and(|confidence| {
                    !confidence.is_finite() || !(0.0..=1.0).contains(&confidence)
                }) {
                    return Err(CoreError::Validation(
                        "staged import mapping hint confidence must be between zero and one".into(),
                    ));
                }
            }
            validate_diagnostics(&object.diagnostics)?;
        }

        for object in &self.objects {
            for link in &object.links {
                if link.target.trim().is_empty() {
                    return Err(CoreError::Validation(
                        "staged import link target cannot be empty".into(),
                    ));
                }
                match link.resolution {
                    StagedLinkResolution::Resolved => {
                        let target = link.resolved_object_id.as_deref().ok_or_else(|| {
                            CoreError::Validation(
                                "resolved staged import links require an object id".into(),
                            )
                        })?;
                        if !object_ids.contains(target) {
                            return Err(CoreError::Validation(
                                "resolved staged import link references an unknown object".into(),
                            ));
                        }
                    }
                    StagedLinkResolution::Ambiguous => {
                        let unique_candidates = link
                            .candidate_object_ids
                            .iter()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>();
                        if unique_candidates.len() < 2
                            || unique_candidates.len() != link.candidate_object_ids.len()
                            || unique_candidates
                                .iter()
                                .any(|candidate| !object_ids.contains(candidate))
                        {
                            return Err(CoreError::Validation(
                                "ambiguous staged import links require at least two unique known candidates"
                                    .into(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut asset_ids = BTreeSet::new();
        for asset in &self.assets {
            if asset.id.trim().is_empty() || !asset_ids.insert(asset.id.as_str()) {
                return Err(CoreError::Validation(
                    "staged import asset ids must be non-empty and unique".into(),
                ));
            }
            validate_source_path(&asset.source_path)?;
            validate_portable_basename(&asset.filename)?;
            if asset
                .owner_object_id
                .as_deref()
                .is_some_and(|owner| !object_ids.contains(owner))
            {
                return Err(CoreError::Validation(
                    "staged import asset references an unknown owner object".into(),
                ));
            }
            validate_diagnostics(&asset.diagnostics)?;
        }
        for unsupported in &self.unsupported {
            validate_source_path(&unsupported.source_path)?;
            if unsupported.source_kind.trim().is_empty() || unsupported.reason.trim().is_empty() {
                return Err(CoreError::Validation(
                    "unsupported staged data requires a source kind and reason".into(),
                ));
            }
        }
        validate_diagnostics(&self.diagnostics)?;
        for diagnostic in self
            .diagnostics
            .iter()
            .chain(self.objects.iter().flat_map(|object| &object.diagnostics))
            .chain(self.assets.iter().flat_map(|asset| &asset.diagnostics))
        {
            if diagnostic
                .object_id
                .as_deref()
                .is_some_and(|object_id| !object_ids.contains(object_id))
            {
                return Err(CoreError::Validation(
                    "staged import diagnostic references an unknown object".into(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn refresh_summary(&mut self, folder_count: usize, total_source_bytes: u64) {
        let object_diagnostics = self.objects.iter().flat_map(|object| &object.diagnostics);
        let asset_diagnostics = self.assets.iter().flat_map(|asset| &asset.diagnostics);
        let diagnostics = self
            .diagnostics
            .iter()
            .chain(object_diagnostics)
            .chain(asset_diagnostics)
            .collect::<Vec<_>>();
        self.summary = ImportAnalysisSummary {
            document_count: self
                .objects
                .iter()
                .filter(|object| object.body.is_some())
                .count(),
            candidate_entity_count: self.objects.len(),
            folder_count,
            asset_count: self.assets.len(),
            link_count: self.objects.iter().map(|object| object.links.len()).sum(),
            unresolved_link_count: self
                .objects
                .iter()
                .flat_map(|object| &object.links)
                .filter(|link| {
                    matches!(
                        link.resolution,
                        StagedLinkResolution::Unresolved
                            | StagedLinkResolution::Ambiguous
                            | StagedLinkResolution::Missing
                    )
                })
                .count(),
            unsupported_count: self.unsupported.len(),
            warning_count: diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == ImportDiagnosticSeverity::Warning)
                .count(),
            error_count: diagnostics
                .iter()
                .filter(|diagnostic| {
                    matches!(
                        diagnostic.severity,
                        ImportDiagnosticSeverity::Fatal | ImportDiagnosticSeverity::Error
                    )
                })
                .count(),
            total_source_bytes,
        };
    }
}
