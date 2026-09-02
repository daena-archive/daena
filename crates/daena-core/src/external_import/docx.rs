// Word document import engine.
use super::*;

pub(super) const MAX_DOCX_ENTRIES: usize = 4_096;
pub(super) const MAX_DOCX_DEPTH: usize = 32;
pub(super) const MAX_DOCX_XML_NODES: u32 = 200_000;
pub(super) const MAX_DOCX_MARKDOWN_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug)]
pub(super) struct DocxConversion {
    pub(super) markdown: String,
    pub(super) title: Option<String>,
    pub(super) assets: Vec<DocxAsset>,
    pub(super) warnings: Vec<DocxWarning>,
    pub(super) core_properties: Option<String>,
    pub(super) package_entry_count: usize,
    pub(super) package_entries: Vec<String>,
}

#[derive(Debug)]
pub(super) struct DocxAsset {
    pub(super) entry_path: String,
    pub(super) filename: String,
    pub(super) mime_type: &'static str,
    pub(super) bytes: Vec<u8>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct DocxWarning {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Debug, Clone)]
pub(super) struct DocxEntryPlan {
    pub(super) index: usize,
    pub(super) size: u64,
}

#[derive(Debug, Clone)]
pub(super) struct DocxRelationship {
    pub(super) target: String,
    pub(super) external: bool,
    pub(super) relationship_type: String,
}

pub(super) fn preflight_docx_package(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<BTreeMap<String, DocxEntryPlan>, CoreError> {
    if archive.len() > MAX_DOCX_ENTRIES {
        return Err(CoreError::Validation(format!(
            "DOCX package exceeds the maximum entry count of {MAX_DOCX_ENTRIES}"
        )));
    }
    let mut entries = BTreeMap::new();
    let mut folded_names = BTreeSet::new();
    let mut expanded_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| {
            CoreError::Validation(format!("invalid DOCX central-directory entry: {error}"))
        })?;
        let is_dir = entry.is_dir();
        let source_path = validate_archive_source_path(entry.name_raw(), is_dir)?;
        if entries.contains_key(&source_path) || !folded_names.insert(source_path.to_lowercase()) {
            return Err(CoreError::Validation(format!(
                "DOCX package contains a duplicate or case-colliding path: {source_path}"
            )));
        }
        if !is_dir
            && entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(CoreError::Validation(format!(
                "DOCX links and special files are not allowed: {source_path}"
            )));
        }
        let depth = source_path.split('/').count().saturating_sub(1);
        if depth > MAX_DOCX_DEPTH {
            return Err(CoreError::Validation(format!(
                "DOCX entry exceeds the maximum package depth of {MAX_DOCX_DEPTH}: {source_path}"
            )));
        }
        let size = entry.size();
        if !is_dir && size > MAX_ARCHIVE_FILE_BYTES {
            return Err(CoreError::Validation(format!(
                "DOCX entry exceeds the file-size limit: {source_path}"
            )));
        }
        expanded_bytes = expanded_bytes
            .checked_add(size)
            .ok_or_else(|| CoreError::Validation("DOCX expanded size overflowed".into()))?;
        if expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err(CoreError::Validation(
                "DOCX package exceeds the expanded-size limit".into(),
            ));
        }
        let packed = entry.compressed_size();
        if size > 0 && (packed == 0 || size > packed.saturating_mul(MAX_ARCHIVE_COMPRESSION_RATIO))
        {
            return Err(CoreError::Validation(format!(
                "DOCX entry exceeds the compression-ratio limit: {source_path}"
            )));
        }
        entries.insert(source_path, DocxEntryPlan { index, size });
    }
    Ok(entries)
}

pub(super) fn read_docx_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &BTreeMap<String, DocxEntryPlan>,
    path: &str,
) -> Result<Vec<u8>, CoreError> {
    let plan = entries.get(path).ok_or_else(|| {
        CoreError::Validation(format!("DOCX package is missing required entry: {path}"))
    })?;
    let mut entry = archive
        .by_index(plan.index)
        .map_err(|error| CoreError::Validation(format!("invalid DOCX entry '{path}': {error}")))?;
    let mut bytes = Vec::with_capacity(plan.size.min(1024 * 1024) as usize);
    entry
        .by_ref()
        .take(plan.size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CoreError::Io {
            operation: "read DOCX package entry",
            source,
        })?;
    if bytes.len() as u64 != plan.size {
        return Err(CoreError::Validation(format!(
            "DOCX entry size does not match its declaration: {path}"
        )));
    }
    Ok(bytes)
}

