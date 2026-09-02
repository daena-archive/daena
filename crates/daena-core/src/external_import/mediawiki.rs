// MediaWiki XML import analyzer.
use super::*;

pub(super) const MAX_MEDIAWIKI_SOURCE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub(super) const MAX_MEDIAWIKI_XML_DEPTH: usize = 128;
pub(super) const MAX_MEDIAWIKI_TEMPLATES_PER_PAGE: usize = 512;
pub(super) const MAX_MEDIAWIKI_TEMPLATE_DEPTH: usize = 64;

#[derive(Debug, Default)]
pub(super) struct MediaWikiRevision {
    id: String,
    parent_id: String,
    timestamp: String,
    model: String,
    format: String,
    sha1: String,
    contributor: String,
    text: String,
}

#[derive(Debug, Default)]
pub(super) struct MediaWikiPage {
    title: String,
    namespace_id: String,
    id: String,
    redirect_target: Option<String>,
    revision: Option<MediaWikiRevision>,
    current_revision: Option<MediaWikiRevision>,
    revision_count: usize,
}

pub(super) struct MediaWikiAnalyzer<'a> {
    pub(super) limits: GenericDocumentImportLimits,
    pub(super) import: StagedImport,
    pub(super) source_name: String,
    pub(super) namespaces: BTreeMap<String, String>,
    pub(super) site_metadata: BTreeMap<String, String>,
    pub(super) folders: BTreeSet<String>,
    pub(super) processed_pages: usize,
    pub(super) total_wikitext_bytes: u64,
    pub(super) total_diagnostics: usize,
    pub(super) omitted_revisions: usize,
    pub(super) progress: &'a mut dyn FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
}

