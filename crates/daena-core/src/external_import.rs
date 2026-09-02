use crate::CoreError;
use pulldown_cmark::{Event, Options, Parser, Tag};
use quick_xml::encoding::Decoder as XmlDecoder;
use quick_xml::events::{BytesStart, Event as XmlEvent};
use quick_xml::Reader as XmlReader;
use quick_xml::XmlVersion;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

mod archive;
mod candidate;
mod docx;
mod generic;
mod html;
mod mediawiki;
mod model;
mod obsidian;

pub use self::candidate::{build_import_candidate_plan, validate_import_candidate_plan};
use self::docx::*;
pub use self::model::{
    ExternalImportCommitReport, GenericDocumentImportLimits, ImportAnalysisProgress,
    ImportAnalysisSummary, ImportCandidateIssue, ImportCandidateMapping, ImportCandidateObject,
    ImportCandidatePlan, ImportCandidatePlanBuild, ImportDecisionReport, ImportDiagnostic,
    ImportDiagnosticSeverity, ImportExistingTarget, ImportFieldTarget, ImportFieldVariant,
    ImportMappingCatalog, ImportMappingDecision, ImportMappingOverrides,
    ImportMissingReferenceReport, ImportObjectDecision, ImportSource, ImportSourceKind,
    ImportValidationBuild, ImportValidationIssue, ImportValidationOutcome,
    ImportValidationSeverity, ImportedAssetReport, ImportedFieldReport, ImportedObjectReport,
    ImportedRelationshipReport, ImporterIdentity, MappingHintKind, StagedAsset, StagedDocument,
    StagedImport, StagedLink, StagedLinkKind, StagedLinkResolution, StagedMappingHint,
    StagedObject, UnsupportedSourceData, ValidatedImportAsset, ValidatedImportField,
    ValidatedImportObject, ValidatedImportPlan, ValidatedImportRelationship,
    ValidatedImportSourceContext, EXTERNAL_IMPORT_ANALYSIS_CANCELLED, GENERIC_DOCUMENT_IMPORTER_ID,
    GENERIC_DOCUMENT_IMPORTER_VERSION, IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION, MEDIAWIKI_IMPORTER_ID,
    MEDIAWIKI_IMPORTER_VERSION, OBSIDIAN_IMPORTER_ID, OBSIDIAN_IMPORTER_VERSION,
    STAGED_IMPORT_SCHEMA_VERSION, VALIDATED_IMPORT_PLAN_SCHEMA_VERSION,
};

use self::mediawiki::*;
use self::obsidian::*;

use self::archive::*;
pub(crate) use self::archive::{
    is_docx_import_asset_source_path, read_archive_asset_bytes, read_docx_import_asset_bytes,
};
use self::generic::*;
use self::html::*;

pub fn analyze_generic_documents(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
) -> Result<StagedImport, CoreError> {
    analyze_generic_documents_with_progress(source, limits, |_| Ok(()))
}

pub fn analyze_generic_documents_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    analyze_documents_with_progress(source, limits, ImportProfile::Generic, progress)
}

pub fn analyze_obsidian_vault(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
) -> Result<StagedImport, CoreError> {
    analyze_obsidian_vault_with_progress(source, limits, |_| Ok(()))
}

pub fn analyze_obsidian_vault_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    analyze_documents_with_progress(source, limits, ImportProfile::Obsidian, progress)
}

pub fn analyze_mediawiki_xml(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
) -> Result<StagedImport, CoreError> {
    analyze_mediawiki_xml_with_progress(source, limits, |_| Ok(()))
}

pub fn analyze_mediawiki_xml_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    mut progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    validate_limits(&limits)?;
    let source = source.as_ref();
    let metadata = fs::symlink_metadata(source).map_err(|source| CoreError::Io {
        operation: "read MediaWiki import source metadata",
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CoreError::Validation(
            "MediaWiki import requires a regular XML file".into(),
        ));
    }
    if metadata.len() > MAX_MEDIAWIKI_SOURCE_BYTES {
        return Err(CoreError::Validation(format!(
            "MediaWiki XML exceeds the maximum source size of {MAX_MEDIAWIKI_SOURCE_BYTES} bytes"
        )));
    }
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| CoreError::Validation("MediaWiki XML filename is not valid UTF-8".into()))?
        .to_owned();
    let canonical_source = fs::canonicalize(source).map_err(|source| CoreError::Io {
        operation: "resolve MediaWiki import source path",
        source,
    })?;
    let source_id = hex_digest(canonical_source.to_string_lossy().as_bytes());
    let file = fs::File::open(source).map_err(|source| CoreError::Io {
        operation: "open MediaWiki import source",
        source,
    })?;
    let mut analyzer = MediaWikiAnalyzer {
        limits,
        import: StagedImport {
            schema_version: STAGED_IMPORT_SCHEMA_VERSION,
            importer: ImporterIdentity {
                id: MEDIAWIKI_IMPORTER_ID.into(),
                version: MEDIAWIKI_IMPORTER_VERSION.into(),
                name: "MediaWiki XML".into(),
            },
            source: ImportSource {
                id: source_id,
                kind: ImportSourceKind::WikiDump,
                display_name: source_name.clone(),
            },
            objects: Vec::new(),
            assets: Vec::new(),
            unsupported: Vec::new(),
            diagnostics: Vec::new(),
            summary: ImportAnalysisSummary::default(),
        },
        source_name,
        namespaces: BTreeMap::new(),
        site_metadata: BTreeMap::new(),
        folders: BTreeSet::new(),
        processed_pages: 0,
        total_wikitext_bytes: 0,
        total_diagnostics: 0,
        omitted_revisions: 0,
        progress: &mut progress,
    };
    analyzer.report_progress(0, None)?;
    analyzer.parse(file)?;
    analyzer.resolve_links_and_redirects()?;
    if analyzer.omitted_revisions > 0 {
        analyzer.import.unsupported.push(UnsupportedSourceData {
            source_path: analyzer.source_name.clone(),
            source_kind: "mediawiki_revision_history".into(),
            reason: "older page revisions were intentionally omitted".into(),
            raw_metadata: BTreeMap::from([(
                "omitted_revision_count".into(),
                serde_json::Value::from(analyzer.omitted_revisions),
            )]),
        });
        analyzer.record_diagnostic(ImportDiagnostic {
            severity: ImportDiagnosticSeverity::Warning,
            code: "mediawiki_revision_history_omitted".into(),
            message: format!(
                "{} older page revisions were omitted; only each latest revision was staged.",
                analyzer.omitted_revisions
            ),
            source_path: Some(analyzer.source_name.clone()),
            object_id: None,
        })?;
    }
    analyzer
        .import
        .objects
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .refresh_summary(analyzer.folders.len(), metadata.len());
    analyzer.import.validate()?;
    Ok(analyzer.import)
}

#[cfg(test)]
mod tests;