pub(super) fn decode_docx_xml(bytes: &[u8], label: &str) -> Result<String, CoreError> {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(bytes.to_vec()).map_err(|_| {
            CoreError::Validation(format!("DOCX XML entry is not valid UTF-8: {label}"))
        });
    }
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little_endian = bytes.starts_with(&[0xff, 0xfe]);
        let payload = &bytes[2..];
        let (pairs, remainder) = payload.as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(CoreError::Validation(format!(
                "DOCX XML entry has malformed UTF-16 data: {label}"
            )));
        }
        let units = pairs
            .iter()
            .map(|pair| {
                if little_endian {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|_| {
            CoreError::Validation(format!("DOCX XML entry is not valid UTF-16: {label}"))
        });
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| CoreError::Validation(format!("DOCX XML entry is not valid UTF-8: {label}")))
}

pub(super) fn parse_docx_xml<'a>(
    xml: &'a str,
    label: &str,
) -> Result<roxmltree::Document<'a>, CoreError> {
    roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_DOCX_XML_NODES,
        },
    )
    .map_err(|error| CoreError::Validation(format!("invalid DOCX XML in {label}: {error}")))
}

pub(super) fn docx_attribute<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.name() == local_name)
        .map(|attribute| attribute.value())
}

pub(super) fn docx_child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == local_name)
}

pub(super) fn docx_relationships(
    xml: &str,
) -> Result<BTreeMap<String, DocxRelationship>, CoreError> {
    let document = parse_docx_xml(xml, "word/_rels/document.xml.rels")?;
    let mut relationships = BTreeMap::new();
    for node in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "Relationship")
    {
        let id = docx_attribute(node, "Id")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CoreError::Validation("DOCX relationship is missing an ID".into()))?;
        let target = docx_attribute(node, "Target")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CoreError::Validation("DOCX relationship is missing a target".into()))?;
        let relationship_type = docx_attribute(node, "Type").unwrap_or_default();
        let relationship = DocxRelationship {
            target: target.into(),
            external: docx_attribute(node, "TargetMode")
                .is_some_and(|value| value.eq_ignore_ascii_case("External")),
            relationship_type: relationship_type.into(),
        };
        if relationships.insert(id.into(), relationship).is_some() {
            return Err(CoreError::Validation(format!(
                "DOCX relationship ID is duplicated: {id}"
            )));
        }
    }
    Ok(relationships)
}

pub(super) fn normalize_docx_part_target(target: &str) -> Option<String> {
    if target.is_empty()
        || target.starts_with('/')
        || target.contains('\\')
        || target.contains(':')
        || target.chars().any(char::is_control)
    {
        return None;
    }
    let mut components = vec!["word".to_owned()];
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            value => components.push(value.to_owned()),
        }
    }
    (!components.is_empty()).then(|| components.join("/"))
}

pub(super) fn docx_warn(
    warnings: &mut BTreeSet<DocxWarning>,
    code: &'static str,
    message: impl Into<String>,
) {
    warnings.insert(DocxWarning {
        code,
        message: message.into(),
    });
}

pub(super) fn validate_docx_content_types(xml: &str) -> Result<(), CoreError> {
    let document = parse_docx_xml(xml, "[Content_Types].xml")?;
    let valid = document.descendants().any(|node| {
        node.is_element()
            && node.tag_name().name() == "Override"
            && docx_attribute(node, "PartName") == Some("/word/document.xml")
            && docx_attribute(node, "ContentType").is_some_and(|value| {
                value
                    == "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            })
    });
    if !valid {
        return Err(CoreError::Validation(
            "DOCX package does not declare a standard Word document part".into(),
        ));
    }
    Ok(())
}

pub(super) fn docx_core_title(xml: &str) -> Result<Option<String>, CoreError> {
    let document = parse_docx_xml(xml, "docProps/core.xml")?;
    let title = document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "title")
        .and_then(|node| node.text())
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty());
    Ok(title)
}

