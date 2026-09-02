// Generic document and vault analysis engine.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImportProfile {
    Generic,
    Obsidian,
}

pub(super) fn analyze_documents_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    profile: ImportProfile,
    mut progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    validate_limits(&limits)?;
    let source = source.as_ref();
    let metadata = fs::symlink_metadata(source).map_err(|source| CoreError::Io {
        operation: "read import source metadata",
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CoreError::Validation(
            "import source root cannot be a symbolic link".into(),
        ));
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(CoreError::Validation(
            "import source must be a regular file or directory".into(),
        ));
    }
    if profile == ImportProfile::Obsidian && !metadata.is_dir() {
        return Err(CoreError::Validation(
            "Obsidian import requires a vault folder".into(),
        ));
    }

    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Selected source")
        .to_owned();
    let canonical_source = fs::canonicalize(source).map_err(|source| CoreError::Io {
        operation: "resolve import source path",
        source,
    })?;
    let source_id = hex_digest(canonical_source.to_string_lossy().as_bytes());
    let is_zip_archive = metadata.is_file() && is_zip_path(source);
    let source_kind = if profile == ImportProfile::Obsidian {
        ImportSourceKind::Vault
    } else if metadata.is_dir() {
        ImportSourceKind::Folder
    } else if is_zip_archive {
        ImportSourceKind::Archive
    } else {
        ImportSourceKind::File
    };
    let (importer_id, importer_version, importer_name) = match profile {
        ImportProfile::Generic => (
            GENERIC_DOCUMENT_IMPORTER_ID,
            GENERIC_DOCUMENT_IMPORTER_VERSION,
            "Generic documents",
        ),
        ImportProfile::Obsidian => (
            OBSIDIAN_IMPORTER_ID,
            OBSIDIAN_IMPORTER_VERSION,
            "Obsidian vault",
        ),
    };
    let mut analyzer = GenericDocumentAnalyzer {
        profile,
        limits,
        import: StagedImport {
            schema_version: STAGED_IMPORT_SCHEMA_VERSION,
            importer: ImporterIdentity {
                id: importer_id.into(),
                version: importer_version.into(),
                name: importer_name.into(),
            },
            source: ImportSource {
                id: source_id,
                kind: source_kind,
                display_name: source_name.clone(),
            },
            objects: Vec::new(),
            assets: Vec::new(),
            unsupported: Vec::new(),
            diagnostics: Vec::new(),
            summary: ImportAnalysisSummary::default(),
        },
        discovered_entries: usize::from(metadata.is_file() && !is_zip_archive),
        discovered_files: 0,
        processed_entries: 0,
        total_source_bytes: 0,
        folders: BTreeSet::new(),
        progress: &mut progress,
    };

    analyzer.report_progress(None)?;

    if metadata.is_dir() {
        analyzer.analyze_directory(source, &[], 0)?;
    } else if is_zip_archive {
        analyzer.analyze_archive(source, metadata.len())?;
    } else {
        analyzer.analyze_file(source, &source_name, &metadata)?;
        analyzer.finish_entry(Some(source_name))?;
    }
    if profile == ImportProfile::Obsidian {
        analyzer.resolve_obsidian_references()?;
    } else {
        analyzer.resolve_markdown_references()?;
    }
    analyzer
        .import
        .objects
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .assets
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .unsupported
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .refresh_summary(analyzer.folders.len(), analyzer.total_source_bytes);
    analyzer.import.validate()?;
    Ok(analyzer.import)
}

pub(super) struct GenericDocumentAnalyzer<'a> {
    profile: ImportProfile,
    limits: GenericDocumentImportLimits,
    import: StagedImport,
    discovered_entries: usize,
    discovered_files: usize,
    processed_entries: usize,
    total_source_bytes: u64,
    folders: BTreeSet<String>,
    progress: &'a mut dyn FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
}

#[derive(Debug)]
pub(super) struct PendingAssetAttachment {
    asset_index: usize,
    owner_object_id: String,
    target: String,
}

