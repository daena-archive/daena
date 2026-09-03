use super::*;
use crate::ProjectStore;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;
use zip::write::SimpleFileOptions;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("daena-external-import-test-{}", Uuid::new_v4()));
        fs::create_dir(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(path).unwrap();
    let mut archive = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for (name, bytes) in entries {
        archive.start_file(*name, options).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap();
}

const DOCX_CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

const DOCX_DOCUMENT_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
 <w:body>
  <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Field Guide</w:t></w:r></w:p>
  <w:p><w:r><w:t xml:space="preserve">A </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r><w:r><w:t xml:space="preserve"> and </w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t>careful</w:t></w:r><w:r><w:t> note.</w:t></w:r></w:p>
  <w:p><w:hyperlink r:id="rLink"><w:r><w:t>Web</w:t></w:r></w:hyperlink></w:p>
  <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>First item</w:t></w:r></w:p>
  <w:p><w:r><w:drawing><wp:docPr id="1" name="Picture" descr="Map"/><a:blip r:embed="rImage"/></w:drawing></w:r></w:p>
  <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Value</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>North</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cold</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
 </w:body>
</w:document>"#;

const DOCX_RELATIONSHIPS_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
 <Relationship Id="rImage" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
</Relationships>"#;

const DOCX_NUMBERING_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
 <w:abstractNum w:abstractNumId="9"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum>
 <w:num w:numId="1"><w:abstractNumId w:val="9"/></w:num>
</w:numbering>"#;

const DOCX_CORE_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Imported Field Guide</dc:title></cp:coreProperties>"#;

fn write_docx_fixture(path: &Path) {
    write_zip(
        path,
        &[
            ("[Content_Types].xml", DOCX_CONTENT_TYPES),
            ("word/document.xml", DOCX_DOCUMENT_XML),
            ("word/_rels/document.xml.rels", DOCX_RELATIONSHIPS_XML),
            ("word/numbering.xml", DOCX_NUMBERING_XML),
            ("docProps/core.xml", DOCX_CORE_XML),
            ("word/media/image1.png", b"\x89PNG\r\n\x1a\nfixture"),
        ],
    );
}

#[test]
fn folder_analysis_is_deterministic_and_does_not_mutate_a_project() {
    let source = TestDirectory::new();
    fs::create_dir(source.path().join("Characters")).unwrap();
    fs::write(
        source.path().join("Characters/Alice.md"),
        "# Alice\n\nA cartographer.",
    )
    .unwrap();
    fs::write(source.path().join("Notes.txt"), "Remember the north road.").unwrap();
    fs::write(source.path().join("portrait.png"), [0_u8, 1, 2]).unwrap();
    let project = ProjectStore::in_memory().unwrap();

    let first =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let second =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    assert_eq!(first, second);
    assert!(project.list_entities().unwrap().is_empty());
    assert_eq!(first.summary.document_count, 2);
    assert_eq!(first.summary.candidate_entity_count, 2);
    assert_eq!(first.summary.folder_count, 1);
    assert_eq!(first.summary.unsupported_count, 1);
    assert_eq!(first.summary.warning_count, 1);
    assert_eq!(
        first
            .objects
            .iter()
            .map(|object| object.source_path.as_str())
            .collect::<Vec<_>>(),
        vec!["Characters/Alice.md", "Notes.txt"]
    );
    assert_eq!(
        first.objects[0].parent_source_path.as_deref(),
        Some("Characters")
    );
    assert_eq!(first.objects[0].title, "Alice");
    assert_eq!(
        first.objects[0].body.as_ref().unwrap().body,
        "# Alice\n\nA cartographer."
    );
}

#[test]
fn invalid_utf8_is_preserved_as_an_explicit_unsupported_result() {
    let source = TestDirectory::new();
    fs::write(source.path().join("broken.md"), [0xff, 0xfe]).unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    assert!(staged.objects.is_empty());
    assert_eq!(staged.summary.unsupported_count, 1);
    assert_eq!(staged.summary.error_count, 1);
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "invalid_utf8"));
}

#[test]
fn source_identity_is_stable_when_document_content_changes() {
    let source = TestDirectory::new();
    let path = source.path().join("Changing.md");
    fs::write(&path, "first version").unwrap();
    let first = analyze_generic_documents(&path, GenericDocumentImportLimits::default())
        .unwrap()
        .objects
        .remove(0);

    fs::write(&path, "second version").unwrap();
    let second = analyze_generic_documents(&path, GenericDocumentImportLimits::default())
        .unwrap()
        .objects
        .remove(0);

    assert_eq!(first.source_id, second.source_id);
    assert_eq!(first.id, second.id);
    assert_ne!(first.content_hash, second.content_hash);
}

#[test]
fn progress_is_incremental_and_can_cancel_analysis() {
    let source = TestDirectory::new();
    fs::write(source.path().join("one.md"), "one").unwrap();
    fs::write(source.path().join("two.md"), "two").unwrap();
    let mut updates = Vec::new();

    let error = analyze_generic_documents_with_progress(
        source.path(),
        GenericDocumentImportLimits::default(),
        |progress| {
            updates.push(progress.clone());
            if progress.processed_entries >= 1 {
                Err(CoreError::Conflict(
                    EXTERNAL_IMPORT_ANALYSIS_CANCELLED.into(),
                ))
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), EXTERNAL_IMPORT_ANALYSIS_CANCELLED);
    assert_eq!(updates[0].processed_entries, 0);
    assert_eq!(updates.last().unwrap().processed_entries, 1);
    assert!(updates.last().unwrap().staged_object_count <= 1);
}

#[test]
fn analysis_enforces_file_and_total_byte_limits() {
    let source = TestDirectory::new();
    fs::write(source.path().join("one.md"), "1234").unwrap();
    fs::write(source.path().join("two.txt"), "5678").unwrap();
    let limits = GenericDocumentImportLimits {
        max_total_bytes: 7,
        ..Default::default()
    };

    let error = analyze_generic_documents(source.path(), limits).unwrap_err();
    assert!(error
        .to_string()
        .contains("exceeds the maximum total size of 7 bytes"));

    let limits = GenericDocumentImportLimits {
        max_files: 1,
        ..Default::default()
    };
    let error = analyze_generic_documents(source.path(), limits).unwrap_err();
    assert!(error
        .to_string()
        .contains("exceeds the maximum file count of 1"));

    let limits = GenericDocumentImportLimits {
        max_file_bytes: 3,
        ..Default::default()
    };
    let error = analyze_generic_documents(source.path(), limits).unwrap_err();
    assert!(error
        .to_string()
        .contains("exceeds the maximum size of 3 bytes"));

    fs::create_dir(source.path().join("nested")).unwrap();
    fs::write(source.path().join("nested/deep.md"), "deep").unwrap();
    let limits = GenericDocumentImportLimits {
        max_depth: 0,
        ..Default::default()
    };
    let error = analyze_generic_documents(source.path(), limits).unwrap_err();
    assert!(error
        .to_string()
        .contains("exceeds the maximum folder depth of 0"));

    let limits = GenericDocumentImportLimits {
        max_entries: 1,
        ..Default::default()
    };
    let error = analyze_generic_documents(source.path(), limits).unwrap_err();
    assert!(error
        .to_string()
        .contains("exceeds the maximum entry count of 1"));
}

#[test]
fn staged_validation_rejects_duplicate_ids_and_traversal_paths() {
    let source = TestDirectory::new();
    fs::write(source.path().join("one.md"), "one").unwrap();
    fs::write(source.path().join("two.md"), "two").unwrap();
    let mut staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    staged.objects[1].id = staged.objects[0].id.clone();
    assert!(staged.validate().is_err());
    staged.objects[1].id = "unique".into();
    staged.objects[1].source_path = "../outside.md".into();
    assert!(staged.validate().is_err());
    staged.objects[1].source_path = "C:/outside.md".into();
    assert!(staged.validate().is_err());
    staged.objects[1].source_path = "nested//outside.md".into();
    assert!(staged.validate().is_err());

    staged.objects[1].source_path = "two.md".into();
    let repeated_candidate = staged.objects[0].id.clone();
    staged.objects[1].links.push(StagedLink {
        kind: StagedLinkKind::Internal,
        target: "ambiguous".into(),
        label: None,
        resolution: StagedLinkResolution::Ambiguous,
        resolved_object_id: None,
        candidate_object_ids: vec![repeated_candidate; 2],
        raw: None,
    });
    assert!(staged.validate().is_err());
}

#[test]
fn candidate_plan_resolves_global_folder_and_item_overrides_deterministically() {
    let source = TestDirectory::new();
    fs::create_dir_all(source.path().join("People/Heroes")).unwrap();
    fs::write(source.path().join("People/Heroes/Alice.md"), "Alice").unwrap();
    fs::write(source.path().join("People/Bob.md"), "Bob").unwrap();
    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let alice = staged
        .objects
        .iter()
        .find(|object| object.title == "Alice")
        .unwrap();
    let mut overrides = ImportMappingOverrides::default();
    overrides.global.entity_type = Some("note".into());
    overrides
        .global
        .field_mappings
        .insert("tag".into(), "core:tag".into());
    overrides.folders.insert(
        "People".into(),
        ImportMappingDecision {
            entity_type: Some("daena.lore:person".into()),
            field_mappings: BTreeMap::from([("tag".into(), "lore:tag".into())]),
            relationship_mappings: BTreeMap::new(),
        },
    );
    overrides.folders.insert(
        "People/Heroes".into(),
        ImportMappingDecision {
            entity_type: Some("hero".into()),
            ..ImportMappingDecision::default()
        },
    );
    overrides.items.insert(
        alice.id.clone(),
        ImportMappingDecision {
            entity_type: Some("protagonist".into()),
            ..ImportMappingDecision::default()
        },
    );

    let first = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: 7,
            current_content_generation: 7,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics.clone(),
        },
        &overrides,
    )
    .unwrap();
    let second = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer,
            source: staged.source,
            captured_content_generation: 7,
            current_content_generation: 7,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics,
        },
        &overrides,
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.unresolved_decision_count, 0);
    assert_eq!(
        first
            .objects
            .iter()
            .find(|object| object.title == "Alice")
            .unwrap()
            .mapping
            .entity_type
            .as_deref(),
        Some("protagonist")
    );
    let bob = first
        .objects
        .iter()
        .find(|object| object.title == "Bob")
        .unwrap();
    assert_eq!(
        bob.mapping.entity_type.as_deref(),
        Some("daena.lore:person")
    );
    assert_eq!(bob.mapping.field_mappings["tag"], "lore:tag");
}