pub(super) fn docx_heading_styles(xml: Option<&str>) -> Result<BTreeMap<String, usize>, CoreError> {
    let Some(xml) = xml else {
        return Ok(BTreeMap::new());
    };
    let document = parse_docx_xml(xml, "word/styles.xml")?;
    let mut styles = BTreeMap::new();
    for style in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "style")
    {
        if docx_attribute(style, "type") != Some("paragraph") {
            continue;
        }
        let Some(style_id) = docx_attribute(style, "styleId") else {
            continue;
        };
        let outline = docx_child(style, "pPr")
            .and_then(|properties| docx_child(properties, "outlineLvl"))
            .and_then(|node| docx_attribute(node, "val"))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value < 6)
            .map(|value| value + 1);
        let name = docx_child(style, "name")
            .and_then(|node| docx_attribute(node, "val"))
            .and_then(docx_heading_level);
        if let Some(level) = outline.or(name) {
            styles.insert(style_id.into(), level);
        }
    }
    Ok(styles)
}

pub(super) fn docx_heading_level(value: &str) -> Option<usize> {
    let compact = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    compact
        .strip_prefix("heading")
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .filter(|level| (1..=6).contains(level))
}

pub(super) fn docx_numbering(
    xml: Option<&str>,
) -> Result<BTreeMap<(String, usize), bool>, CoreError> {
    let Some(xml) = xml else {
        return Ok(BTreeMap::new());
    };
    let document = parse_docx_xml(xml, "word/numbering.xml")?;
    let mut abstract_levels = BTreeMap::<(String, usize), bool>::new();
    for abstract_numbering in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "abstractNum")
    {
        let Some(abstract_id) = docx_attribute(abstract_numbering, "abstractNumId") else {
            continue;
        };
        for level in abstract_numbering
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "lvl")
        {
            let index = docx_attribute(level, "ilvl")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let format = docx_child(level, "numFmt")
                .and_then(|node| docx_attribute(node, "val"))
                .unwrap_or("bullet");
            abstract_levels.insert(
                (abstract_id.into(), index),
                !matches!(format, "bullet" | "none"),
            );
        }
    }
    let mut result = BTreeMap::new();
    for numbering in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "num")
    {
        let Some(number_id) = docx_attribute(numbering, "numId") else {
            continue;
        };
        let Some(abstract_id) =
            docx_child(numbering, "abstractNumId").and_then(|node| docx_attribute(node, "val"))
        else {
            continue;
        };
        for ((candidate, level), ordered) in &abstract_levels {
            if candidate == abstract_id {
                result.insert((number_id.into(), *level), *ordered);
            }
        }
    }
    Ok(result)
}

pub(super) fn docx_escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '|' | '<' | '>' | '#' | '+' | '-' | '!'
        ) {
            escaped.push('\\');
        }
        escaped.push(match character {
            '\r' | '\n' => ' ',
            value => value,
        });
    }
    escaped
}

pub(super) fn markdown_inline_code(value: &str) -> String {
    let delimiter = "`".repeat(longest_character_run(value, '`').saturating_add(1).max(1));
    let pad = value.starts_with(['`', ' ']) || value.ends_with(['`', ' ']);
    if pad {
        format!("{delimiter} {value} {delimiter}")
    } else {
        format!("{delimiter}{value}{delimiter}")
    }
}

pub(super) fn docx_toggle(properties: roxmltree::Node<'_, '_>, name: &str) -> bool {
    docx_child(properties, name).is_some_and(|node| {
        !docx_attribute(node, "val").is_some_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "off")
        })
    })
}

pub(super) struct DocxMarkdownWriter<'a> {
    pub(super) output: String,
    pub(super) relationships: &'a BTreeMap<String, DocxRelationship>,
    pub(super) image_targets: &'a BTreeMap<String, String>,
    pub(super) heading_styles: &'a BTreeMap<String, usize>,
    pub(super) numbering: &'a BTreeMap<(String, usize), bool>,
    pub(super) warnings: BTreeSet<DocxWarning>,
}