impl MediaWikiAnalyzer<'_> {
    pub(super) fn parse(&mut self, file: fs::File) -> Result<(), CoreError> {
        let mut reader = XmlReader::from_reader(BufReader::new(file));
        reader.config_mut().trim_text(false);
        reader.config_mut().check_end_names = true;
        let mut buffer = Vec::with_capacity(64 * 1024);
        let mut stack = Vec::<Vec<u8>>::new();
        let mut page = None::<MediaWikiPage>;
        let mut pending_namespace = None::<(String, String)>;
        let mut root_seen = false;
        let mut event_count = 0_u64;
        loop {
            let event = reader.read_event_into(&mut buffer).map_err(|error| {
                CoreError::Validation(format!(
                    "invalid MediaWiki XML near byte {}: {error}",
                    reader.error_position()
                ))
            })?;
            event_count = event_count.saturating_add(1);
            if event_count.is_multiple_of(512) {
                self.report_progress(reader.buffer_position(), None)?;
            }
            match event {
                XmlEvent::Start(start) => {
                    let name = xml_local_name(start.name().as_ref()).to_vec();
                    if !root_seen {
                        if name.as_slice() != b"mediawiki" {
                            return Err(CoreError::Validation(
                                "XML source root must be a MediaWiki export".into(),
                            ));
                        }
                        root_seen = true;
                    }
                    if stack.len() >= MAX_MEDIAWIKI_XML_DEPTH {
                        return Err(CoreError::Validation(format!(
                            "MediaWiki XML exceeds the maximum nesting depth of {MAX_MEDIAWIKI_XML_DEPTH}"
                        )));
                    }
                    if name.as_slice() == b"page" {
                        if page.is_some() {
                            return Err(CoreError::Validation(
                                "MediaWiki XML contains nested page elements".into(),
                            ));
                        }
                        page = Some(MediaWikiPage::default());
                    } else if name.as_slice() == b"revision" {
                        let current_page = page.as_mut().ok_or_else(|| {
                            CoreError::Validation(
                                "MediaWiki revision appeared outside a page".into(),
                            )
                        })?;
                        if current_page.current_revision.is_some() {
                            return Err(CoreError::Validation(
                                "MediaWiki XML contains nested revision elements".into(),
                            ));
                        }
                        current_page.current_revision = Some(MediaWikiRevision::default());
                        current_page.revision_count = current_page.revision_count.saturating_add(1);
                    } else if name.as_slice() == b"redirect" {
                        if let Some(current_page) = page.as_mut() {
                            current_page.redirect_target =
                                mediawiki_xml_attribute(&start, b"title", reader.decoder())?;
                        }
                    } else if name.as_slice() == b"namespace" && page.is_none() {
                        let key = mediawiki_xml_attribute(&start, b"key", reader.decoder())?
                            .unwrap_or_default();
                        pending_namespace = Some((key, String::new()));
                    }
                    stack.push(name);
                }
                XmlEvent::Empty(start) => {
                    let name = xml_local_name(start.name().as_ref()).to_vec();
                    if name.as_slice() == b"redirect" {
                        if let Some(current_page) = page.as_mut() {
                            current_page.redirect_target =
                                mediawiki_xml_attribute(&start, b"title", reader.decoder())?;
                        }
                    }
                }
                XmlEvent::Text(text) => {
                    let decoded = text.decode().map_err(|error| {
                        CoreError::Validation(format!("invalid MediaWiki XML text: {error}"))
                    })?;
                    let value = quick_xml::escape::unescape(&decoded).map_err(|error| {
                        CoreError::Validation(format!(
                            "MediaWiki XML contains an unsupported entity reference: {error}"
                        ))
                    })?;
                    append_mediawiki_xml_text(
                        &stack,
                        &value,
                        &mut page,
                        &mut pending_namespace,
                        &mut self.site_metadata,
                    );
                    self.validate_current_revision_size(&page)?;
                }
                XmlEvent::CData(text) => {
                    let value = text.decode().map_err(|error| {
                        CoreError::Validation(format!("invalid MediaWiki XML CDATA: {error}"))
                    })?;
                    append_mediawiki_xml_text(
                        &stack,
                        &value,
                        &mut page,
                        &mut pending_namespace,
                        &mut self.site_metadata,
                    );
                    self.validate_current_revision_size(&page)?;
                }
                XmlEvent::End(end) => {
                    let name = xml_local_name(end.name().as_ref()).to_vec();
                    if stack.last().map(Vec::as_slice) != Some(name.as_slice()) {
                        return Err(CoreError::Validation(
                            "MediaWiki XML element nesting is invalid".into(),
                        ));
                    }
                    if name.as_slice() == b"revision" {
                        let current_page = page.as_mut().expect("revision requires page");
                        let revision = current_page
                            .current_revision
                            .take()
                            .expect("revision state is present");
                        if current_page
                            .revision
                            .as_ref()
                            .is_none_or(|current| mediawiki_revision_is_newer(&revision, current))
                        {
                            current_page.revision = Some(revision);
                        }
                    } else if name.as_slice() == b"namespace" && page.is_none() {
                        if let Some((key, value)) = pending_namespace.take() {
                            self.namespaces.insert(key, value.trim().to_owned());
                        }
                    } else if name.as_slice() == b"page" {
                        let current_page = page.take().expect("page state is present");
                        self.finish_page(current_page, reader.buffer_position())?;
                    }
                    stack.pop();
                }
                XmlEvent::DocType(_) => {
                    return Err(CoreError::Validation(
                        "MediaWiki XML DTD and entity declarations are not allowed".into(),
                    ));
                }
                XmlEvent::Decl(declaration) => {
                    if declaration
                        .encoding()
                        .transpose()
                        .map_err(|error| {
                            CoreError::Validation(format!(
                                "invalid MediaWiki XML encoding declaration: {error}"
                            ))
                        })?
                        .is_some_and(|encoding| !encoding.eq_ignore_ascii_case(b"utf-8"))
                    {
                        return Err(CoreError::Validation(
                            "MediaWiki XML must use UTF-8 encoding".into(),
                        ));
                    }
                }
                XmlEvent::GeneralRef(reference) => {
                    let value = if let Some(character) =
                        reference.resolve_char_ref().map_err(|error| {
                            CoreError::Validation(format!(
                                "invalid MediaWiki XML character reference: {error}"
                            ))
                        })? {
                        character.to_string()
                    } else {
                        match reference
                            .decode()
                            .map_err(|error| {
                                CoreError::Validation(format!(
                                    "invalid MediaWiki XML entity reference: {error}"
                                ))
                            })?
                            .as_ref()
                        {
                            "amp" => "&".into(),
                            "lt" => "<".into(),
                            "gt" => ">".into(),
                            "apos" => "'".into(),
                            "quot" => "\"".into(),
                            entity => {
                                return Err(CoreError::Validation(format!(
                                    "MediaWiki XML entity reference '&{entity};' is not allowed"
                                )))
                            }
                        }
                    };
                    append_mediawiki_xml_text(
                        &stack,
                        &value,
                        &mut page,
                        &mut pending_namespace,
                        &mut self.site_metadata,
                    );
                    self.validate_current_revision_size(&page)?;
                }
                XmlEvent::Eof => break,
                XmlEvent::Comment(_) | XmlEvent::PI(_) => {}
            }
            buffer.clear();
        }
        if !root_seen || !stack.is_empty() || page.is_some() {
            return Err(CoreError::Validation(
                "MediaWiki XML ended before all elements were closed".into(),
            ));
        }
        self.report_progress(reader.buffer_position(), None)
    }
}