impl GenericDocumentAnalyzer<'_> {
    pub(super) fn analyze_archive(
        &mut self,
        path: &Path,
        compressed_bytes: u64,
    ) -> Result<(), CoreError> {
        if compressed_bytes > MAX_ARCHIVE_COMPRESSED_BYTES {
            return Err(CoreError::Validation(format!(
                "ZIP archive exceeds the maximum compressed size of {MAX_ARCHIVE_COMPRESSED_BYTES} bytes"
            )));
        }
        let file = fs::File::open(path).map_err(|source| CoreError::Io {
            operation: "open import ZIP archive",
            source,
        })?;
        let mut archive = ZipArchive::new(file)
            .map_err(|error| CoreError::Validation(format!("invalid ZIP archive: {error}")))?;
        if archive.len() > self.limits.max_entries {
            return Err(CoreError::Validation(format!(
                "ZIP archive exceeds the maximum entry count of {}",
                self.limits.max_entries
            )));
        }

        struct ArchiveEntryPlan {
            index: usize,
            source_path: String,
            is_dir: bool,
            size: u64,
        }
        let mut entries = Vec::with_capacity(archive.len());
        let mut names = BTreeSet::new();
        let mut folded_names = BTreeSet::new();
        let mut expanded_bytes = 0_u64;
        for index in 0..archive.len() {
            let entry = archive.by_index(index).map_err(|error| {
                CoreError::Validation(format!("invalid ZIP central-directory entry: {error}"))
            })?;
            let is_dir = entry.is_dir();
            let source_path = validate_archive_source_path(entry.name_raw(), is_dir)?;
            self.report_progress(Some(source_path.clone()))?;
            if !names.insert(source_path.clone())
                || !folded_names.insert(source_path.to_lowercase())
            {
                return Err(CoreError::Validation(format!(
                    "ZIP archive contains duplicate or case-colliding path: {source_path}"
                )));
            }
            if !is_dir
                && entry
                    .unix_mode()
                    .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
            {
                return Err(CoreError::Validation(format!(
                    "ZIP links and special files are not allowed: {source_path}"
                )));
            }
            let depth = source_path.split('/').count().saturating_sub(1);
            if depth > self.limits.max_depth {
                return Err(CoreError::Validation(format!(
                    "ZIP entry exceeds the maximum folder depth of {}: {source_path}",
                    self.limits.max_depth
                )));
            }
            let size = entry.size();
            if !is_dir && size > self.limits.max_file_bytes {
                return Err(CoreError::Validation(format!(
                    "ZIP entry '{source_path}' exceeds the maximum file size of {} bytes",
                    self.limits.max_file_bytes
                )));
            }
            expanded_bytes = expanded_bytes
                .checked_add(size)
                .ok_or_else(|| CoreError::Validation("ZIP expanded size overflowed".into()))?;
            if expanded_bytes > self.limits.max_total_bytes {
                return Err(CoreError::Validation(format!(
                    "ZIP archive exceeds the maximum expanded size of {} bytes",
                    self.limits.max_total_bytes
                )));
            }
            let packed = entry.compressed_size();
            if size > 0
                && (packed == 0 || size > packed.saturating_mul(MAX_ARCHIVE_COMPRESSION_RATIO))
            {
                return Err(CoreError::Validation(format!(
                    "ZIP entry exceeds the maximum compression ratio of {MAX_ARCHIVE_COMPRESSION_RATIO}:1: {source_path}"
                )));
            }
            entries.push(ArchiveEntryPlan {
                index,
                source_path,
                is_dir,
                size,
            });
        }

        self.discovered_entries = entries.len();
        for planned in entries {
            record_parent_folders(&mut self.folders, &planned.source_path);
            if planned.is_dir {
                self.folders.insert(planned.source_path.clone());
                self.finish_entry(Some(planned.source_path))?;
                continue;
            }
            self.discovered_files = self.discovered_files.saturating_add(1);
            if self.discovered_files > self.limits.max_files {
                return Err(CoreError::Validation(format!(
                    "ZIP archive exceeds the maximum file count of {}",
                    self.limits.max_files
                )));
            }
            if asset_mime_type(&planned.source_path).is_none()
                && document_format(&planned.source_path).is_none()
            {
                self.record_unsupported(
                    planned.source_path.clone(),
                    "archive_entry",
                    "file type is not supported by the generic document importer",
                )?;
                self.finish_entry(Some(planned.source_path))?;
                continue;
            }
            let next_total = self
                .total_source_bytes
                .checked_add(planned.size)
                .ok_or_else(|| {
                    CoreError::Validation("import source byte count overflowed".into())
                })?;
            let mut entry = archive.by_index(planned.index).map_err(|error| {
                CoreError::Validation(format!("invalid ZIP entry data: {error}"))
            })?;
            let mut bytes = Vec::with_capacity(planned.size.min(1024 * 1024) as usize);
            entry
                .by_ref()
                .take(planned.size.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|source| CoreError::Io {
                    operation: "read import ZIP entry",
                    source,
                })?;
            if bytes.len() as u64 != planned.size {
                return Err(CoreError::Validation(format!(
                    "ZIP entry size does not match its central-directory declaration: {}",
                    planned.source_path
                )));
            }
            drop(entry);
            self.analyze_loaded_file(&planned.source_path, planned.size, next_total, bytes)?;
            self.finish_entry(Some(planned.source_path))?;
        }
        Ok(())
    }

    pub(super) fn analyze_directory(
        &mut self,
        directory: &Path,
        relative_parts: &[String],
        depth: usize,
    ) -> Result<(), CoreError> {
        if depth > self.limits.max_depth {
            return Err(CoreError::Validation(format!(
                "import source exceeds the maximum folder depth of {}",
                self.limits.max_depth
            )));
        }
        let entries = fs::read_dir(directory).map_err(|source| CoreError::Io {
            operation: "read import source directory",
            source,
        })?;
        let mut named_entries = Vec::new();
        for entry in entries {
            self.report_progress(None)?;
            let entry = entry.map_err(|source| CoreError::Io {
                operation: "read import source directory entry",
                source,
            })?;
            self.discovered_entries = self.discovered_entries.saturating_add(1);
            if self.discovered_entries > self.limits.max_entries {
                return Err(CoreError::Validation(format!(
                    "import source exceeds the maximum entry count of {}",
                    self.limits.max_entries
                )));
            }
            let name = match entry.file_name().into_string() {
                Ok(name) if !name.is_empty() => name,
                _ => {
                    self.record_unsupported(
                        non_utf8_entry_label(relative_parts),
                        "filesystem_entry",
                        "entry name is not valid UTF-8",
                    )?;
                    continue;
                }
            };
            named_entries.push((name, entry.path()));
        }
        named_entries.sort_by(|left, right| left.0.cmp(&right.0));

        for (name, path) in named_entries {
            let mut child_parts = relative_parts.to_vec();
            child_parts.push(name);
            let relative_path = child_parts.join("/");
            self.report_progress(Some(relative_path.clone()))?;
            let metadata = fs::symlink_metadata(&path).map_err(|source| CoreError::Io {
                operation: "read import source entry metadata",
                source,
            })?;
            if metadata.file_type().is_symlink() {
                self.record_unsupported(
                    relative_path.clone(),
                    "symlink",
                    "symbolic links are not followed during import analysis",
                )?;
            } else if metadata.is_dir()
                && self.profile == ImportProfile::Obsidian
                && child_parts.len() == 1
                && matches!(child_parts[0].as_str(), ".obsidian" | ".trash")
            {
                self.record_unsupported(
                    relative_path.clone(),
                    "obsidian_configuration",
                    "Obsidian configuration and trash folders are intentionally excluded",
                )?;
            } else if metadata.is_dir() {
                self.folders.insert(relative_path.clone());
                self.analyze_directory(&path, &child_parts, depth + 1)?;
            } else if metadata.is_file() {
                self.analyze_file(&path, &relative_path, &metadata)?;
            } else {
                self.record_unsupported(
                    relative_path.clone(),
                    "filesystem_entry",
                    "entry is not a regular file or directory",
                )?;
            }
            self.finish_entry(Some(relative_path))?;
        }
        Ok(())
    }

    pub(super) fn analyze_file(
        &mut self,
        path: &Path,
        source_path: &str,
        metadata: &fs::Metadata,
    ) -> Result<(), CoreError> {
        self.discovered_files = self.discovered_files.saturating_add(1);
        if self.discovered_files > self.limits.max_files {
            return Err(CoreError::Validation(format!(
                "import source exceeds the maximum file count of {}",
                self.limits.max_files
            )));
        }
        let size = metadata.len();
        if size > self.limits.max_file_bytes {
            return Err(CoreError::Validation(format!(
                "import file '{source_path}' exceeds the maximum size of {} bytes",
                self.limits.max_file_bytes
            )));
        }
        let next_total = self
            .total_source_bytes
            .checked_add(size)
            .ok_or_else(|| CoreError::Validation("import source byte count overflowed".into()))?;
        if next_total > self.limits.max_total_bytes {
            return Err(CoreError::Validation(format!(
                "import source exceeds the maximum total size of {} bytes",
                self.limits.max_total_bytes
            )));
        }
        if asset_mime_type(source_path).is_none() && !self.supports_document(source_path) {
            return self.record_unsupported(
                source_path.to_owned(),
                "file",
                if self.profile == ImportProfile::Obsidian {
                    "file type is not supported by the Obsidian vault importer"
                } else {
                    "file type is not supported by the generic document importer"
                },
            );
        }
        let bytes = fs::read(path).map_err(|source| CoreError::Io {
            operation: "read import source file",
            source,
        })?;
        if bytes.len() as u64 != size {
            return Err(CoreError::Conflict(format!(
                "import file '{source_path}' changed during analysis"
            )));
        }
        self.analyze_loaded_file(source_path, size, next_total, bytes)
    }

    pub(super) fn supports_document(&self, source_path: &str) -> bool {
        match self.profile {
            ImportProfile::Generic => document_format(source_path).is_some(),
            ImportProfile::Obsidian => document_format(source_path) == Some("markdown"),
        }
    }

    pub(super) fn analyze_loaded_file(
        &mut self,
        source_path: &str,
        size: u64,
        next_total: u64,
        bytes: Vec<u8>,
    ) -> Result<(), CoreError> {
        self.total_source_bytes = next_total;
        if let Some(mime_type) = asset_mime_type(source_path) {
            if !asset_signature_matches(mime_type, &bytes) {
                self.record_unsupported(
                    source_path.to_owned(),
                    "asset",
                    "asset bytes do not match the supported file signature",
                )?;
                self.record_diagnostic(ImportDiagnostic {
                    severity: ImportDiagnosticSeverity::Error,
                    code: "invalid_asset_content".into(),
                    message: "asset bytes do not match the supported file signature".into(),
                    source_path: Some(source_path.to_owned()),
                    object_id: None,
                })?;
                return Ok(());
            }
            let filename = source_path
                .rsplit('/')
                .next()
                .unwrap_or(source_path)
                .to_owned();
            let source_id = hex_digest(
                format!(
                    "{}\0{}\0asset\0{}",
                    self.import.importer.id, self.import.source.id, source_path
                )
                .as_bytes(),
            );
            self.import.assets.push(StagedAsset {
                id: source_id,
                source_path: source_path.to_owned(),
                filename,
                size,
                mime_type: Some(mime_type.into()),
                content_hash: Some(format!("sha256:{}", hex_digest(&bytes))),
                owner_object_id: None,
                relationship: Some("attachment".into()),
                raw_metadata: BTreeMap::new(),
                diagnostics: Vec::new(),
            });
            return Ok(());
        }
        let source_format =
            document_format(source_path).expect("supported file format was checked");
        if source_format == "docx" {
            return self.analyze_docx_file(source_path, size, bytes);
        }
        let mut body = if let Ok(body) = String::from_utf8(bytes) {
            body
        } else {
            self.record_unsupported(
                source_path.to_owned(),
                "document",
                "document content is not valid UTF-8",
            )?;
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Error,
                code: "invalid_utf8".into(),
                message: "document content is not valid UTF-8".into(),
                source_path: Some(source_path.to_owned()),
                object_id: None,
            })?;
            return Ok(());
        };
        let content_hash = hex_digest(body.as_bytes());
        let source_id = hex_digest(
            format!(
                "{}\0{}\0{}",
                self.import.importer.id, self.import.source.id, source_path
            )
            .as_bytes(),
        );
        let mut title = document_title(source_path);
        let parent_source_path = source_path
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_owned());
        let mut body_format = source_format;
        let frontmatter = (source_format == "markdown")
            .then(|| markdown_frontmatter(&body).map(str::to_owned))
            .flatten();
        let mut fields = BTreeMap::new();
        let mut raw_source_data = BTreeMap::new();
        let mut aliases = Vec::new();
        let mut tags = Vec::new();
        let mut mapping_hints = Vec::new();
        if let Some(frontmatter) = &frontmatter {
            raw_source_data.insert(
                "frontmatter".into(),
                serde_json::Value::String(frontmatter.clone()),
            );
            if self.profile == ImportProfile::Obsidian {
                let parsed = parse_obsidian_frontmatter(frontmatter);
                fields = parsed.fields;
                aliases = parsed.aliases;
                tags = parsed.tags;
                if let Some(entity_type) = parsed.entity_type_hint {
                    mapping_hints.push(StagedMappingHint {
                        kind: MappingHintKind::EntityType,
                        source_key: Some("type".into()),
                        suggested_value: serde_json::Value::String(entity_type),
                        confidence: Some(0.85),
                        reason: Some("Obsidian YAML frontmatter type".into()),
                    });
                }
                for message in parsed.warnings {
                    self.record_diagnostic(ImportDiagnostic {
                        severity: ImportDiagnosticSeverity::Warning,
                        code: "obsidian_frontmatter_partial".into(),
                        message,
                        source_path: Some(source_path.to_owned()),
                        object_id: Some(source_id.clone()),
                    })?;
                }
            } else {
                fields.insert(
                    "frontmatter".into(),
                    serde_json::Value::String(frontmatter.clone()),
                );
            }
        }
        let mut metadata = BTreeMap::new();
        if source_format == "html" {
            let conversion = convert_html_to_markdown(&body)?;
            if let Some(html_title) = conversion.title {
                title = html_title;
            }
            raw_source_data.insert("html".into(), serde_json::Value::String(body));
            body = conversion.markdown;
            body_format = "markdown";
            metadata.insert(
                "converted_from".into(),
                serde_json::Value::String("html".into()),
            );
            for warning in conversion.warnings {
                self.record_diagnostic(ImportDiagnostic {
                    severity: ImportDiagnosticSeverity::Warning,
                    code: warning.code.into(),
                    message: warning.message,
                    source_path: Some(source_path.to_owned()),
                    object_id: Some(source_id.clone()),
                })?;
            }
        }
        let links = if body_format == "markdown" {
            let link_body = if self.profile == ImportProfile::Obsidian {
                markdown_body_after_frontmatter(&body)
            } else {
                &body
            };
            let mut links = discover_markdown_links(link_body);
            if self.profile == ImportProfile::Obsidian {
                links.extend(discover_obsidian_links(link_body));
            }
            links
        } else {
            Vec::new()
        };
        if frontmatter.is_some() {
            metadata.insert(
                "frontmatter_format".into(),
                serde_json::Value::String("yaml".into()),
            );
        }
        self.import.objects.push(StagedObject {
            id: source_id.clone(),
            source_id,
            source_kind: if self.profile == ImportProfile::Obsidian {
                "obsidian_markdown".into()
            } else {
                source_format.to_owned()
            },
            source_path: source_path.to_owned(),
            content_hash,
            title,
            body: Some(StagedDocument {
                format: body_format.to_owned(),
                body,
            }),
            parent_source_path,
            tags,
            aliases,
            fields,
            metadata,
            raw_source_data,
            links,
            mapping_hints,
            diagnostics: Vec::new(),
        });
        Ok(())
    }

    pub(super) fn analyze_docx_file(
        &mut self,
        source_path: &str,
        size: u64,
        bytes: Vec<u8>,
    ) -> Result<(), CoreError> {
        let content_hash = hex_digest(&bytes);
        let source_id = hex_digest(
            format!(
                "{}\0{}\0{}",
                self.import.importer.id, self.import.source.id, source_path
            )
            .as_bytes(),
        );
        let conversion = convert_docx_to_markdown(&bytes, source_path)?;
        let title = conversion
            .title
            .unwrap_or_else(|| document_title(source_path));
        for warning in conversion.warnings {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: warning.code.into(),
                message: warning.message,
                source_path: Some(source_path.to_owned()),
                object_id: Some(source_id.clone()),
            })?;
        }
        for asset in conversion.assets {
            let asset_source_path = format!("{source_path}!/{}", asset.entry_path);
            let asset_id = hex_digest(
                format!(
                    "{}\0{}\0asset\0{}",
                    self.import.importer.id, self.import.source.id, asset_source_path
                )
                .as_bytes(),
            );
            self.import.assets.push(StagedAsset {
                id: asset_id,
                source_path: asset_source_path,
                filename: asset.filename,
                size: asset.bytes.len() as u64,
                mime_type: Some(asset.mime_type.into()),
                content_hash: Some(format!("sha256:{}", hex_digest(&asset.bytes))),
                owner_object_id: None,
                relationship: Some("attachment".into()),
                raw_metadata: BTreeMap::from([(
                    "docx_entry".into(),
                    serde_json::Value::String(asset.entry_path),
                )]),
                diagnostics: Vec::new(),
            });
        }
        let links = discover_markdown_links(&conversion.markdown);
        self.import.objects.push(StagedObject {
            id: source_id.clone(),
            source_id,
            source_kind: "docx".into(),
            source_path: source_path.to_owned(),
            content_hash,
            title,
            body: Some(StagedDocument {
                format: "markdown".into(),
                body: conversion.markdown,
            }),
            parent_source_path: source_path
                .rsplit_once('/')
                .map(|(parent, _)| parent.to_owned()),
            tags: Vec::new(),
            aliases: Vec::new(),
            fields: BTreeMap::new(),
            metadata: BTreeMap::from([
                (
                    "converted_from".into(),
                    serde_json::Value::String("docx".into()),
                ),
                (
                    "package_entry_count".into(),
                    serde_json::Value::from(conversion.package_entry_count),
                ),
                ("source_size".into(), serde_json::Value::from(size)),
            ]),
            raw_source_data: {
                let mut raw = BTreeMap::from([(
                    "package_entries".into(),
                    serde_json::Value::Array(
                        conversion
                            .package_entries
                            .into_iter()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                )]);
                if let Some(value) = conversion.core_properties {
                    raw.insert(
                        "core_properties_xml".into(),
                        serde_json::Value::String(value),
                    );
                }
                raw
            },
            links,
            mapping_hints: Vec::new(),
            diagnostics: Vec::new(),
        });
        Ok(())
    }

    pub(super) fn resolve_markdown_references(&mut self) -> Result<(), CoreError> {
        let objects_by_path = self
            .import
            .objects
            .iter()
            .map(|object| (object.source_path.clone(), object.id.clone()))
            .collect::<BTreeMap<_, _>>();
        let assets_by_path = self
            .import
            .assets
            .iter()
            .enumerate()
            .map(|(index, asset)| (asset.source_path.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut missing = Vec::new();
        let mut attachments = Vec::new();
        for object in &mut self.import.objects {
            for link in &mut object.links {
                if is_external_markdown_target(&link.target) {
                    link.resolution = StagedLinkResolution::NotApplicable;
                    continue;
                }
                let Some(target_path) =
                    resolve_relative_source_path(&object.source_path, &link.target)
                else {
                    link.resolution = StagedLinkResolution::Missing;
                    missing.push((
                        object.id.clone(),
                        object.source_path.clone(),
                        link.target.clone(),
                    ));
                    continue;
                };
                if let Some(target_id) = objects_by_path.get(&target_path) {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(target_id.clone());
                } else if let Some(asset_index) = assets_by_path.get(&target_path) {
                    stage_asset_attachment(
                        &object.id,
                        &mut object.mapping_hints,
                        link,
                        &target_path,
                        *asset_index,
                        "standard Markdown file reference",
                        &mut attachments,
                    );
                } else {
                    link.resolution = StagedLinkResolution::Missing;
                    missing.push((
                        object.id.clone(),
                        object.source_path.clone(),
                        link.target.clone(),
                    ));
                }
            }
        }
        apply_asset_attachments(&mut self.import.assets, attachments);
        for (object_id, source_path, target) in missing {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: "markdown_target_missing".into(),
                message: format!(
                    "Markdown target '{target}' was not found in the selected source."
                ),
                source_path: Some(source_path),
                object_id: Some(object_id),
            })?;
        }
        Ok(())
    }

    pub(super) fn resolve_obsidian_references(&mut self) -> Result<(), CoreError> {
        let objects_by_path = self
            .import
            .objects
            .iter()
            .map(|object| (object.source_path.clone(), object.id.clone()))
            .collect::<BTreeMap<_, _>>();
        let assets_by_path = self
            .import
            .assets
            .iter()
            .enumerate()
            .map(|(index, asset)| (asset.source_path.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut object_keys = BTreeMap::<String, BTreeSet<String>>::new();
        for object in &self.import.objects {
            for key in std::iter::once(object.source_path.as_str())
                .chain(std::iter::once(obsidian_path_without_markdown_extension(
                    &object.source_path,
                )))
                .chain(
                    Path::new(&object.source_path)
                        .file_stem()
                        .and_then(|value| value.to_str()),
                )
                .chain(std::iter::once(object.title.as_str()))
                .chain(object.aliases.iter().map(String::as_str))
            {
                object_keys
                    .entry(obsidian_lookup_key(key))
                    .or_default()
                    .insert(object.id.clone());
            }
        }
        let mut asset_keys = BTreeMap::<String, BTreeSet<usize>>::new();
        for (index, asset) in self.import.assets.iter().enumerate() {
            for key in [asset.source_path.as_str(), asset.filename.as_str()] {
                asset_keys
                    .entry(obsidian_lookup_key(key))
                    .or_default()
                    .insert(index);
            }
        }

        struct PendingDiagnostic {
            code: &'static str,
            message: String,
            source_path: String,
            object_id: String,
        }
        let mut diagnostics = Vec::new();
        let mut attachments = Vec::new();
        let (objects, assets) = (&mut self.import.objects, &mut self.import.assets);
        for object in objects {
            for link in &mut object.links {
                if is_external_markdown_target(&link.target) {
                    link.resolution = StagedLinkResolution::NotApplicable;
                    continue;
                }
                if link.raw.is_none() {
                    let Some(target_path) =
                        resolve_relative_source_path(&object.source_path, &link.target)
                    else {
                        link.resolution = StagedLinkResolution::Missing;
                        diagnostics.push(PendingDiagnostic {
                            code: "markdown_target_missing",
                            message: format!(
                                "Markdown target '{}' was not found in the selected vault.",
                                link.target
                            ),
                            source_path: object.source_path.clone(),
                            object_id: object.id.clone(),
                        });
                        continue;
                    };
                    if let Some(target_id) = objects_by_path.get(&target_path) {
                        link.resolution = StagedLinkResolution::Resolved;
                        link.resolved_object_id = Some(target_id.clone());
                    } else if let Some(asset_index) = assets_by_path.get(&target_path) {
                        stage_asset_attachment(
                            &object.id,
                            &mut object.mapping_hints,
                            link,
                            &target_path,
                            *asset_index,
                            "Obsidian attachment or embed",
                            &mut attachments,
                        );
                    } else {
                        link.resolution = StagedLinkResolution::Missing;
                        diagnostics.push(PendingDiagnostic {
                            code: "markdown_target_missing",
                            message: format!(
                                "Markdown target '{}' was not found in the selected vault.",
                                link.target
                            ),
                            source_path: object.source_path.clone(),
                            object_id: object.id.clone(),
                        });
                    }
                    continue;
                }

                let target = obsidian_target_path(&link.target);
                if target.is_empty() {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(object.id.clone());
                    continue;
                }
                let candidate_paths = obsidian_candidate_paths(&object.source_path, target);
                let mut object_candidates = BTreeSet::new();
                for path in &candidate_paths {
                    if let Some(candidates) = object_keys.get(&obsidian_lookup_key(path)) {
                        object_candidates.extend(candidates.iter().cloned());
                    }
                    if Path::new(path).extension().is_none() {
                        let markdown_path = format!("{path}.md");
                        if let Some(candidates) =
                            object_keys.get(&obsidian_lookup_key(&markdown_path))
                        {
                            object_candidates.extend(candidates.iter().cloned());
                        }
                    }
                }
                if object_candidates.is_empty() {
                    for key in obsidian_fallback_keys(target) {
                        if let Some(candidates) = object_keys.get(&key) {
                            object_candidates.extend(candidates.iter().cloned());
                        }
                    }
                }
                let mut asset_candidates = BTreeSet::new();
                for path in &candidate_paths {
                    if let Some(candidates) = asset_keys.get(&obsidian_lookup_key(path)) {
                        asset_candidates.extend(candidates.iter().copied());
                    }
                }
                if asset_candidates.is_empty() {
                    for key in obsidian_fallback_keys(target) {
                        if let Some(candidates) = asset_keys.get(&key) {
                            asset_candidates.extend(candidates.iter().copied());
                        }
                    }
                }

                let prefer_asset =
                    link.kind == StagedLinkKind::Embed && obsidian_target_looks_like_asset(target);
                if prefer_asset && asset_candidates.len() == 1 {
                    let index = *asset_candidates.iter().next().expect("one candidate");
                    let target_path = assets[index].source_path.clone();
                    stage_asset_attachment(
                        &object.id,
                        &mut object.mapping_hints,
                        link,
                        &target_path,
                        index,
                        "Obsidian attachment or embed",
                        &mut attachments,
                    );
                } else if object_candidates.len() == 1 {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = object_candidates.into_iter().next();
                } else if object_candidates.len() > 1 {
                    link.resolution = StagedLinkResolution::Ambiguous;
                    link.candidate_object_ids = object_candidates.into_iter().collect();
                    diagnostics.push(PendingDiagnostic {
                        code: "obsidian_target_ambiguous",
                        message: format!(
                            "Obsidian target '{}' matches multiple notes.",
                            link.target
                        ),
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                } else if asset_candidates.len() == 1 {
                    let index = *asset_candidates.iter().next().expect("one candidate");
                    let target_path = assets[index].source_path.clone();
                    stage_asset_attachment(
                        &object.id,
                        &mut object.mapping_hints,
                        link,
                        &target_path,
                        index,
                        "Obsidian attachment or embed",
                        &mut attachments,
                    );
                } else {
                    link.resolution = StagedLinkResolution::Missing;
                    let (code, message) = if asset_candidates.len() > 1 {
                        (
                            "obsidian_asset_ambiguous",
                            format!(
                                "Obsidian target '{}' matches multiple attachments.",
                                link.target
                            ),
                        )
                    } else {
                        (
                            "obsidian_target_missing",
                            format!(
                                "Obsidian target '{}' was not found in the vault.",
                                link.target
                            ),
                        )
                    };
                    diagnostics.push(PendingDiagnostic {
                        code,
                        message,
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                }
            }
        }
        apply_asset_attachments(assets, attachments);
        for diagnostic in diagnostics {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: diagnostic.code.into(),
                message: diagnostic.message,
                source_path: Some(diagnostic.source_path),
                object_id: Some(diagnostic.object_id),
            })?;
        }
        Ok(())
    }

    pub(super) fn record_unsupported(
        &mut self,
        source_path: String,
        source_kind: &str,
        reason: &str,
    ) -> Result<(), CoreError> {
        self.import.unsupported.push(UnsupportedSourceData {
            source_path: source_path.clone(),
            source_kind: source_kind.into(),
            reason: reason.into(),
            raw_metadata: BTreeMap::new(),
        });
        self.record_diagnostic(ImportDiagnostic {
            severity: ImportDiagnosticSeverity::Warning,
            code: "unsupported_source_entry".into(),
            message: reason.into(),
            source_path: Some(source_path),
            object_id: None,
        })
    }

    pub(super) fn record_diagnostic(
        &mut self,
        diagnostic: ImportDiagnostic,
    ) -> Result<(), CoreError> {
        if self.import.diagnostics.len() >= self.limits.max_diagnostics {
            return Err(CoreError::Validation(format!(
                "import analysis exceeds the maximum diagnostic count of {}",
                self.limits.max_diagnostics
            )));
        }
        self.import.diagnostics.push(diagnostic);
        Ok(())
    }

    pub(super) fn finish_entry(&mut self, source_path: Option<String>) -> Result<(), CoreError> {
        self.processed_entries = self.processed_entries.saturating_add(1);
        self.report_progress(source_path)
    }

    pub(super) fn report_progress(&mut self, source_path: Option<String>) -> Result<(), CoreError> {
        (self.progress)(ImportAnalysisProgress {
            processed_entries: self.processed_entries,
            staged_object_count: self.import.objects.len(),
            unsupported_count: self.import.unsupported.len(),
            source_bytes: self.total_source_bytes,
            source_path,
        })
    }
}

pub(super) fn stage_asset_attachment(
    object_id: &str,
    mapping_hints: &mut Vec<StagedMappingHint>,
    link: &mut StagedLink,
    target_path: &str,
    asset_index: usize,
    reason: &str,
    attachments: &mut Vec<PendingAssetAttachment>,
) {
    link.resolution = StagedLinkResolution::NotApplicable;
    attachments.push(PendingAssetAttachment {
        asset_index,
        owner_object_id: object_id.into(),
        target: link.target.clone(),
    });
    mapping_hints.push(StagedMappingHint {
        kind: MappingHintKind::AssetRelationship,
        source_key: Some(target_path.into()),
        suggested_value: serde_json::Value::String("attachment".into()),
        confidence: Some(1.0),
        reason: Some(reason.into()),
    });
}

pub(super) fn apply_asset_attachments(
    assets: &mut Vec<StagedAsset>,
    attachments: Vec<PendingAssetAttachment>,
) {
    let mut by_asset = BTreeMap::<usize, BTreeMap<String, BTreeSet<String>>>::new();
    for attachment in attachments {
        by_asset
            .entry(attachment.asset_index)
            .or_default()
            .entry(attachment.owner_object_id)
            .or_default()
            .insert(attachment.target);
    }
    let mut additional_assets = Vec::new();
    for (asset_index, owners) in by_asset {
        let Some(asset) = assets.get_mut(asset_index) else {
            continue;
        };
        let original_id = asset.id.clone();
        let original_metadata = asset.raw_metadata.clone();
        for (owner_index, (owner_object_id, targets)) in owners.into_iter().enumerate() {
            let target_metadata = if targets.len() == 1 {
                serde_json::Value::String(targets.into_iter().next().expect("one target"))
            } else {
                serde_json::Value::Array(
                    targets.into_iter().map(serde_json::Value::String).collect(),
                )
            };
            if owner_index == 0 {
                asset.owner_object_id = Some(owner_object_id);
                asset
                    .raw_metadata
                    .insert("resolved_from".into(), target_metadata);
                continue;
            }
            let mut duplicate = asset.clone();
            duplicate.id =
                hex_digest(format!("{original_id}\0owner\0{owner_object_id}").as_bytes());
            duplicate.owner_object_id = Some(owner_object_id);
            duplicate.raw_metadata = original_metadata.clone();
            duplicate
                .raw_metadata
                .insert("resolved_from".into(), target_metadata);
            additional_assets.push(duplicate);
        }
    }
    assets.extend(additional_assets);
}

pub(super) fn obsidian_path_without_markdown_extension(path: &str) -> &str {
    path.get(path.len().saturating_sub(3)..)
        .filter(|suffix| suffix.eq_ignore_ascii_case(".md"))
        .map_or(path, |_| &path[..path.len() - 3])
}

pub(super) fn obsidian_lookup_key(value: &str) -> String {
    value.trim().trim_start_matches('/').to_lowercase()
}

pub(super) fn obsidian_target_path(target: &str) -> &str {
    let target = target
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(target);
    target
        .split_once('^')
        .map(|(path, _)| path)
        .unwrap_or(target)
        .trim()
}

pub(super) fn obsidian_candidate_paths(source_path: &str, target: &str) -> Vec<String> {
    if target.contains('\\') || target.chars().any(char::is_control) {
        return Vec::new();
    }
    let target = target.trim();
    let root_target = target.trim_start_matches('/');
    let mut paths = BTreeSet::new();
    if !target.starts_with('/') {
        if let Some(relative) = resolve_relative_source_path(source_path, target) {
            paths.insert(relative);
        }
    }
    if !root_target.is_empty() {
        paths.insert(root_target.into());
    }
    paths.into_iter().collect()
}

pub(super) fn obsidian_fallback_keys(target: &str) -> Vec<String> {
    let target = target.trim().trim_start_matches('/');
    let mut keys = BTreeSet::new();
    keys.insert(obsidian_lookup_key(target));
    keys.insert(obsidian_lookup_key(
        obsidian_path_without_markdown_extension(target),
    ));
    if let Some(filename) = Path::new(target)
        .file_name()
        .and_then(|value| value.to_str())
    {
        keys.insert(obsidian_lookup_key(filename));
    }
    if let Some(stem) = Path::new(target)
        .file_stem()
        .and_then(|value| value.to_str())
    {
        keys.insert(obsidian_lookup_key(stem));
    }
    keys.into_iter().filter(|key| !key.is_empty()).collect()
}

pub(super) fn obsidian_target_looks_like_asset(target: &str) -> bool {
    Path::new(target)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| !extension.eq_ignore_ascii_case("md"))
}