impl<'a> DocxMarkdownWriter<'a> {
    pub(super) fn render(
        mut self,
        document: &roxmltree::Document<'_>,
    ) -> Result<(String, Vec<DocxWarning>), CoreError> {
        let body = document
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "body")
            .ok_or_else(|| CoreError::Validation("DOCX document XML has no body".into()))?;
        self.render_blocks(body)?;
        if self.output.len() > MAX_DOCX_MARKDOWN_BYTES {
            return Err(CoreError::Validation(
                "converted DOCX exceeds the Markdown output limit".into(),
            ));
        }
        Ok((
            format!("{}\n", self.output.trim()),
            self.warnings.into_iter().collect(),
        ))
    }

    pub(super) fn render_blocks(
        &mut self,
        container: roxmltree::Node<'_, '_>,
    ) -> Result<(), CoreError> {
        for child in container.children().filter(roxmltree::Node::is_element) {
            match child.tag_name().name() {
                "p" => self.render_paragraph(child),
                "tbl" => self.render_table(child),
                "sdt" => {
                    if let Some(content) = child
                        .descendants()
                        .find(|node| node.is_element() && node.tag_name().name() == "sdtContent")
                    {
                        self.render_blocks(content)?;
                    }
                }
                "altChunk" => docx_warn(
                    &mut self.warnings,
                    "docx_content_omitted",
                    "An external DOCX altChunk could not be converted and was omitted.",
                ),
                "sectPr" => {}
                name => docx_warn(
                    &mut self.warnings,
                    "docx_content_omitted",
                    format!("Unsupported DOCX body element <{name}> was omitted."),
                ),
            }
        }
        Ok(())
    }

    pub(super) fn render_paragraph(&mut self, paragraph: roxmltree::Node<'_, '_>) {
        let properties = docx_child(paragraph, "pPr");
        let style = properties
            .and_then(|node| docx_child(node, "pStyle"))
            .and_then(|node| docx_attribute(node, "val"));
        let heading = style
            .and_then(docx_heading_level)
            .or_else(|| style.and_then(|style| self.heading_styles.get(style).copied()));
        let list = properties
            .and_then(|node| docx_child(node, "numPr"))
            .and_then(|numbering| {
                let number_id =
                    docx_child(numbering, "numId").and_then(|node| docx_attribute(node, "val"))?;
                let level = docx_child(numbering, "ilvl")
                    .and_then(|node| docx_attribute(node, "val"))
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                let ordered = self
                    .numbering
                    .get(&(number_id.into(), level))
                    .copied()
                    .unwrap_or(false);
                Some((level, ordered))
            });
        let inline = self.render_inline_children(paragraph).trim().to_owned();
        if inline.is_empty() {
            self.ensure_blank_line();
            return;
        }
        if let Some(level) = heading {
            self.ensure_blank_line();
            self.output.push_str(&"#".repeat(level));
            self.output.push(' ');
            self.output.push_str(&inline);
            self.ensure_blank_line();
        } else if let Some((level, ordered)) = list {
            self.ensure_line_break();
            self.output.push_str(&"  ".repeat(level));
            self.output.push_str(if ordered { "1. " } else { "- " });
            self.output.push_str(&inline);
            self.ensure_line_break();
        } else {
            self.ensure_blank_line();
            self.output.push_str(&inline);
            self.ensure_blank_line();
        }
    }

    pub(super) fn render_table(&mut self, table: roxmltree::Node<'_, '_>) {
        self.ensure_blank_line();
        let rows = table
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "tr")
            .collect::<Vec<_>>();
        for (row_index, row) in rows.iter().enumerate() {
            self.output.push_str("| ");
            let cells = row
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "tc")
                .collect::<Vec<_>>();
            for cell in &cells {
                let value = cell
                    .children()
                    .filter(|node| node.is_element() && node.tag_name().name() == "p")
                    .map(|paragraph| self.render_inline_children(paragraph).trim().to_owned())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ");
                self.output.push_str(&value);
                self.output.push_str(" | ");
            }
            self.ensure_line_break();
            if row_index == 0 && !cells.is_empty() {
                self.output.push('|');
                for _ in &cells {
                    self.output.push_str(" --- |");
                }
                self.ensure_line_break();
            }
            if row.descendants().any(|node| {
                node.is_element() && matches!(node.tag_name().name(), "gridSpan" | "vMerge")
            }) {
                docx_warn(
                    &mut self.warnings,
                    "docx_table_simplified",
                    "A merged DOCX table cell was flattened during Markdown conversion.",
                );
            }
        }
        self.ensure_blank_line();
    }

    pub(super) fn render_inline_children(&mut self, node: roxmltree::Node<'_, '_>) -> String {
        let mut output = String::new();
        for child in node.children().filter(roxmltree::Node::is_element) {
            match child.tag_name().name() {
                "pPr" | "bookmarkStart" | "bookmarkEnd" | "proofErr" => {}
                "r" => output.push_str(&self.render_run(child)),
                "hyperlink" => output.push_str(&self.render_hyperlink(child)),
                "smartTag" | "sdt" | "ins" | "moveTo" | "fldSimple" => {
                    if child.tag_name().name() == "fldSimple" {
                        docx_warn(
                            &mut self.warnings,
                            "docx_field_simplified",
                            "A DOCX field was reduced to its displayed text.",
                        );
                    }
                    output.push_str(&self.render_inline_children(child));
                }
                "del" | "moveFrom" => docx_warn(
                    &mut self.warnings,
                    "docx_revision_omitted",
                    "Deleted or moved-from revision text was omitted.",
                ),
                _ => output.push_str(&self.render_inline_children(child)),
            }
        }
        output
    }

    pub(super) fn render_hyperlink(&mut self, hyperlink: roxmltree::Node<'_, '_>) -> String {
        let label = self.render_inline_children(hyperlink);
        let target = docx_attribute(hyperlink, "anchor")
            .map(|anchor| format!("#{anchor}"))
            .or_else(|| {
                let id = docx_attribute(hyperlink, "id")?;
                let relationship = self.relationships.get(id)?;
                relationship
                    .relationship_type
                    .ends_with("/hyperlink")
                    .then(|| relationship.target.clone())
            });
        let Some(target) = target else {
            return label;
        };
        let Some(target) = safe_html_target(&target) else {
            docx_warn(
                &mut self.warnings,
                "docx_unsafe_target_removed",
                "Removed an unsafe DOCX hyperlink target.",
            );
            return label;
        };
        let label = if label.trim().is_empty() {
            docx_escape_text(target)
        } else {
            label
        };
        format!("[{label}]({})", markdown_destination(target))
    }

    pub(super) fn render_run(&mut self, run: roxmltree::Node<'_, '_>) -> String {
        let properties = docx_child(run, "rPr");
        let run_style = properties
            .and_then(|properties| docx_child(properties, "rStyle"))
            .and_then(|node| docx_attribute(node, "val"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let code_style = matches!(run_style.as_str(), "code" | "verbatim" | "htmlcode");
        let mut content = String::new();
        for child in run.children().filter(roxmltree::Node::is_element) {
            match child.tag_name().name() {
                "rPr" => {}
                "t" | "delText" => {
                    if child.tag_name().name() == "t" {
                        let text = child.text().unwrap_or_default();
                        if code_style {
                            content.push_str(&text.replace(['\r', '\n'], " "));
                        } else {
                            content.push_str(&docx_escape_text(text));
                        }
                    }
                }
                "tab" => content.push('\t'),
                "br" | "cr" => content.push_str("  \n"),
                "noBreakHyphen" => content.push('-'),
                "softHyphen" => content.push('\u{00ad}'),
                "drawing" | "pict" | "object" => {
                    content.push_str(&self.render_drawing(child));
                }
                "footnoteReference" | "endnoteReference" => docx_warn(
                    &mut self.warnings,
                    "docx_note_omitted",
                    "A DOCX footnote or endnote reference was omitted.",
                ),
                "instrText" => docx_warn(
                    &mut self.warnings,
                    "docx_field_simplified",
                    "A DOCX field instruction was omitted while retaining displayed text.",
                ),
                "sym" => docx_warn(
                    &mut self.warnings,
                    "docx_symbol_omitted",
                    "A symbol-font DOCX character could not be converted reliably.",
                ),
                _ => {}
            }
        }
        if content.is_empty() {
            return content;
        }
        let Some(properties) = properties else {
            return content;
        };
        if code_style {
            content = markdown_inline_code(content.trim());
        }
        if docx_toggle(properties, "strike") || docx_toggle(properties, "dstrike") {
            content = format!("~~{content}~~");
        }
        if docx_toggle(properties, "i") {
            content = format!("*{content}*");
        }
        if docx_toggle(properties, "b") {
            content = format!("**{content}**");
        }
        content
    }

    pub(super) fn render_drawing(&mut self, drawing: roxmltree::Node<'_, '_>) -> String {
        let alt = drawing
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "docPr")
            .and_then(|node| {
                docx_attribute(node, "descr")
                    .or_else(|| docx_attribute(node, "title"))
                    .or_else(|| docx_attribute(node, "name"))
            })
            .unwrap_or("Image");
        let relationship_id = drawing
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "blip")
            .and_then(|node| {
                docx_attribute(node, "embed").or_else(|| docx_attribute(node, "link"))
            });
        let Some(target) = relationship_id.and_then(|id| self.image_targets.get(id)) else {
            docx_warn(
                &mut self.warnings,
                "docx_image_omitted",
                "A DOCX drawing had no safe, supported image payload and was omitted.",
            );
            return String::new();
        };
        format!(
            "![{}]({})",
            docx_escape_text(alt),
            markdown_destination(target)
        )
    }

    pub(super) fn ensure_line_break(&mut self) {
        while self.output.ends_with(' ') {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    pub(super) fn ensure_blank_line(&mut self) {
        self.ensure_line_break();
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }
}

pub(super) fn convert_docx_to_markdown(
    bytes: &[u8],
    source_path: &str,
) -> Result<DocxConversion, CoreError> {
    if bytes.len() as u64 > MAX_ARCHIVE_COMPRESSED_BYTES {
        return Err(CoreError::Validation(
            "DOCX package exceeds the compressed-size limit".into(),
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| CoreError::Validation(format!("invalid DOCX package: {error}")))?;
    let entries = preflight_docx_package(&mut archive)?;
    let content_types_bytes = read_docx_entry(&mut archive, &entries, "[Content_Types].xml")?;
    let content_types = decode_docx_xml(&content_types_bytes, "[Content_Types].xml")?;
    validate_docx_content_types(&content_types)?;
    let document_bytes = read_docx_entry(&mut archive, &entries, "word/document.xml")?;
    let document_xml = decode_docx_xml(&document_bytes, "word/document.xml")?;
    let document = parse_docx_xml(&document_xml, "word/document.xml")?;

    let relationships_xml = if entries.contains_key("word/_rels/document.xml.rels") {
        let bytes = read_docx_entry(&mut archive, &entries, "word/_rels/document.xml.rels")?;
        Some(decode_docx_xml(&bytes, "word/_rels/document.xml.rels")?)
    } else {
        None
    };
    let relationships = relationships_xml
        .as_deref()
        .map(docx_relationships)
        .transpose()?
        .unwrap_or_default();
    let styles_xml = if entries.contains_key("word/styles.xml") {
        let bytes = read_docx_entry(&mut archive, &entries, "word/styles.xml")?;
        Some(decode_docx_xml(&bytes, "word/styles.xml")?)
    } else {
        None
    };
    let numbering_xml = if entries.contains_key("word/numbering.xml") {
        let bytes = read_docx_entry(&mut archive, &entries, "word/numbering.xml")?;
        Some(decode_docx_xml(&bytes, "word/numbering.xml")?)
    } else {
        None
    };
    let heading_styles = docx_heading_styles(styles_xml.as_deref())?;
    let numbering = docx_numbering(numbering_xml.as_deref())?;

    let core_properties = if entries.contains_key("docProps/core.xml") {
        let bytes = read_docx_entry(&mut archive, &entries, "docProps/core.xml")?;
        Some(decode_docx_xml(&bytes, "docProps/core.xml")?)
    } else {
        None
    };
    let mut warnings = BTreeSet::new();
    let title = match core_properties
        .as_deref()
        .map(docx_core_title)
        .transpose()?
    {
        Some(Some(title)) if title.chars().count() <= 512 => Some(title),
        Some(Some(_)) => {
            docx_warn(
                &mut warnings,
                "docx_title_ignored",
                "Ignored a DOCX title longer than the 512-character import limit.",
            );
            None
        }
        _ => None,
    };

    for (path, code, message) in [
        (
            "word/comments.xml",
            "docx_comments_omitted",
            "DOCX comments are not converted in this import iteration.",
        ),
        (
            "word/footnotes.xml",
            "docx_notes_omitted",
            "DOCX footnote bodies are not converted in this import iteration.",
        ),
        (
            "word/endnotes.xml",
            "docx_notes_omitted",
            "DOCX endnote bodies are not converted in this import iteration.",
        ),
    ] {
        if entries.contains_key(path) {
            docx_warn(&mut warnings, code, message);
        }
    }
    if entries
        .keys()
        .any(|path| path.starts_with("word/header") || path.starts_with("word/footer"))
    {
        docx_warn(
            &mut warnings,
            "docx_headers_omitted",
            "DOCX headers and footers are not converted in this import iteration.",
        );
    }
    if entries.keys().any(|path| {
        path.starts_with("word/embeddings/")
            || path.starts_with("word/activeX/")
            || path.ends_with("vbaProject.bin")
    }) {
        docx_warn(
            &mut warnings,
            "docx_active_content_removed",
            "Embedded objects or active DOCX package content were not imported.",
        );
    }
    if entries.keys().any(|path| {
        path.starts_with("customXml/")
            || path.starts_with("word/glossary/")
            || path.starts_with("word/charts/")
            || path.starts_with("word/diagrams/")
    }) {
        docx_warn(
            &mut warnings,
            "docx_package_content_unconverted",
            "Additional DOCX package parts are listed in staged raw metadata but were not converted.",
        );
    }

    let container_name = source_path.rsplit('/').next().unwrap_or(source_path);
    let mut image_targets = BTreeMap::new();
    let mut assets_by_entry = BTreeMap::<String, DocxAsset>::new();
    for (relationship_id, relationship) in &relationships {
        if !relationship.relationship_type.ends_with("/image") {
            continue;
        }
        if relationship.external {
            let target = relationship.target.trim();
            let lower = target.to_ascii_lowercase();
            if (lower.starts_with("http://")
                || lower.starts_with("https://")
                || target.starts_with("//"))
                && safe_html_target(target).is_some()
            {
                image_targets.insert(relationship_id.clone(), target.into());
            } else {
                docx_warn(
                    &mut warnings,
                    "docx_unsafe_target_removed",
                    "Removed an unsafe external DOCX image target.",
                );
            }
            continue;
        }
        let Some(entry_path) = normalize_docx_part_target(&relationship.target) else {
            docx_warn(
                &mut warnings,
                "docx_unsafe_target_removed",
                "Removed a DOCX image relationship that escaped the package.",
            );
            continue;
        };
        let Some(mime_type) =
            asset_mime_type(&entry_path).filter(|value| value.starts_with("image/"))
        else {
            docx_warn(
                &mut warnings,
                "docx_image_omitted",
                format!("Unsupported DOCX image format was omitted: {entry_path}"),
            );
            continue;
        };
        if !entries.contains_key(&entry_path) {
            docx_warn(
                &mut warnings,
                "docx_image_missing",
                format!("DOCX image relationship target is missing: {entry_path}"),
            );
            continue;
        }
        if !assets_by_entry.contains_key(&entry_path) {
            let image_bytes = read_docx_entry(&mut archive, &entries, &entry_path)?;
            if !asset_signature_matches(mime_type, &image_bytes) {
                docx_warn(
                    &mut warnings,
                    "docx_image_invalid",
                    format!("DOCX image bytes do not match their format: {entry_path}"),
                );
                continue;
            }
            let filename = entry_path.rsplit('/').next().unwrap_or("image").to_owned();
            assets_by_entry.insert(
                entry_path.clone(),
                DocxAsset {
                    entry_path: entry_path.clone(),
                    filename,
                    mime_type,
                    bytes: image_bytes,
                },
            );
        }
        image_targets.insert(
            relationship_id.clone(),
            format!("{container_name}!/{entry_path}"),
        );
    }

    let (markdown, render_warnings) = DocxMarkdownWriter {
        output: String::new(),
        relationships: &relationships,
        image_targets: &image_targets,
        heading_styles: &heading_styles,
        numbering: &numbering,
        warnings,
    }
    .render(&document)?;

    Ok(DocxConversion {
        markdown,
        title,
        assets: assets_by_entry.into_values().collect(),
        warnings: render_warnings,
        core_properties,
        package_entry_count: entries.len(),
        package_entries: entries.keys().cloned().collect(),
    })
}