#[test]
fn candidate_plan_applies_source_category_mappings_before_folder_and_item_overrides() {
    let source = TestDirectory::new();
    fs::create_dir_all(source.path().join("People")).unwrap();
    fs::write(source.path().join("People/Alice.md"), "Alice").unwrap();
    let mut staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    staged.objects[0].tags = vec!["Heroes".into()];
    let object_id = staged.objects[0].id.clone();
    let mut overrides = ImportMappingOverrides::default();
    overrides.global.entity_type = Some("note".into());
    overrides.categories.insert(
        "Heroes".into(),
        ImportMappingDecision {
            entity_type: Some("daena.lore:person".into()),
            ..ImportMappingDecision::default()
        },
    );

    let category_plan = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: 1,
            current_content_generation: 1,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: 0,
            diagnostics: Vec::new(),
        },
        &overrides,
    )
    .unwrap();
    assert_eq!(
        category_plan.objects[0].mapping.entity_type.as_deref(),
        Some("daena.lore:person")
    );

    overrides.folders.insert(
        "People".into(),
        ImportMappingDecision {
            entity_type: Some("character".into()),
            ..ImportMappingDecision::default()
        },
    );
    overrides.items.insert(
        object_id,
        ImportMappingDecision {
            entity_type: Some("protagonist".into()),
            ..ImportMappingDecision::default()
        },
    );
    let specific_plan = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer,
            source: staged.source,
            captured_content_generation: 1,
            current_content_generation: 1,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects,
            unsupported_count: 0,
            diagnostics: Vec::new(),
        },
        &overrides,
    )
    .unwrap();
    assert_eq!(
        specific_plan.objects[0].mapping.entity_type.as_deref(),
        Some("protagonist")
    );
}

#[test]
fn candidate_plan_surfaces_unresolved_types_and_stale_generation() {
    let source = TestDirectory::new();
    fs::write(source.path().join("note.md"), "note").unwrap();
    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    let plan = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer,
            source: staged.source,
            captured_content_generation: 1,
            current_content_generation: 2,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects,
            unsupported_count: 0,
            diagnostics: Vec::new(),
        },
        &ImportMappingOverrides::default(),
    )
    .unwrap();

    assert_eq!(plan.unresolved_decision_count, 1);
    assert_eq!(plan.objects[0].issues[0].code, "entity_type_required");
    assert_eq!(plan.issues[0].code, "project_generation_changed");
}