impl MediaWikiAnalyzer<'_> {
    pub(super) fn finish_page(
        &mut self,
        mut page: MediaWikiPage,
        source_bytes: u64,
    ) -> Result<(), CoreError> {
        self.processed_pages = self.processed_pages.saturating_add(1);
        if self.processed_pages > self.limits.max_files
            || self.processed_pages > self.limits.max_entries
        {
            return Err(CoreError::Validation(format!(
                "MediaWiki XML exceeds the maximum page count of {}",
                self.limits.max_files.min(self.limits.max_entries)
            )));
        }
        page.title = page.title.trim().to_owned();
        if page.title.is_empty() {
            return Err(CoreError::Validation(
                "MediaWiki page title cannot be empty".into(),
            ));
        }
        let revision = page.revision.take().unwrap_or_default();
        let wikitext_bytes = revision.text.len() as u64;
        if wikitext_bytes > self.limits.max_file_bytes {
            return Err(CoreError::Validation(format!(
                "MediaWiki page '{}' exceeds the maximum page size of {} bytes",
                page.title, self.limits.max_file_bytes
            )));
        }
        self.total_wikitext_bytes = self
            .total_wikitext_bytes
            .checked_add(wikitext_bytes)
            .ok_or_else(|| CoreError::Validation("MediaWiki content size overflowed".into()))?;
        if self.total_wikitext_bytes > self.limits.max_total_bytes {
            return Err(CoreError::Validation(format!(
                "MediaWiki pages exceed the maximum staged content size of {} bytes",
                self.limits.max_total_bytes
            )));
        }
        self.omitted_revisions = self
            .omitted_revisions
            .saturating_add(page.revision_count.saturating_sub(1));
        let markup = analyze_mediawiki_markup(&revision.text, page.redirect_target.as_deref());
        let namespace_id = page.namespace_id.trim();
        let namespace_id = if namespace_id.is_empty() {
            0
        } else {
            namespace_id.parse::<i64>().map_err(|_| {
                CoreError::Validation(format!(
                    "MediaWiki page '{}' has an invalid namespace id",
                    page.title
                ))
            })?
        };
        let namespace_id = namespace_id.to_string();
        let namespace_name = self
            .namespaces
            .get(&namespace_id)
            .cloned()
            .unwrap_or_default();
        let parent_source_path = format!("namespaces/{namespace_id}");
        self.folders.insert(parent_source_path.clone());
        let native_identity = if page.id.trim().is_empty() {
            format!("title:{}", normalize_mediawiki_title(&page.title))
        } else {
            format!("page:{}", page.id.trim())
        };
        let object_id = hex_digest(
            format!(
                "{}\0{}\0{}",
                self.import.importer.id, self.import.source.id, native_identity
            )
            .as_bytes(),
        );
        let source_path = format!(
            "{parent_source_path}/pages/{}.wiki",
            &hex_digest(native_identity.as_bytes())[..24]
        );
        let mut fields = BTreeMap::from([
            (
                "namespace_id".into(),
                serde_json::Value::String(namespace_id.clone()),
            ),
            (
                "page_id".into(),
                serde_json::Value::String(page.id.trim().into()),
            ),
        ]);
        if !namespace_name.is_empty() {
            fields.insert(
                "namespace".into(),
                serde_json::Value::String(namespace_name.clone()),
            );
        }
        for (key, value) in [
            ("revision_id", revision.id.trim()),
            ("revision_timestamp", revision.timestamp.trim()),
            ("content_model", revision.model.trim()),
            ("source_format", revision.format.trim()),
            (
                "redirect_target",
                markup.redirect_target.as_deref().unwrap_or(""),
            ),
        ] {
            if !value.is_empty() {
                fields.insert(key.into(), serde_json::Value::String(value.into()));
            }
        }
        if !markup.categories.is_empty() {
            fields.insert(
                "categories".into(),
                serde_json::Value::Array(
                    markup
                        .categories
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        if !markup.template_names.is_empty() {
            fields.insert(
                "templates".into(),
                serde_json::Value::Array(
                    markup
                        .template_names
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        for (key, value) in &markup.infobox_fields {
            fields.insert(format!("infobox.{key}"), value.clone());
        }
        let mut mapping_hints = vec![StagedMappingHint {
            kind: MappingHintKind::Hierarchy,
            source_key: Some("namespace".into()),
            suggested_value: serde_json::Value::String(namespace_id.clone()),
            confidence: Some(1.0),
            reason: Some("MediaWiki namespace".into()),
        }];
        for category in &markup.categories {
            mapping_hints.push(StagedMappingHint {
                kind: MappingHintKind::SourceCategory,
                source_key: Some("categories".into()),
                suggested_value: serde_json::Value::String(category.clone()),
                confidence: Some(1.0),
                reason: Some("MediaWiki category".into()),
            });
            mapping_hints.push(StagedMappingHint {
                kind: MappingHintKind::Hierarchy,
                source_key: Some("categories".into()),
                suggested_value: serde_json::Value::String(category.clone()),
                confidence: Some(0.7),
                reason: Some("MediaWiki category hierarchy candidate".into()),
            });
        }
        for key in markup.infobox_fields.keys() {
            mapping_hints.push(StagedMappingHint {
                kind: MappingHintKind::Field,
                source_key: Some(format!("infobox.{key}")),
                suggested_value: serde_json::Value::String(key.clone()),
                confidence: Some(0.65),
                reason: Some("MediaWiki infobox parameter".into()),
            });
        }
        let mut metadata = BTreeMap::from([
            (
                "source_format".into(),
                serde_json::Value::String("mediawiki".into()),
            ),
            (
                "document_format".into(),
                serde_json::Value::String("wikitext".into()),
            ),
            (
                "namespace_id".into(),
                serde_json::Value::String(namespace_id),
            ),
            (
                "revision_count".into(),
                serde_json::Value::from(page.revision_count),
            ),
        ]);
        if !namespace_name.is_empty() {
            metadata.insert(
                "namespace".into(),
                serde_json::Value::String(namespace_name),
            );
        }
        for (key, value) in &self.site_metadata {
            metadata.insert(
                format!("wiki_{key}"),
                serde_json::Value::String(value.clone()),
            );
        }
        if let Some(target) = &markup.redirect_target {
            metadata.insert(
                "mediawiki_redirect".into(),
                serde_json::Value::String(target.clone()),
            );
        }
        let mut latest_revision = serde_json::Map::new();
        for (key, value) in [
            ("id", revision.id),
            ("parent_id", revision.parent_id),
            ("timestamp", revision.timestamp),
            ("model", revision.model),
            ("format", revision.format),
            ("sha1", revision.sha1),
            ("contributor", revision.contributor),
        ] {
            if !value.trim().is_empty() {
                latest_revision.insert(key.into(), serde_json::Value::String(value));
            }
        }
        let mut object_diagnostics = Vec::new();
        for warning in markup.warnings {
            self.reserve_diagnostic()?;
            object_diagnostics.push(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: "mediawiki_wikitext_partial".into(),
                message: warning,
                source_path: Some(source_path.clone()),
                object_id: Some(object_id.clone()),
            });
        }
        self.import.objects.push(StagedObject {
            id: object_id.clone(),
            source_id: object_id,
            source_kind: "mediawiki_page".into(),
            source_path: source_path.clone(),
            content_hash: hex_digest(revision.text.as_bytes()),
            title: page.title,
            body: Some(StagedDocument {
                format: "markdown".into(),
                body: revision.text.clone(),
            }),
            parent_source_path: Some(parent_source_path),
            tags: markup.categories,
            aliases: Vec::new(),
            fields,
            metadata,
            raw_source_data: BTreeMap::from([
                ("wikitext".into(), serde_json::Value::String(revision.text)),
                (
                    "latest_revision".into(),
                    serde_json::Value::Object(latest_revision),
                ),
                (
                    "templates".into(),
                    serde_json::Value::Array(markup.templates),
                ),
            ]),
            links: markup.links,
            mapping_hints,
            diagnostics: object_diagnostics,
        });
        self.report_progress(source_bytes, Some(source_path))
    }

    pub(super) fn resolve_links_and_redirects(&mut self) -> Result<(), CoreError> {
        let mut objects_by_title = BTreeMap::<String, BTreeSet<String>>::new();
        for object in &self.import.objects {
            objects_by_title
                .entry(normalize_mediawiki_title(&object.title))
                .or_default()
                .insert(object.id.clone());
        }
        struct PendingDiagnostic {
            code: &'static str,
            message: String,
            source_path: String,
            object_id: String,
        }
        let mut diagnostics = Vec::new();
        let mut redirect_aliases = Vec::<(String, String)>::new();
        for object in &mut self.import.objects {
            let redirect_target = object
                .metadata
                .get("mediawiki_redirect")
                .and_then(serde_json::Value::as_str)
                .map(normalize_mediawiki_title);
            for link in &mut object.links {
                if link.resolution == StagedLinkResolution::NotApplicable {
                    continue;
                }
                let target_key =
                    normalize_mediawiki_title(mediawiki_link_page_target(&link.target));
                if target_key.is_empty() {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(object.id.clone());
                    continue;
                }
                let candidates = objects_by_title
                    .get(&target_key)
                    .cloned()
                    .unwrap_or_default();
                if candidates.len() == 1 {
                    let target_id = candidates.into_iter().next().expect("one candidate");
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(target_id.clone());
                    if redirect_target.as_deref() == Some(target_key.as_str()) {
                        redirect_aliases.push((target_id.clone(), object.title.clone()));
                        object.mapping_hints.push(StagedMappingHint {
                            kind: MappingHintKind::Relationship,
                            source_key: Some("redirect_target".into()),
                            suggested_value: serde_json::Value::String(target_id),
                            confidence: Some(1.0),
                            reason: Some("unique MediaWiki redirect target".into()),
                        });
                    }
                } else if candidates.len() > 1 {
                    link.resolution = StagedLinkResolution::Ambiguous;
                    link.candidate_object_ids = candidates.into_iter().collect();
                    diagnostics.push(PendingDiagnostic {
                        code: "mediawiki_target_ambiguous",
                        message: format!(
                            "MediaWiki target '{}' matches multiple staged pages.",
                            link.target
                        ),
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                } else {
                    link.resolution = StagedLinkResolution::Missing;
                    diagnostics.push(PendingDiagnostic {
                        code: "mediawiki_target_missing",
                        message: format!(
                            "MediaWiki target '{}' was not found in the selected dump.",
                            link.target
                        ),
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                }
            }
        }
        let object_indexes = self
            .import
            .objects
            .iter()
            .enumerate()
            .map(|(index, object)| (object.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        for (target_id, alias) in redirect_aliases {
            if let Some(index) = object_indexes.get(&target_id) {
                let target = &mut self.import.objects[*index];
                if alias != target.title && !target.aliases.contains(&alias) {
                    target.aliases.push(alias);
                    target.aliases.sort();
                }
            }
        }
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

    pub(super) fn reserve_diagnostic(&mut self) -> Result<(), CoreError> {
        if self.total_diagnostics >= self.limits.max_diagnostics {
            return Err(CoreError::Validation(format!(
                "MediaWiki analysis exceeds the maximum diagnostic count of {}",
                self.limits.max_diagnostics
            )));
        }
        self.total_diagnostics += 1;
        Ok(())
    }

    pub(super) fn validate_current_revision_size(
        &self,
        page: &Option<MediaWikiPage>,
    ) -> Result<(), CoreError> {
        if page
            .as_ref()
            .and_then(|page| page.current_revision.as_ref())
            .is_some_and(|revision| revision.text.len() as u64 > self.limits.max_file_bytes)
        {
            return Err(CoreError::Validation(format!(
                "MediaWiki revision exceeds the maximum page size of {} bytes",
                self.limits.max_file_bytes
            )));
        }
        Ok(())
    }

    pub(super) fn record_diagnostic(
        &mut self,
        diagnostic: ImportDiagnostic,
    ) -> Result<(), CoreError> {
        self.reserve_diagnostic()?;
        self.import.diagnostics.push(diagnostic);
        Ok(())
    }

    pub(super) fn report_progress(
        &mut self,
        source_bytes: u64,
        source_path: Option<String>,
    ) -> Result<(), CoreError> {
        (self.progress)(ImportAnalysisProgress {
            processed_entries: self.processed_pages,
            staged_object_count: self.import.objects.len(),
            unsupported_count: self.import.unsupported.len(),
            source_bytes,
            source_path,
        })
    }
}

pub(super) fn xml_local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

pub(super) fn mediawiki_xml_attribute(
    start: &BytesStart<'_>,
    key: &[u8],
    decoder: XmlDecoder,
) -> Result<Option<String>, CoreError> {
    for attribute in start.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| {
            CoreError::Validation(format!("invalid MediaWiki XML attribute: {error}"))
        })?;
        if xml_local_name(attribute.key.as_ref()) == key {
            return attribute
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
                .map(|value| Some(value.into_owned()))
                .map_err(|error| {
                    CoreError::Validation(format!("invalid MediaWiki XML attribute value: {error}"))
                });
        }
    }
    Ok(None)
}

pub(super) fn append_mediawiki_xml_text(
    stack: &[Vec<u8>],
    value: &str,
    page: &mut Option<MediaWikiPage>,
    pending_namespace: &mut Option<(String, String)>,
    site_metadata: &mut BTreeMap<String, String>,
) {
    let element = stack.last().map(Vec::as_slice).unwrap_or_default();
    let parent = stack
        .get(stack.len().saturating_sub(2))
        .map(Vec::as_slice)
        .unwrap_or_default();
    if let Some(page) = page.as_mut() {
        if let Some(revision) = page.current_revision.as_mut() {
            match (parent, element) {
                (b"revision", b"id") => revision.id.push_str(value),
                (b"revision", b"parentid") => revision.parent_id.push_str(value),
                (b"revision", b"timestamp") => revision.timestamp.push_str(value),
                (b"revision", b"model") => revision.model.push_str(value),
                (b"revision", b"format") => revision.format.push_str(value),
                (b"revision", b"sha1") => revision.sha1.push_str(value),
                (_, b"username" | b"ip") => revision.contributor.push_str(value),
                (_, b"text") => revision.text.push_str(value),
                _ => {}
            }
        } else {
            match (parent, element) {
                (b"page", b"title") => page.title.push_str(value),
                (b"page", b"ns") => page.namespace_id.push_str(value),
                (b"page", b"id") => page.id.push_str(value),
                _ => {}
            }
        }
    } else if element == b"namespace" {
        if let Some((_, namespace)) = pending_namespace.as_mut() {
            namespace.push_str(value);
        }
    } else if parent == b"siteinfo"
        && matches!(
            element,
            b"sitename" | b"dbname" | b"base" | b"generator" | b"case"
        )
    {
        site_metadata
            .entry(String::from_utf8_lossy(element).into_owned())
            .or_default()
            .push_str(value);
    }
}

pub(super) fn mediawiki_revision_is_newer(
    candidate: &MediaWikiRevision,
    current: &MediaWikiRevision,
) -> bool {
    let candidate_timestamp = candidate.timestamp.trim();
    let current_timestamp = current.timestamp.trim();
    if candidate_timestamp != current_timestamp {
        return candidate_timestamp > current_timestamp;
    }
    let candidate_id = candidate.id.trim().parse::<u64>().unwrap_or_default();
    let current_id = current.id.trim().parse::<u64>().unwrap_or_default();
    candidate_id >= current_id
}

#[derive(Debug, Default)]
pub(super) struct MediaWikiMarkup {
    categories: Vec<String>,
    links: Vec<StagedLink>,
    redirect_target: Option<String>,
    template_names: Vec<String>,
    templates: Vec<serde_json::Value>,
    infobox_fields: BTreeMap<String, serde_json::Value>,
    warnings: Vec<String>,
}

pub(super) fn analyze_mediawiki_markup(
    wikitext: &str,
    xml_redirect: Option<&str>,
) -> MediaWikiMarkup {
    let mut markup = MediaWikiMarkup::default();
    let mut categories = BTreeSet::new();
    let mut index = 0;
    while let Some(relative_start) = wikitext[index..].find("[[") {
        let start = index + relative_start;
        let Some(relative_end) = wikitext[start + 2..].find("]]") else {
            markup
                .warnings
                .push("Preserved an unclosed MediaWiki internal link.".into());
            break;
        };
        let end = start + 2 + relative_end;
        let raw = &wikitext[start..end + 2];
        let content = wikitext[start + 2..end].trim();
        let (target, label) = content
            .split_once('|')
            .map(|(target, label)| (target.trim(), Some(label.trim())))
            .unwrap_or((content, None));
        if !target.is_empty() {
            let semantic_target = target.trim_start_matches(':');
            let (prefix, suffix) = semantic_target
                .split_once(':')
                .map(|(prefix, suffix)| (prefix.trim(), suffix.trim()))
                .unwrap_or(("", semantic_target));
            let is_category = !target.starts_with(':') && prefix.eq_ignore_ascii_case("category");
            let is_file = matches_ignore_ascii_case(prefix, &["file", "image"]);
            if is_category {
                let category = mediawiki_link_page_target(suffix).trim();
                if !category.is_empty() {
                    categories.insert(category.to_owned());
                }
            }
            markup.links.push(StagedLink {
                kind: if is_file {
                    StagedLinkKind::Embed
                } else {
                    StagedLinkKind::Internal
                },
                target: semantic_target.into(),
                label: label.filter(|label| !label.is_empty()).map(str::to_owned),
                resolution: if is_category || is_file {
                    StagedLinkResolution::NotApplicable
                } else {
                    StagedLinkResolution::Unresolved
                },
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: Some(raw.into()),
            });
        }
        index = end + 2;
    }
    markup.categories = categories.into_iter().collect();

    let (templates, template_warnings) = discover_mediawiki_templates(wikitext);
    markup.warnings.extend(template_warnings);
    let mut template_names = BTreeSet::new();
    for template in templates {
        template_names.insert(template.name.clone());
        if template.name.to_ascii_lowercase().starts_with("infobox") {
            for (key, value) in &template.parameters {
                let key = normalize_mediawiki_field_key(key);
                if key.is_empty() {
                    continue;
                }
                insert_mediawiki_field_value(
                    &mut markup.infobox_fields,
                    key,
                    serde_json::Value::String(value.clone()),
                );
            }
        }
        markup.templates.push(serde_json::json!({
            "name": template.name,
            "parameters": template.parameters,
            "raw": template.raw,
        }));
    }
    markup.template_names = template_names.into_iter().collect();
    markup.redirect_target = xml_redirect
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            let trimmed = wikitext.trim_start();
            let prefix = trimmed.get(..9)?;
            prefix.eq_ignore_ascii_case("#redirect").then(|| {
                markup
                    .links
                    .iter()
                    .find(|link| link.kind == StagedLinkKind::Internal)
                    .map(|link| link.target.clone())
            })?
        });
    if let Some(target) = &markup.redirect_target {
        let normalized = normalize_mediawiki_title(mediawiki_link_page_target(target));
        if !markup.links.iter().any(|link| {
            link.kind == StagedLinkKind::Internal
                && normalize_mediawiki_title(mediawiki_link_page_target(&link.target)) == normalized
        }) {
            markup.links.push(StagedLink {
                kind: StagedLinkKind::Internal,
                target: target.clone(),
                label: None,
                resolution: StagedLinkResolution::Unresolved,
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: None,
            });
        }
    }
    markup.warnings.sort();
    markup.warnings.dedup();
    markup
}

#[derive(Debug)]
pub(super) struct MediaWikiTemplate {
    name: String,
    parameters: BTreeMap<String, String>,
    raw: String,
}

pub(super) fn discover_mediawiki_templates(
    wikitext: &str,
) -> (Vec<MediaWikiTemplate>, Vec<String>) {
    let bytes = wikitext.as_bytes();
    let mut templates = Vec::new();
    let mut warnings = Vec::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] != b'{' || bytes[index + 1] != b'{' || bytes.get(index + 2) == Some(&b'{') {
            index += 1;
            continue;
        }
        let start = index;
        let mut depth = 1_usize;
        index += 2;
        while index + 1 < bytes.len() && depth > 0 {
            if bytes[index] == b'{' && bytes[index + 1] == b'{' {
                depth = depth.saturating_add(1);
                if depth > MAX_MEDIAWIKI_TEMPLATE_DEPTH {
                    warnings.push(format!(
                        "Template nesting exceeded the maximum depth of {MAX_MEDIAWIKI_TEMPLATE_DEPTH}."
                    ));
                    return (templates, warnings);
                }
                index += 2;
            } else if bytes[index] == b'}' && bytes[index + 1] == b'}' {
                depth -= 1;
                index += 2;
            } else {
                index += 1;
            }
        }
        if depth != 0 {
            warnings.push("Preserved an unclosed MediaWiki template invocation.".into());
            break;
        }
        if templates.len() >= MAX_MEDIAWIKI_TEMPLATES_PER_PAGE {
            warnings.push(format!(
                "Only the first {MAX_MEDIAWIKI_TEMPLATES_PER_PAGE} template invocations were analyzed."
            ));
            break;
        }
        let raw = &wikitext[start..index];
        let inner = &raw[2..raw.len() - 2];
        let parts = split_mediawiki_template_parts(inner);
        let Some(name) = parts
            .first()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
        else {
            continue;
        };
        if name.starts_with('{') {
            continue;
        }
        let mut parameters = BTreeMap::new();
        let mut positional = 0_usize;
        for part in parts.into_iter().skip(1) {
            let (key, value) = if let Some((key, value)) = split_mediawiki_parameter(&part) {
                (key.trim().to_owned(), value.trim().to_owned())
            } else {
                positional += 1;
                (positional.to_string(), part.trim().to_owned())
            };
            if !key.is_empty() {
                parameters.insert(key, value);
            }
        }
        templates.push(MediaWikiTemplate {
            name: name.replace('_', " ").trim().to_owned(),
            parameters,
            raw: raw.to_owned(),
        });
    }
    (templates, warnings)
}

pub(super) fn split_mediawiki_template_parts(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut template_depth = 0_usize;
    let mut link_depth = 0_usize;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"{{") {
            template_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"}}") {
            template_depth = template_depth.saturating_sub(1);
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"[[") {
            link_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"]]") {
            link_depth = link_depth.saturating_sub(1);
            index += 2;
        } else if bytes[index] == b'|' && template_depth == 0 && link_depth == 0 {
            parts.push(value[start..index].to_owned());
            start = index + 1;
            index += 1;
        } else {
            index += 1;
        }
    }
    parts.push(value[start..].to_owned());
    parts
}

pub(super) fn split_mediawiki_parameter(value: &str) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let mut template_depth = 0_usize;
    let mut link_depth = 0_usize;
    let mut index = 0;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"{{") {
            template_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"}}") {
            template_depth = template_depth.saturating_sub(1);
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"[[") {
            link_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"]]") {
            link_depth = link_depth.saturating_sub(1);
            index += 2;
        } else if bytes[index] == b'=' && template_depth == 0 && link_depth == 0 {
            return Some((&value[..index], &value[index + 1..]));
        } else {
            index += 1;
        }
    }
    None
}

pub(super) fn insert_mediawiki_field_value(
    fields: &mut BTreeMap<String, serde_json::Value>,
    key: String,
    value: serde_json::Value,
) {
    match fields.remove(&key) {
        None => {
            fields.insert(key, value);
        }
        Some(serde_json::Value::Array(mut values)) => {
            values.push(value);
            fields.insert(key, serde_json::Value::Array(values));
        }
        Some(previous) => {
            fields.insert(key, serde_json::Value::Array(vec![previous, value]));
        }
    }
}

pub(super) fn normalize_mediawiki_field_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut pending_separator = false;
    for character in value.trim().chars().take(128) {
        if character.is_alphanumeric() {
            if pending_separator && !normalized.is_empty() {
                normalized.push('_');
            }
            normalized.extend(character.to_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    normalized
}

pub(super) fn normalize_mediawiki_title(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(':')
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(super) fn mediawiki_link_page_target(value: &str) -> &str {
    value
        .split_once('#')
        .map(|(page, _)| page)
        .unwrap_or(value)
        .trim()
}

pub(super) fn matches_ignore_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}