#[test]
fn markdown_analysis_preserves_frontmatter_and_resolves_safe_links_and_assets() {
    let source = TestDirectory::new();
    fs::create_dir_all(source.path().join("Notes")).unwrap();
    fs::create_dir_all(source.path().join("assets")).unwrap();
    fs::write(
            source.path().join("Notes/Note.md"),
            "---\ncategory: place\n---\n# Note\n\n[Other][other]\n![Map](../assets/map.png)\n![Missing](../../outside.png)\n[Web](https://example.com)\n\n[other]: Other%20Note.md\n",
        )
        .unwrap();
    fs::write(source.path().join("Notes/Other Note.md"), "# Other").unwrap();
    fs::write(
        source.path().join("assets/map.png"),
        b"\x89PNG\r\n\x1a\nfixture",
    )
    .unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let note = staged
        .objects
        .iter()
        .find(|object| object.source_path == "Notes/Note.md")
        .unwrap();
    assert_eq!(note.fields["frontmatter"], "category: place\n");
    assert_eq!(note.raw_source_data["frontmatter"], "category: place\n");
    assert!(note
        .body
        .as_ref()
        .unwrap()
        .body
        .starts_with("---\ncategory: place\n---\n"));
    assert_eq!(note.links.len(), 4);
    assert!(note.links.iter().any(|link| {
        link.target == "Other%20Note.md" && link.resolution == StagedLinkResolution::Resolved
    }));
    assert!(note.links.iter().any(|link| {
        link.target == "../assets/map.png" && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert!(note.links.iter().any(|link| {
        link.target == "../../outside.png" && link.resolution == StagedLinkResolution::Missing
    }));
    assert_eq!(staged.assets.len(), 1);
    assert_eq!(
        staged.assets[0].owner_object_id.as_deref(),
        Some(note.id.as_str())
    );
    assert_eq!(staged.summary.asset_count, 1);
    assert_eq!(staged.summary.unresolved_link_count, 1);
}

#[test]
fn shared_markdown_attachment_is_staged_once_for_each_owner() {
    let source = TestDirectory::new();
    fs::create_dir_all(source.path().join("assets")).unwrap();
    fs::write(
        source.path().join("assets/map.png"),
        b"\x89PNG\r\n\x1a\nfixture",
    )
    .unwrap();
    fs::write(source.path().join("one.md"), "![Map](assets/map.png)").unwrap();
    fs::write(source.path().join("two.md"), "![Map](assets/map.png)").unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let owners = staged
        .assets
        .iter()
        .filter_map(|asset| asset.owner_object_id.as_deref())
        .collect::<BTreeSet<_>>();
    assert_eq!(staged.assets.len(), 2);
    assert_eq!(owners.len(), 2);
    assert_eq!(
        staged
            .assets
            .iter()
            .map(|asset| asset.source_path.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["assets/map.png"])
    );
    assert_eq!(
        staged
            .assets
            .iter()
            .map(|asset| asset.id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
}

#[test]
fn html_analysis_converts_structure_and_resolves_links_and_assets() {
    let source = TestDirectory::new();
    fs::create_dir_all(source.path().join("Notes")).unwrap();
    fs::create_dir_all(source.path().join("assets")).unwrap();
    let html = r#"<!doctype html>
<html><head><title>Field Guide</title></head><body>
<h1>Field Guide</h1>
<p>A <strong>bold</strong> and <em>careful</em> <code>note</code>.</p>
<ul><li>First</li><li>Second</li></ul>
<blockquote>Quoted passage</blockquote>
<table><tr><th>Name</th><th>Value</th></tr><tr><td>North</td><td>Cold</td></tr></table>
<p><a href="Other.html">Other note</a> <a href="https://example.com">Web</a></p>
<img src="../assets/map.png" alt="Map">
</body></html>"#;
    fs::write(source.path().join("Notes/Guide.html"), html).unwrap();
    fs::write(
        source.path().join("Notes/Other.html"),
        "<!doctype html><title>Other</title><p>Another note.</p>",
    )
    .unwrap();
    fs::write(
        source.path().join("assets/map.png"),
        b"\x89PNG\r\n\x1a\nfixture",
    )
    .unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let guide = staged
        .objects
        .iter()
        .find(|object| object.source_path == "Notes/Guide.html")
        .unwrap();
    let body = guide.body.as_ref().unwrap();

    assert_eq!(guide.source_kind, "html");
    assert_eq!(guide.title, "Field Guide");
    assert_eq!(body.format, "markdown");
    assert_eq!(guide.metadata["converted_from"], "html");
    assert_eq!(guide.raw_source_data["html"], html);
    assert!(body.body.contains("# Field Guide"));
    assert!(
        body.body.contains("A **bold** and *careful* `note`."),
        "{}",
        body.body
    );
    assert!(body.body.contains("- First"));
    assert!(body.body.contains("> Quoted passage"));
    assert!(body.body.contains("| Name | Value |"));
    assert!(body.body.contains("| --- | --- |"));
    assert!(guide.links.iter().any(|link| {
        link.target == "Other.html" && link.resolution == StagedLinkResolution::Resolved
    }));
    assert!(guide.links.iter().any(|link| {
        link.target == "https://example.com"
            && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert!(guide.links.iter().any(|link| {
        link.target == "../assets/map.png" && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert_eq!(staged.assets.len(), 1);
    assert_eq!(
        staged.assets[0].owner_object_id.as_deref(),
        Some(guide.id.as_str())
    );
}

#[test]
fn html_analysis_removes_active_content_and_unsafe_targets() {
    let source = TestDirectory::new();
    fs::write(
        source.path().join("unsafe.html"),
        r#"<!doctype html><html><body>
<p onclick="steal()">Keep this text.</p>
<script>script_payload()</script><style>style_payload{}</style>
<iframe src="https://example.com">iframe_payload</iframe>
<svg><text>svg_payload</text></svg>
<p>&lt;script&gt;encoded_payload()&lt;/script&gt;</p>
<a href="javascript:alert(1)">Unsafe link</a>
<img src="data:text/html,unsafe" alt="unsafe image">
<a href="/absolute/path">Absolute link</a>
</body></html>"#,
    )
    .unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let object = &staged.objects[0];
    let body = &object.body.as_ref().unwrap().body;

    assert!(body.contains("Keep this text."));
    assert!(body.contains("Unsafe link"));
    assert!(body.contains("Absolute link"));
    assert!(!body.contains("script_payload"));
    assert!(!body.contains("style_payload"));
    assert!(!body.contains("iframe_payload"));
    assert!(!body.contains("svg_payload"));
    assert!(!body.contains("javascript:"));
    assert!(!body.contains("data:text/html"));
    assert!(!body.contains("/absolute/path"));
    assert!(!Parser::new_ext(body, Options::all())
        .any(|event| matches!(event, Event::Html(_) | Event::InlineHtml(_))));
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "html_content_removed"));
    assert!(staged.summary.warning_count > 0);
}

#[test]
fn malformed_html_recovers_with_a_visible_warning() {
    let source = TestDirectory::new();
    fs::write(
        source.path().join("broken.htm"),
        "<!doctype html><title>Recovered</title><p>First <b>bold<p>Second</div>",
    )
    .unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    assert_eq!(staged.objects[0].title, "Recovered");
    assert!(staged.objects[0]
        .body
        .as_ref()
        .unwrap()
        .body
        .contains("Second"));
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "html_parser_recovered"));
}

#[test]
fn html_conversion_enforces_dom_depth_limit() {
    let mut html = String::from("<!doctype html><body>");
    html.push_str(&"<div>".repeat(MAX_HTML_DOM_DEPTH + 8));
    html.push_str("too deep");
    html.push_str(&"</div>".repeat(MAX_HTML_DOM_DEPTH + 8));

    let error = convert_html_to_markdown(&html).unwrap_err();
    assert!(error.to_string().contains("DOM complexity limit"));
}

#[test]
fn html_commit_preserves_converted_markdown_after_clean_rebuild() {
    let source = TestDirectory::new();
    let source_path = source.path().join("Guide.html");
    fs::write(
        &source_path,
        "<!doctype html><title>Guide</title><h1>Guide</h1><p>Converted <strong>body</strong>.</p>",
    )
    .unwrap();
    let staged =
        analyze_generic_documents(&source_path, GenericDocumentImportLimits::default()).unwrap();
    let expected_body = staged.objects[0].body.as_ref().unwrap().body.clone();
    let project = TestDirectory::new();
    let store = ProjectStore::open_directory(project.path()).unwrap();
    let generation = store.content_generation().unwrap();
    let mut mappings = ImportMappingOverrides::default();
    mappings.global.entity_type = Some("note".into());
    let candidate = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "html-session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: generation,
            current_content_generation: generation,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let validated = validate_import_candidate_plan(ImportValidationBuild {
        candidate,
        staged_objects: staged.objects,
        staged_assets: staged.assets,
        staged_unsupported: staged.unsupported,
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::new(),
            relationship_types: BTreeSet::new(),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::new(),
    })
    .unwrap()
    .plan
    .unwrap();
    let report = store
        .commit_external_import(
            &validated,
            Some(&source_path),
            true,
            "00000000-0000-4000-8000-000000000003",
        )
        .unwrap();

    store.flush_checkpoint("HTML import test").unwrap();
    drop(store);
    fs::remove_dir_all(project.path().join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
    let documents = rebuilt
        .list_documents(report.created[0].entity_id.clone())
        .unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].format, "markdown");
    assert_eq!(documents[0].body, expected_body);
}

#[test]
fn docx_analysis_preserves_structure_links_and_embedded_images() {
    let source = TestDirectory::new();
    let source_path = source.path().join("Guide.docx");
    write_docx_fixture(&source_path);

    let staged =
        analyze_generic_documents(&source_path, GenericDocumentImportLimits::default()).unwrap();
    let object = &staged.objects[0];
    let body = &object.body.as_ref().unwrap().body;

    assert_eq!(object.source_kind, "docx");
    assert_eq!(object.title, "Imported Field Guide");
    assert_eq!(object.body.as_ref().unwrap().format, "markdown");
    assert_eq!(object.metadata["converted_from"], "docx");
    assert!(object.raw_source_data["core_properties_xml"]
        .as_str()
        .unwrap()
        .contains("Imported Field Guide"));
    assert!(body.contains("# Field Guide"));
    assert!(body.contains("A **bold** and *careful* note."));
    assert!(body.contains("[Web](https://example.com)"));
    assert!(body.contains("1. First item"));
    assert!(body.contains("![Map](Guide.docx!/word/media/image1.png)"));
    assert!(body.contains("| Name | Value |"));
    assert!(body.contains("| --- | --- |"));
    assert!(object.links.iter().any(|link| {
        link.target == "https://example.com"
            && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert!(object.links.iter().any(|link| {
        link.target == "Guide.docx!/word/media/image1.png"
            && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert_eq!(staged.assets.len(), 1);
    assert_eq!(
        staged.assets[0].source_path,
        "Guide.docx!/word/media/image1.png"
    );
    assert_eq!(
        staged.assets[0].owner_object_id.as_deref(),
        Some(object.id.as_str())
    );
    assert_eq!(staged.summary.unresolved_link_count, 0);
}

#[test]
fn docx_analysis_rejects_unsafe_packages_and_xml() {
    let source = TestDirectory::new();
    let traversal = source.path().join("traversal.docx");
    write_zip(
        &traversal,
        &[
            ("[Content_Types].xml", DOCX_CONTENT_TYPES),
            ("word/document.xml", DOCX_DOCUMENT_XML),
            ("../outside.xml", b"outside"),
        ],
    );
    assert!(
        analyze_generic_documents(&traversal, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("escapes or is not normalized")
    );

    let dtd = source.path().join("dtd.docx");
    write_zip(
            &dtd,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                (
                    "word/document.xml",
                    br#"<?xml version="1.0"?><!DOCTYPE w:document [<!ENTITY x "expanded">]><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>&x;</w:t></w:r></w:p></w:body></w:document>"#,
                ),
            ],
        );
    assert!(
        analyze_generic_documents(&dtd, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("DTD")
    );

    let malformed = source.path().join("malformed.docx");
    write_zip(
        &malformed,
        &[
            ("[Content_Types].xml", DOCX_CONTENT_TYPES),
            ("word/document.xml", b"<w:document>"),
        ],
    );
    assert!(
        analyze_generic_documents(&malformed, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("invalid DOCX XML")
    );

    let bomb = source.path().join("bomb.docx");
    let repeated = vec![b' '; 256 * 1024];
    write_zip(
        &bomb,
        &[
            ("[Content_Types].xml", DOCX_CONTENT_TYPES),
            ("word/document.xml", repeated.as_slice()),
        ],
    );
    assert!(
        analyze_generic_documents(&bomb, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("compression-ratio limit")
    );
}

#[test]
fn docx_analysis_reports_omitted_active_and_unsupported_content() {
    let source = TestDirectory::new();
    let source_path = source.path().join("warnings.docx");
    write_zip(
        &source_path,
        &[
            ("[Content_Types].xml", DOCX_CONTENT_TYPES),
            ("word/document.xml", DOCX_DOCUMENT_XML),
            ("word/comments.xml", b"<comments/>"),
            ("word/vbaProject.bin", b"not executed"),
        ],
    );

    let staged =
        analyze_generic_documents(&source_path, GenericDocumentImportLimits::default()).unwrap();

    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "docx_comments_omitted"));
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "docx_active_content_removed"));
    assert!(staged.summary.warning_count >= 2);
}

#[test]
fn docx_commit_preserves_markdown_and_image_after_clean_rebuild() {
    let source = TestDirectory::new();
    let source_path = source.path().join("Guide.docx");
    write_docx_fixture(&source_path);
    let staged =
        analyze_generic_documents(&source_path, GenericDocumentImportLimits::default()).unwrap();
    let expected_body = staged.objects[0].body.as_ref().unwrap().body.clone();
    let project = TestDirectory::new();
    let store = ProjectStore::open_directory(project.path()).unwrap();
    let generation = store.content_generation().unwrap();
    let mut mappings = ImportMappingOverrides::default();
    mappings.global.entity_type = Some("note".into());
    let candidate = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "docx-session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: generation,
            current_content_generation: generation,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let validated = validate_import_candidate_plan(ImportValidationBuild {
        candidate,
        staged_objects: staged.objects,
        staged_assets: staged.assets,
        staged_unsupported: staged.unsupported,
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::new(),
            relationship_types: BTreeSet::new(),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::new(),
    })
    .unwrap()
    .plan
    .unwrap();
    let report = store
        .commit_external_import(
            &validated,
            Some(&source_path),
            true,
            "00000000-0000-4000-8000-000000000004",
        )
        .unwrap();

    assert_eq!(report.assets.len(), 1);
    assert_eq!(
        store
            .asset_bytes(report.assets[0].asset_id.clone())
            .unwrap(),
        b"\x89PNG\r\n\x1a\nfixture"
    );
    store.flush_checkpoint("DOCX import test").unwrap();
    drop(store);
    fs::remove_dir_all(project.path().join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
    let documents = rebuilt
        .list_documents(report.created[0].entity_id.clone())
        .unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].format, "markdown");
    assert_eq!(documents[0].body, expected_body);
    let assets = rebuilt
        .list_assets(report.created[0].entity_id.clone())
        .unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(
        rebuilt.asset_bytes(assets[0].id.clone()).unwrap(),
        b"\x89PNG\r\n\x1a\nfixture"
    );
}

#[test]
fn docx_asset_reader_supports_file_folder_and_archive_sources() {
    let source = TestDirectory::new();
    fs::create_dir_all(source.path().join("Docs")).unwrap();
    let docx_path = source.path().join("Docs/Guide.docx");
    write_docx_fixture(&docx_path);
    let expected = b"\x89PNG\r\n\x1a\nfixture";
    let docx_bytes = fs::read(&docx_path).unwrap();
    let docx_hash = hex_digest(&docx_bytes);

    assert_eq!(
        read_docx_import_asset_bytes(
            &docx_path,
            &ImportSourceKind::File,
            "Guide.docx!/word/media/image1.png",
            expected.len() as u64,
            &docx_hash,
        )
        .unwrap(),
        expected
    );
    assert_eq!(
        read_docx_import_asset_bytes(
            source.path(),
            &ImportSourceKind::Folder,
            "Docs/Guide.docx!/word/media/image1.png",
            expected.len() as u64,
            &docx_hash,
        )
        .unwrap(),
        expected
    );

    let archive_path = source.path().join("documents.zip");
    write_zip(&archive_path, &[("Docs/Guide.docx", &docx_bytes)]);
    let staged =
        analyze_generic_documents(&archive_path, GenericDocumentImportLimits::default()).unwrap();
    assert_eq!(staged.objects[0].source_kind, "docx");
    assert_eq!(
        staged.assets[0].source_path,
        "Docs/Guide.docx!/word/media/image1.png"
    );
    assert_eq!(
        read_docx_import_asset_bytes(
            &archive_path,
            &ImportSourceKind::Archive,
            "Docs/Guide.docx!/word/media/image1.png",
            expected.len() as u64,
            &docx_hash,
        )
        .unwrap(),
        expected
    );
    assert!(read_docx_import_asset_bytes(
        &docx_path,
        &ImportSourceKind::File,
        "Guide.docx!/word/media/image1.png",
        expected.len() as u64,
        "changed-package-hash",
    )
    .unwrap_err()
    .to_string()
    .contains("package changed"));
}

#[test]
fn obsidian_vault_preserves_frontmatter_and_resolves_vault_links_and_embeds() {
    let vault = TestDirectory::new();
    fs::create_dir_all(vault.path().join(".obsidian")).unwrap();
    fs::create_dir_all(vault.path().join("Characters")).unwrap();
    fs::create_dir_all(vault.path().join("Places")).unwrap();
    fs::create_dir_all(vault.path().join("assets")).unwrap();
    fs::write(vault.path().join(".obsidian/app.json"), "{}").unwrap();
    let home_body = r#"---
aliases:
  - The Grey
  - Mithrandir
tags: [wizard, fellowship]
type: person
species: Maia
rank: 7
homepage: "[Metadata](Missing.md)"
---
# Gandalf

Travel to [[Places/Middle Earth|Middle Earth]] or [[Middle Earth]].
![[assets/map.png|Map]]
![[Places/Middle Earth#North]]
`[[Ignored inline]]`
```text
[[Ignored fenced]]
```
> [!note] Unsupported plugin syntax remains verbatim.
"#;
    fs::write(vault.path().join("Characters/Gandalf.md"), home_body).unwrap();
    fs::write(
        vault.path().join("Places/Middle Earth.md"),
        "# Middle Earth\n\n## North\n",
    )
    .unwrap();
    fs::write(
        vault.path().join("assets/map.png"),
        b"\x89PNG\r\n\x1a\nfixture",
    )
    .unwrap();

    let staged =
        analyze_obsidian_vault(vault.path(), GenericDocumentImportLimits::default()).unwrap();
    let gandalf = staged
        .objects
        .iter()
        .find(|object| object.source_path == "Characters/Gandalf.md")
        .unwrap();

    assert_eq!(staged.importer.id, OBSIDIAN_IMPORTER_ID);
    assert_eq!(staged.source.kind, ImportSourceKind::Vault);
    assert_eq!(gandalf.source_kind, "obsidian_markdown");
    assert_eq!(gandalf.aliases, vec!["Mithrandir", "The Grey"]);
    assert_eq!(gandalf.tags, vec!["fellowship", "wizard"]);
    assert_eq!(gandalf.fields["species"], "Maia");
    assert_eq!(gandalf.fields["rank"], 7);
    assert!(!gandalf.links.iter().any(|link| link.target == "Missing.md"));
    assert_eq!(gandalf.body.as_ref().unwrap().body, home_body);
    assert!(gandalf.raw_source_data["frontmatter"]
        .as_str()
        .unwrap()
        .contains("type: person"));
    assert!(gandalf.mapping_hints.iter().any(|hint| {
        hint.kind == MappingHintKind::EntityType && hint.suggested_value == "person"
    }));
    assert_eq!(
        gandalf
            .links
            .iter()
            .filter(|link| link.raw.is_some())
            .count(),
        4
    );
    assert!(gandalf.links.iter().any(|link| {
        link.target == "Middle Earth" && link.resolution == StagedLinkResolution::Resolved
    }));
    assert!(gandalf.links.iter().any(|link| {
        link.target == "Places/Middle Earth#North"
            && link.kind == StagedLinkKind::Embed
            && link.resolution == StagedLinkResolution::Resolved
    }));
    assert!(gandalf.links.iter().any(|link| {
        link.target == "assets/map.png"
            && link.kind == StagedLinkKind::Embed
            && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert_eq!(staged.assets.len(), 1);
    assert_eq!(
        staged.assets[0].owner_object_id.as_deref(),
        Some(gandalf.id.as_str())
    );
    assert!(staged.unsupported.iter().any(|item| {
        item.source_path == ".obsidian" && item.source_kind == "obsidian_configuration"
    }));
}

#[test]
fn obsidian_vault_reports_ambiguous_missing_and_partial_frontmatter() {
    let vault = TestDirectory::new();
    fs::create_dir_all(vault.path().join("A")).unwrap();
    fs::create_dir_all(vault.path().join("B")).unwrap();
    fs::write(vault.path().join("A/Twin.md"), "# First").unwrap();
    fs::write(vault.path().join("B/Twin.md"), "# Second").unwrap();
    fs::write(
        vault.path().join("Home.md"),
        "---\ncustom:\n  nested: value\n---\n[[Twin]] [[Missing]]",
    )
    .unwrap();

    let staged =
        analyze_obsidian_vault(vault.path(), GenericDocumentImportLimits::default()).unwrap();
    let home = staged
        .objects
        .iter()
        .find(|object| object.source_path == "Home.md")
        .unwrap();
    let twin = home
        .links
        .iter()
        .find(|link| link.target == "Twin")
        .unwrap();
    let missing = home
        .links
        .iter()
        .find(|link| link.target == "Missing")
        .unwrap();

    assert_eq!(twin.resolution, StagedLinkResolution::Ambiguous);
    assert_eq!(twin.candidate_object_ids.len(), 2);
    assert_eq!(missing.resolution, StagedLinkResolution::Missing);
    assert_eq!(home.fields["custom"], "  nested: value");
    for code in [
        "obsidian_target_ambiguous",
        "obsidian_target_missing",
        "obsidian_frontmatter_partial",
    ] {
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code));
    }
}

#[test]
fn generic_markdown_does_not_apply_obsidian_semantics() {
    let source = TestDirectory::new();
    fs::write(
        source.path().join("Note.md"),
        "---\naliases: [Alias]\n---\n[[Other]]",
    )
    .unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let object = &staged.objects[0];

    assert!(object.aliases.is_empty());
    assert!(object.links.is_empty());
    assert_eq!(object.fields.len(), 1);
    assert!(object.fields["frontmatter"]
        .as_str()
        .unwrap()
        .contains("aliases"));
}

#[test]
fn obsidian_frontmatter_preserves_a_lone_double_quote_without_panicking() {
    let parsed = parse_obsidian_frontmatter("malformed: \"");

    assert_eq!(parsed.fields["malformed"], "\"");
}

#[test]
fn obsidian_link_scanner_ignores_code_spans_fences_and_escaped_embeds() {
    let marker = char::from(0x60);
    let inline_fence = marker.to_string().repeat(2);
    let inner_fence = marker.to_string().repeat(3);
    let block_fence = marker.to_string().repeat(4);
    let body = format!(
        "{inline_fence}[[Hidden inline]]{inline_fence} [[Visible]]\n\
             {block_fence}text\n\
             {inner_fence} [[Still fenced]]\n\
             {block_fence}\n\
             \\![[Escaped embed]] \\\\![[Visible embed]]"
    );

    let links = discover_obsidian_links(&body);

    assert_eq!(
        links
            .iter()
            .map(|link| link.target.as_str())
            .collect::<Vec<_>>(),
        vec!["Visible", "Visible embed"]
    );
    assert_eq!(links[1].kind, StagedLinkKind::Embed);
}

#[test]
fn obsidian_vault_commit_preserves_markdown_and_attachment_after_clean_rebuild() {
    let vault = TestDirectory::new();
    fs::create_dir_all(vault.path().join("assets")).unwrap();
    let body =
        "---\naliases: [Start]\ntags: [lore]\n---\n# Home\n\n[[Target]]\n\n![[assets/map.png]]\n";
    fs::write(vault.path().join("Home.md"), body).unwrap();
    fs::write(vault.path().join("Target.md"), "# Target\n").unwrap();
    fs::write(
        vault.path().join("assets/map.png"),
        b"\x89PNG\r\n\x1a\nfixture",
    )
    .unwrap();
    let staged =
        analyze_obsidian_vault(vault.path(), GenericDocumentImportLimits::default()).unwrap();
    let project = TestDirectory::new();
    let store = ProjectStore::open_directory(project.path()).unwrap();
    let generation = store.content_generation().unwrap();
    let mut mappings = ImportMappingOverrides::default();
    mappings.global.entity_type = Some("note".into());
    mappings
        .global
        .relationship_mappings
        .insert("internal".into(), "references".into());
    let candidate = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "obsidian-session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: generation,
            current_content_generation: generation,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let validated = validate_import_candidate_plan(ImportValidationBuild {
        candidate,
        staged_objects: staged.objects,
        staged_assets: staged.assets,
        staged_unsupported: staged.unsupported,
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::new(),
            relationship_types: BTreeSet::from(["references".into()]),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::new(),
    })
    .unwrap()
    .plan
    .unwrap();
    let report = store
        .commit_external_import(
            &validated,
            Some(vault.path()),
            true,
            "00000000-0000-4000-8000-000000000006",
        )
        .unwrap();

    assert_eq!(report.created.len(), 2);
    assert_eq!(report.assets.len(), 1);
    assert_eq!(report.relationships.len(), 1);
    store.flush_checkpoint("Obsidian import test").unwrap();
    drop(store);
    fs::remove_dir_all(project.path().join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
    let entity_id = report
        .created
        .iter()
        .find(|item| item.source_path == "Home.md")
        .unwrap()
        .entity_id
        .clone();
    let documents = rebuilt.list_documents(entity_id.clone()).unwrap();
    assert_eq!(documents[0].body, body);
    assert_eq!(
        rebuilt.list_relationships(entity_id.clone()).unwrap().len(),
        1
    );
    let source_fields = rebuilt.list_fields(entity_id.clone()).unwrap();
    let source_context = source_fields
        .iter()
        .find(|field| field.key.starts_with("externalImportSource."))
        .and_then(|field| field.value.get("sourceContext"))
        .unwrap();
    assert_eq!(source_context["aliases"], serde_json::json!(["Start"]));
    assert_eq!(source_context["tags"], serde_json::json!(["lore"]));
    let assets = rebuilt.list_assets(entity_id).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(
        rebuilt.asset_bytes(assets[0].id.clone()).unwrap(),
        b"\x89PNG\r\n\x1a\nfixture"
    );
}

#[test]
fn obsidian_import_rejects_single_files() {
    let source = TestDirectory::new();
    let note = source.path().join("Note.md");
    fs::write(&note, "# Note").unwrap();

    assert!(
        analyze_obsidian_vault(&note, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("requires a vault folder")
    );
}

fn write_mediawiki_fixture(path: &Path) {
    fs::write(
            path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<mediawiki xmlns="http://www.mediawiki.org/xml/export-0.11/">
  <siteinfo>
    <sitename>Example Wiki</sitename><dbname>example</dbname>
    <base>https://example.test/wiki/Main_Page</base>
    <generator>MediaWiki 1.45</generator><case>first-letter</case>
    <namespaces><namespace key="0" case="first-letter"></namespace><namespace key="10" case="first-letter">Template</namespace></namespaces>
  </siteinfo>
  <page>
    <title>Gandalf</title><ns>0</ns><id>1</id>
    <revision><id>11</id><timestamp>2025-02-01T00:00:00Z</timestamp>
      <contributor><username>Archivist</username></contributor>
      <model>wikitext</model><format>text/x-wiki</format>
      <text xml:space="preserve"><![CDATA[{{Infobox person
| born = Before the First Age
| location = [[Middle Earth]]
}}
'''Gandalf''' travels through [[Middle_Earth|Middle Earth]].
[[Category:Characters]]
[[File:Gandalf.png|thumb]]
]]></text><sha1>new-hash</sha1>
    </revision>
    <revision><id>10</id><timestamp>2025-01-01T00:00:00Z</timestamp>
      <model>wikitext</model><format>text/x-wiki</format><text xml:space="preserve">Older revision</text>
    </revision>
  </page>
  <page><title>Middle Earth</title><ns>0</ns><id>2</id>
    <revision><id>20</id><timestamp>2025-02-02T00:00:00Z</timestamp>
      <model>wikitext</model><format>text/x-wiki</format><text xml:space="preserve">A world &amp; realm.</text>
    </revision>
  </page>
  <page><title>Mithrandir</title><ns>0</ns><id>3</id><redirect title="Gandalf" />
    <revision><id>30</id><timestamp>2025-02-03T00:00:00Z</timestamp>
      <model>wikitext</model><format>text/x-wiki</format><text xml:space="preserve">#REDIRECT [[Gandalf]]</text>
    </revision>
  </page>
</mediawiki>"#,
        )
        .unwrap();
}

#[test]
fn mediawiki_analysis_streams_latest_pages_and_preserves_wikitext_metadata() {
    let source = TestDirectory::new();
    let source_path = source.path().join("wiki.xml");
    write_mediawiki_fixture(&source_path);

    let staged =
        analyze_mediawiki_xml(&source_path, GenericDocumentImportLimits::default()).unwrap();
    let gandalf = staged
        .objects
        .iter()
        .find(|object| object.title == "Gandalf")
        .unwrap();
    let middle_earth = staged
        .objects
        .iter()
        .find(|object| object.title == "Middle Earth")
        .unwrap();

    assert_eq!(staged.importer.id, MEDIAWIKI_IMPORTER_ID);
    assert_eq!(staged.source.kind, ImportSourceKind::WikiDump);
    assert_eq!(staged.objects.len(), 3);
    assert_eq!(gandalf.source_kind, "mediawiki_page");
    assert!(gandalf
        .body
        .as_ref()
        .unwrap()
        .body
        .contains("Before the First Age"));
    assert!(!gandalf
        .body
        .as_ref()
        .unwrap()
        .body
        .contains("Older revision"));
    assert_eq!(
        gandalf.raw_source_data["wikitext"],
        gandalf.body.as_ref().unwrap().body
    );
    assert_eq!(gandalf.raw_source_data["latest_revision"]["id"], "11");
    assert_eq!(gandalf.metadata["wiki_generator"], "MediaWiki 1.45");
    assert_eq!(gandalf.fields["source_format"], "text/x-wiki");
    assert_eq!(gandalf.fields["infobox.born"], "Before the First Age");
    assert_eq!(gandalf.tags, vec!["Characters"]);
    assert_eq!(middle_earth.body.as_ref().unwrap().body, "A world & realm.");
    assert!(gandalf.mapping_hints.iter().any(|hint| {
        hint.kind == MappingHintKind::Field && hint.source_key.as_deref() == Some("infobox.born")
    }));
    assert!(gandalf.links.iter().any(|link| {
        link.target == "Middle_Earth"
            && link.resolution == StagedLinkResolution::Resolved
            && link.resolved_object_id.as_deref() == Some(middle_earth.id.as_str())
    }));
    assert!(gandalf.links.iter().any(|link| {
        link.target == "File:Gandalf.png"
            && link.kind == StagedLinkKind::Embed
            && link.resolution == StagedLinkResolution::NotApplicable
    }));
    assert!(gandalf.aliases.contains(&"Mithrandir".into()));
    assert_eq!(staged.unsupported.len(), 1);
    assert_eq!(
        staged.unsupported[0].source_kind,
        "mediawiki_revision_history"
    );
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "mediawiki_revision_history_omitted"));
}

#[test]
fn mediawiki_analysis_rejects_dtd_malformed_xml_and_page_limits() {
    let source = TestDirectory::new();
    let dtd = source.path().join("dtd.xml");
    fs::write(
            &dtd,
            r#"<?xml version="1.0"?><!DOCTYPE mediawiki [<!ENTITY x "expanded">]><mediawiki><page><title>&x;</title></page></mediawiki>"#,
        )
        .unwrap();
    assert!(
        analyze_mediawiki_xml(&dtd, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("DTD")
    );

    let malformed = source.path().join("malformed.xml");
    fs::write(&malformed, "<mediawiki><page></mediawiki>").unwrap();
    assert!(
        analyze_mediawiki_xml(&malformed, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("invalid MediaWiki XML")
    );

    let limited = source.path().join("limited.xml");
    write_mediawiki_fixture(&limited);
    let limits = GenericDocumentImportLimits {
        max_files: 2,
        ..Default::default()
    };
    assert!(analyze_mediawiki_xml(&limited, limits)
        .unwrap_err()
        .to_string()
        .contains("maximum page count"));

    let limits = GenericDocumentImportLimits {
        max_file_bytes: 8,
        ..Default::default()
    };
    assert!(analyze_mediawiki_xml(&limited, limits)
        .unwrap_err()
        .to_string()
        .contains("maximum page size"));
}

#[test]
fn mediawiki_links_keep_ambiguous_and_missing_targets_reviewable() {
    let source = TestDirectory::new();
    let source_path = source.path().join("links.xml");
    fs::write(
            &source_path,
            r#"<mediawiki>
<page><title>Home</title><ns>0</ns><id>1</id><revision><id>1</id><text>[[Twin]] [[Missing]]</text></revision></page>
<page><title>Twin</title><ns>0</ns><id>2</id><revision><id>2</id><text>First</text></revision></page>
<page><title>Twin</title><ns>1</ns><id>3</id><revision><id>3</id><text>Second</text></revision></page>
</mediawiki>"#,
        )
        .unwrap();

    let staged =
        analyze_mediawiki_xml(&source_path, GenericDocumentImportLimits::default()).unwrap();
    let home = staged
        .objects
        .iter()
        .find(|object| object.title == "Home")
        .unwrap();
    let twin = home
        .links
        .iter()
        .find(|link| link.target == "Twin")
        .unwrap();
    let missing = home
        .links
        .iter()
        .find(|link| link.target == "Missing")
        .unwrap();

    assert_eq!(twin.resolution, StagedLinkResolution::Ambiguous);
    assert_eq!(twin.candidate_object_ids.len(), 2);
    assert_eq!(missing.resolution, StagedLinkResolution::Missing);
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "mediawiki_target_ambiguous"));
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "mediawiki_target_missing"));
}

#[test]
fn mediawiki_streaming_progress_can_cancel_before_the_dump_completes() {
    let source = TestDirectory::new();
    let source_path = source.path().join("large.xml");
    let mut xml = String::from("<mediawiki>");
    for page in 0..500 {
        use std::fmt::Write as _;
        write!(
                xml,
                "<page><title>Page {page}</title><ns>0</ns><id>{page}</id><revision><id>{page}</id><text>Body {page}</text></revision></page>"
            )
            .unwrap();
    }
    xml.push_str("</mediawiki>");
    fs::write(&source_path, xml).unwrap();
    let mut callbacks = 0;

    let error = analyze_mediawiki_xml_with_progress(
        &source_path,
        GenericDocumentImportLimits::default(),
        |progress| {
            callbacks += 1;
            if progress.processed_entries >= 25 {
                Err(CoreError::Conflict(
                    EXTERNAL_IMPORT_ANALYSIS_CANCELLED.into(),
                ))
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), EXTERNAL_IMPORT_ANALYSIS_CANCELLED);
    assert!(callbacks > 1);
    assert!(callbacks < 500);
}

#[test]
fn mediawiki_commit_preserves_latest_wikitext_after_clean_rebuild() {
    let source = TestDirectory::new();
    let source_path = source.path().join("wiki.xml");
    write_mediawiki_fixture(&source_path);
    let staged =
        analyze_mediawiki_xml(&source_path, GenericDocumentImportLimits::default()).unwrap();
    let gandalf_staged_id = staged
        .objects
        .iter()
        .find(|object| object.title == "Gandalf")
        .unwrap()
        .id
        .clone();
    let expected = staged
        .objects
        .iter()
        .find(|object| object.id == gandalf_staged_id)
        .unwrap()
        .body
        .as_ref()
        .unwrap()
        .body
        .clone();
    let project = TestDirectory::new();
    let store = ProjectStore::open_directory(project.path()).unwrap();
    let generation = store.content_generation().unwrap();
    let mut mappings = ImportMappingOverrides::default();
    mappings.global.entity_type = Some("note".into());
    mappings
        .global
        .relationship_mappings
        .insert("internal".into(), "references".into());
    mappings
        .global
        .field_mappings
        .insert("infobox.born".into(), "wiki:born".into());
    let candidate = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "mediawiki-session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: generation,
            current_content_generation: generation,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let validated = validate_import_candidate_plan(ImportValidationBuild {
        candidate,
        staged_objects: staged.objects,
        staged_assets: staged.assets,
        staged_unsupported: staged.unsupported,
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::from([(
                "wiki:born".into(),
                ImportFieldTarget {
                    namespace: "wiki".into(),
                    key: "born".into(),
                    entity_types: BTreeSet::from(["note".into()]),
                    field_type: "text".into(),
                    required: false,
                    multiple: false,
                    options: BTreeSet::new(),
                    one_of: Vec::new(),
                },
            )]),
            relationship_types: BTreeSet::from(["references".into()]),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::new(),
    })
    .unwrap()
    .plan
    .unwrap();
    let report = store
        .commit_external_import(
            &validated,
            Some(&source_path),
            true,
            "00000000-0000-4000-8000-000000000007",
        )
        .unwrap();

    let gandalf_entity_id = report
        .created
        .iter()
        .find(|created| created.staged_object_id == gandalf_staged_id)
        .unwrap()
        .entity_id
        .clone();
    assert_eq!(report.relationships.len(), 2);
    store.flush_checkpoint("MediaWiki import test").unwrap();
    drop(store);
    fs::remove_dir_all(project.path().join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
    let documents = rebuilt.list_documents(gandalf_entity_id.clone()).unwrap();
    assert_eq!(documents[0].body, expected);
    assert_eq!(
        rebuilt
            .list_relationships(gandalf_entity_id.clone())
            .unwrap()
            .into_iter()
            .filter(|relationship| relationship.source_id == gandalf_entity_id)
            .count(),
        1
    );
    let source_fields = rebuilt.list_fields(gandalf_entity_id).unwrap();
    let source_context = source_fields
        .iter()
        .find(|field| field.key.starts_with("externalImportSource."))
        .and_then(|field| field.value.get("sourceContext"))
        .unwrap();
    assert_eq!(source_context["tags"], serde_json::json!(["Characters"]));
    assert_eq!(
        source_context["unmappedFields"]["templates"][0],
        "Infobox person"
    );
    assert_eq!(
        source_fields
            .iter()
            .find(|field| field.namespace == "wiki" && field.key == "born")
            .unwrap()
            .value,
        "Before the First Age"
    );
}

#[test]
fn zip_analysis_matches_folder_structure_and_content() {
    let folder = TestDirectory::new();
    fs::create_dir_all(folder.path().join("Notes")).unwrap();
    fs::create_dir_all(folder.path().join("assets")).unwrap();
    let note = b"# Note\n\n[Other](Other.md)\n![Map](../assets/map.png)\n";
    let other = b"# Other\n";
    let image = b"\x89PNG\r\n\x1a\nfixture";
    fs::write(folder.path().join("Notes/Note.md"), note).unwrap();
    fs::write(folder.path().join("Notes/Other.md"), other).unwrap();
    fs::write(folder.path().join("assets/map.png"), image).unwrap();
    let archive_directory = TestDirectory::new();
    let archive_path = archive_directory.path().join("fixture.zip");
    write_zip(
        &archive_path,
        &[
            ("Notes/Note.md", note),
            ("Notes/Other.md", other),
            ("assets/map.png", image),
        ],
    );

    let folder_result =
        analyze_generic_documents(folder.path(), GenericDocumentImportLimits::default()).unwrap();
    let archive_result =
        analyze_generic_documents(&archive_path, GenericDocumentImportLimits::default()).unwrap();

    assert_eq!(archive_result.source.kind, ImportSourceKind::Archive);
    assert_eq!(
        archive_result
            .objects
            .iter()
            .map(|object| (&object.source_path, &object.title, &object.body))
            .collect::<Vec<_>>(),
        folder_result
            .objects
            .iter()
            .map(|object| (&object.source_path, &object.title, &object.body))
            .collect::<Vec<_>>()
    );
    assert_eq!(archive_result.assets.len(), 1);
    assert_eq!(archive_result.assets[0].source_path, "assets/map.png");
    assert_eq!(
        archive_result.assets[0].content_hash,
        folder_result.assets[0].content_hash
    );
    assert_eq!(
        archive_result
            .objects
            .iter()
            .flat_map(|object| object.links.iter())
            .map(|link| (&link.kind, &link.target, &link.resolution))
            .collect::<Vec<_>>(),
        folder_result
            .objects
            .iter()
            .flat_map(|object| object.links.iter())
            .map(|link| (&link.kind, &link.target, &link.resolution))
            .collect::<Vec<_>>()
    );
    assert_eq!(archive_result.summary, folder_result.summary);
}

#[test]
fn zip_analysis_rejects_traversal_bombs_and_malformed_archives() {
    let source = TestDirectory::new();
    let traversal = source.path().join("traversal.zip");
    write_zip(&traversal, &[("../outside.md", b"outside")]);
    assert!(
        analyze_generic_documents(&traversal, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("escapes or is not normalized")
    );

    let symlink = source.path().join("symlink.zip");
    let file = fs::File::create(&symlink).unwrap();
    let mut archive = zip::ZipWriter::new(file);
    archive
        .add_symlink(
            "linked.md",
            "target.md",
            SimpleFileOptions::default().unix_permissions(0o777),
        )
        .unwrap();
    archive.finish().unwrap();
    assert!(
        analyze_generic_documents(&symlink, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("links and special files")
    );

    let collision = source.path().join("collision.zip");
    write_zip(&collision, &[("Note.md", b"one"), ("note.md", b"two")]);
    assert!(
        analyze_generic_documents(&collision, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("duplicate or case-colliding")
    );

    let bomb = source.path().join("bomb.zip");
    let repeated = vec![0_u8; 256 * 1024];
    write_zip(&bomb, &[("bomb.md", repeated.as_slice())]);
    assert!(
        analyze_generic_documents(&bomb, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("compression ratio")
    );

    let malformed = source.path().join("malformed.zip");
    fs::write(&malformed, b"not a ZIP archive").unwrap();
    assert!(
        analyze_generic_documents(&malformed, GenericDocumentImportLimits::default())
            .unwrap_err()
            .to_string()
            .contains("invalid ZIP archive")
    );
}

#[test]
fn zip_central_directory_preflight_can_be_cancelled() {
    let source = TestDirectory::new();
    let archive_path = source.path().join("cancel.zip");
    write_zip(&archive_path, &[("one.md", b"one"), ("two.md", b"two")]);

    let error = analyze_generic_documents_with_progress(
        &archive_path,
        GenericDocumentImportLimits::default(),
        |progress| {
            if progress.source_path.is_some() {
                Err(CoreError::Conflict(
                    EXTERNAL_IMPORT_ANALYSIS_CANCELLED.into(),
                ))
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), EXTERNAL_IMPORT_ANALYSIS_CANCELLED);
}

#[test]
fn zip_attachment_commit_survives_checkpoint_rebuild() {
    let source = TestDirectory::new();
    let archive_path = source.path().join("fixture.zip");
    let note = b"# Note\n\n![Map](assets/map.png)\n";
    let image = b"\x89PNG\r\n\x1a\nfixture";
    write_zip(
        &archive_path,
        &[("Note.md", note), ("assets/map.png", image)],
    );
    let staged =
        analyze_generic_documents(&archive_path, GenericDocumentImportLimits::default()).unwrap();
    let project = TestDirectory::new();
    let store = ProjectStore::open_directory(project.path()).unwrap();
    let generation = store.content_generation().unwrap();
    let mut mappings = ImportMappingOverrides::default();
    mappings.global.entity_type = Some("note".into());
    let candidate = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "zip-session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: generation,
            current_content_generation: generation,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: staged.unsupported.len(),
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let validated = validate_import_candidate_plan(ImportValidationBuild {
        candidate,
        staged_objects: staged.objects,
        staged_assets: staged.assets,
        staged_unsupported: staged.unsupported,
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::new(),
            relationship_types: BTreeSet::new(),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::new(),
    })
    .unwrap()
    .plan
    .unwrap();
    let report = store
        .commit_external_import(
            &validated,
            Some(&archive_path),
            true,
            "00000000-0000-4000-8000-000000000002",
        )
        .unwrap();
    assert_eq!(report.created.len(), 1);
    assert_eq!(report.assets.len(), 1);
    assert_eq!(
        store
            .asset_bytes(report.assets[0].asset_id.clone())
            .unwrap(),
        image
    );
    store.flush_checkpoint("ZIP import test").unwrap();
    drop(store);
    fs::remove_dir_all(project.path().join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
    let assets = rebuilt
        .list_assets(report.created[0].entity_id.clone())
        .unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(rebuilt.asset_bytes(assets[0].id.clone()).unwrap(), image);
}

#[test]
fn markdown_analysis_rejects_malformed_asset_signatures() {
    let source = TestDirectory::new();
    fs::write(source.path().join("fake.png"), b"not a png").unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    assert!(staged.assets.is_empty());
    assert_eq!(staged.unsupported.len(), 1);
    assert!(staged
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "invalid_asset_content"));
}

#[test]
fn validation_enforces_mapped_field_types_and_required_fields() {
    let source = TestDirectory::new();
    fs::write(
        source.path().join("note.md"),
        "---\ncount: many\n---\n# Note",
    )
    .unwrap();
    let staged =
        analyze_obsidian_vault(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let mut mappings = ImportMappingOverrides::default();
    mappings.global.entity_type = Some("note".into());
    let candidate_without_field = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: 1,
            current_content_generation: 1,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: 0,
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let field_target = ImportFieldTarget {
        namespace: "core".into(),
        key: "count".into(),
        entity_types: BTreeSet::from(["note".into()]),
        field_type: "number".into(),
        required: true,
        multiple: false,
        options: BTreeSet::new(),
        one_of: Vec::new(),
    };
    let build = |candidate, target| ImportValidationBuild {
        candidate,
        staged_objects: staged.objects.clone(),
        staged_assets: staged.assets.clone(),
        staged_unsupported: staged.unsupported.clone(),
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::from([("core:count".into(), target)]),
            relationship_types: BTreeSet::new(),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::new(),
    };
    let missing =
        validate_import_candidate_plan(build(candidate_without_field, field_target.clone()))
            .unwrap();
    assert!(missing
        .issues
        .iter()
        .any(|issue| issue.code == "required_target_field_missing"));

    mappings
        .global
        .field_mappings
        .insert("count".into(), "core:count".into());
    let candidate_with_field = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer.clone(),
            source: staged.source.clone(),
            captured_content_generation: 1,
            current_content_generation: 1,
            manifest_fingerprint: "manifest-v1".into(),
            objects: staged.objects.clone(),
            unsupported_count: 0,
            diagnostics: staged.diagnostics.clone(),
        },
        &mappings,
    )
    .unwrap();
    let invalid =
        validate_import_candidate_plan(build(candidate_with_field.clone(), field_target.clone()))
            .unwrap();
    assert!(invalid
        .issues
        .iter()
        .any(|issue| issue.code == "target_field_value_invalid"));

    let valid = validate_import_candidate_plan(build(
        candidate_with_field,
        ImportFieldTarget {
            field_type: "text".into(),
            ..field_target
        },
    ))
    .unwrap();
    assert!(valid.plan.is_some());
}

#[test]
fn validation_requires_explicit_duplicate_decision_and_uses_enabled_catalog() {
    let source = TestDirectory::new();
    fs::write(source.path().join("note.md"), "# Note").unwrap();
    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();
    let object = staged.objects[0].clone();
    let mut overrides = ImportMappingOverrides::default();
    overrides.global.entity_type = Some("note".into());
    let candidate = build_import_candidate_plan(
        ImportCandidatePlanBuild {
            session_id: "session".into(),
            importer: staged.importer,
            source: staged.source,
            captured_content_generation: 4,
            current_content_generation: 4,
            manifest_fingerprint: "manifest-v1".into(),
            objects: vec![object.clone()],
            unsupported_count: 0,
            diagnostics: Vec::new(),
        },
        &overrides,
    )
    .unwrap();
    let build = ImportValidationBuild {
        candidate,
        staged_objects: vec![object.clone()],
        staged_assets: vec![StagedAsset {
            id: "asset".into(),
            source_path: "map.png".into(),
            filename: "map.png".into(),
            size: 8,
            mime_type: Some("image/png".into()),
            content_hash: Some(format!("sha256:{}", "0".repeat(64))),
            owner_object_id: Some(object.id.clone()),
            relationship: Some("attachment".into()),
            raw_metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        }],
        staged_unsupported: Vec::new(),
        catalog: ImportMappingCatalog {
            fingerprint: "manifest-v1".into(),
            entity_types: BTreeSet::from(["note".into()]),
            fields: BTreeMap::new(),
            relationship_types: BTreeSet::new(),
        },
        decisions: BTreeMap::new(),
        existing_targets: BTreeMap::new(),
        duplicate_targets: BTreeMap::from([(object.id.clone(), vec!["existing".into()])]),
    };

    let unresolved = validate_import_candidate_plan(build.clone()).unwrap();
    assert!(unresolved.plan.is_none());
    assert!(unresolved
        .issues
        .iter()
        .any(|issue| issue.code == "duplicate_source_identity"));

    let accepted = validate_import_candidate_plan(ImportValidationBuild {
        decisions: BTreeMap::from([(object.id, ImportObjectDecision::Create)]),
        ..build
    })
    .unwrap();
    let plan = accepted.plan.unwrap();
    assert_eq!(plan.content_generation, 4);
    assert_eq!(plan.objects.len(), 1);
    assert_eq!(plan.assets.len(), 1);
    assert_eq!(plan.objects[0].entity_type.as_deref(), Some("note"));
}

#[cfg(unix)]
#[test]
fn folder_analysis_reports_symlinks_without_following_them() {
    use std::os::unix::fs::symlink;

    let source = TestDirectory::new();
    let outside = TestDirectory::new();
    fs::write(outside.path().join("secret.md"), "not imported").unwrap();
    symlink(outside.path(), source.path().join("linked")).unwrap();

    let staged =
        analyze_generic_documents(source.path(), GenericDocumentImportLimits::default()).unwrap();

    assert!(staged.objects.is_empty());
    assert_eq!(staged.unsupported.len(), 1);
    assert_eq!(staged.unsupported[0].source_path, "linked");
    assert_eq!(staged.unsupported[0].source_kind, "symlink");

    let error = analyze_generic_documents(
        source.path().join("linked"),
        GenericDocumentImportLimits::default(),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("import source root cannot be a symbolic link"));
}
