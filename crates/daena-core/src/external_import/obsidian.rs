// Obsidian vault helpers and shared import validators.
use super::*;

#[derive(Debug, Default)]
pub(super) struct ObsidianFrontmatter {
    pub(super) fields: BTreeMap<String, serde_json::Value>,
    pub(super) aliases: Vec<String>,
    pub(super) tags: Vec<String>,
    pub(super) entity_type_hint: Option<String>,
    pub(super) warnings: Vec<String>,
}

pub(super) fn parse_obsidian_frontmatter(frontmatter: &str) -> ObsidianFrontmatter {
    let lines = frontmatter.lines().collect::<Vec<_>>();
    let mut parsed = ObsidianFrontmatter::default();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            parsed
                .warnings
                .push("Ignored an unattached indented YAML frontmatter line.".into());
            index += 1;
            continue;
        }
        let Some((key, remainder)) = line.split_once(':') else {
            parsed
                .warnings
                .push("Ignored a YAML frontmatter line without a key/value separator.".into());
            index += 1;
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key.chars().all(|character| {
                character.is_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            parsed
                .warnings
                .push(format!("Ignored unsupported YAML frontmatter key: {key}"));
            index += 1;
            continue;
        }
        let remainder = remainder.trim();
        let mut consumed_until = index + 1;
        let value = if remainder.is_empty() || matches!(remainder, "|" | ">") {
            let mut block = Vec::new();
            while consumed_until < lines.len()
                && (lines[consumed_until].starts_with(char::is_whitespace)
                    || lines[consumed_until].trim().is_empty())
            {
                block.push(lines[consumed_until]);
                consumed_until += 1;
            }
            let non_empty = block
                .iter()
                .filter(|line| !line.trim().is_empty())
                .collect::<Vec<_>>();
            if !non_empty.is_empty()
                && non_empty
                    .iter()
                    .all(|line| line.trim_start().starts_with("- "))
            {
                serde_json::Value::Array(
                    non_empty
                        .into_iter()
                        .map(|line| parse_obsidian_yaml_scalar(line.trim_start()[2..].trim()))
                        .collect(),
                )
            } else if matches!(remainder, "|" | ">") {
                let separator = if remainder == ">" { " " } else { "\n" };
                serde_json::Value::String(
                    block
                        .into_iter()
                        .map(|line| line.trim_start())
                        .collect::<Vec<_>>()
                        .join(separator),
                )
            } else if block.is_empty() {
                serde_json::Value::Null
            } else {
                parsed.warnings.push(format!(
                    "Preserved unsupported nested YAML for '{key}' as text."
                ));
                serde_json::Value::String(
                    block
                        .into_iter()
                        .map(str::trim_end)
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            }
        } else {
            parse_obsidian_yaml_value(remainder, &mut parsed.warnings)
        };
        if parsed.fields.insert(key.into(), value).is_some() {
            parsed.warnings.push(format!(
                "A duplicate YAML frontmatter key was replaced: {key}"
            ));
        }
        index = consumed_until;
    }

    parsed.aliases = ["aliases", "alias"]
        .into_iter()
        .filter_map(|key| parsed.fields.get(key))
        .flat_map(obsidian_string_values)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    parsed.tags = parsed
        .fields
        .get("tags")
        .or_else(|| parsed.fields.get("tag"))
        .into_iter()
        .flat_map(obsidian_string_values)
        .flat_map(|value| {
            value
                .split([',', ' '])
                .map(|tag| tag.trim().trim_start_matches('#').to_owned())
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    parsed.entity_type_hint = parsed
        .fields
        .get("type")
        .or_else(|| parsed.fields.get("entity_type"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    parsed.warnings.sort();
    parsed.warnings.dedup();
    parsed
}

pub(super) fn parse_obsidian_yaml_value(
    value: &str,
    warnings: &mut Vec<String>,
) -> serde_json::Value {
    let value = value.trim();
    if value.starts_with('[') && value.ends_with(']') {
        return serde_json::Value::Array(
            split_obsidian_inline_list(&value[1..value.len() - 1])
                .into_iter()
                .map(|value| parse_obsidian_yaml_scalar(&value))
                .collect(),
        );
    }
    if value.starts_with('{') && value.ends_with('}') {
        if let Ok(value) = serde_json::from_str(value) {
            return value;
        }
        warnings.push("Preserved a non-JSON inline YAML mapping as text.".into());
    }
    parse_obsidian_yaml_scalar(value)
}

pub(super) fn parse_obsidian_yaml_scalar(value: &str) -> serde_json::Value {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return serde_json::from_str(value)
            .unwrap_or_else(|_| serde_json::Value::String(value[1..value.len() - 1].into()));
    }
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        return serde_json::Value::String(value[1..value.len() - 1].replace("''", "'"));
    }
    match value.to_ascii_lowercase().as_str() {
        "true" => return serde_json::Value::Bool(true),
        "false" => return serde_json::Value::Bool(false),
        "null" | "~" => return serde_json::Value::Null,
        _ => {}
    }
    if let Ok(integer) = value.parse::<i64>() {
        return serde_json::Value::from(integer);
    }
    if let Ok(float) = value.parse::<f64>() {
        if float.is_finite() {
            return serde_json::Value::from(float);
        }
    }
    serde_json::Value::String(value.into())
}

pub(super) fn split_obsidian_inline_list(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quote == Some('"') {
            current.push(character);
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            current.push(character);
            continue;
        }
        if character == ',' && quote.is_none() {
            values.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(character);
        }
    }
    if !current.trim().is_empty() {
        values.push(current.trim().to_owned());
    }
    values
}

pub(super) fn obsidian_string_values(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|value| match value {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
            .collect(),
        serde_json::Value::String(value) => vec![value.clone()],
        serde_json::Value::Number(value) => vec![value.to_string()],
        _ => Vec::new(),
    }
}

pub(super) fn markdown_frontmatter(body: &str) -> Option<&str> {
    let remainder = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))?;
    let mut offset = 0;
    for line in remainder.split_inclusive('\n') {
        let value = line.trim_end_matches(['\r', '\n']);
        if value == "---" || value == "..." {
            return Some(&remainder[..offset]);
        }
        offset += line.len();
    }
    None
}

pub(super) fn markdown_body_after_frontmatter(body: &str) -> &str {
    let Some(remainder) = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))
    else {
        return body;
    };
    let mut offset = 0;
    for line in remainder.split_inclusive('\n') {
        let value = line.trim_end_matches(['\r', '\n']);
        offset += line.len();
        if value == "---" || value == "..." {
            return &remainder[offset..];
        }
    }
    body
}

pub(super) fn asset_mime_type(source_path: &str) -> Option<&'static str> {
    let extension = Path::new(source_path)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

pub(super) fn asset_signature_matches(mime_type: &str, bytes: &[u8]) -> bool {
    match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        _ => false,
    }
}

pub(super) fn is_external_markdown_target(target: &str) -> bool {
    let target = target.trim();
    if target.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic())
        && characters.all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '-' | '.'))
}

pub(super) fn resolve_relative_source_path(source_path: &str, target: &str) -> Option<String> {
    let path = target
        .split(['?', '#'])
        .next()
        .map(str::trim)
        .unwrap_or_default();
    if path.is_empty() {
        return Some(source_path.to_owned());
    }
    let decoded = percent_decode_utf8(path)?;
    if decoded.starts_with('/') || decoded.contains('\\') || decoded.chars().any(char::is_control) {
        return None;
    }
    let mut components = source_path
        .rsplit_once('/')
        .map(|(parent, _)| parent.split('/').map(str::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();
    for component in decoded.split('/') {
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

pub(super) fn percent_decode_utf8(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push(hex_nibble(high)? << 4 | hex_nibble(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

pub(super) fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub(super) fn validate_limits(limits: &GenericDocumentImportLimits) -> Result<(), CoreError> {
    if limits.max_entries == 0
        || limits.max_files == 0
        || limits.max_file_bytes == 0
        || limits.max_total_bytes == 0
        || limits.max_diagnostics == 0
    {
        return Err(CoreError::Validation(
            "import analysis limits must be greater than zero".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_diagnostics(diagnostics: &[ImportDiagnostic]) -> Result<(), CoreError> {
    for diagnostic in diagnostics {
        if diagnostic.code.trim().is_empty() || diagnostic.message.trim().is_empty() {
            return Err(CoreError::Validation(
                "staged import diagnostics require a code and message".into(),
            ));
        }
        if let Some(source_path) = &diagnostic.source_path {
            validate_source_path(source_path)?;
        }
    }
    Ok(())
}

pub(super) fn validate_non_empty_unique_values(
    label: &str,
    values: &[String],
) -> Result<(), CoreError> {
    let mut unique = BTreeSet::new();
    if values
        .iter()
        .any(|value| value.trim().is_empty() || !unique.insert(value.as_str()))
    {
        return Err(CoreError::Validation(format!(
            "staged import {label} values must be non-empty and unique"
        )));
    }
    Ok(())
}

pub(super) fn validate_source_path(path: &str) -> Result<(), CoreError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return Err(CoreError::Validation(
            "staged import source paths must be non-empty portable relative paths".into(),
        ));
    }
    let components = path.split('/').collect::<Vec<_>>();
    let windows_prefix = components.first().is_some_and(|component| {
        let bytes = component.as_bytes();
        bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    });
    if windows_prefix
        || components
            .iter()
            .any(|component| component.is_empty() || *component == "." || *component == "..")
    {
        return Err(CoreError::Validation(
            "staged import source paths must be normalized relative paths".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_portable_basename(name: &str) -> Result<(), CoreError> {
    let trimmed = name.trim();
    let bytes = trimmed.as_bytes();
    let windows_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || windows_prefix
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control)
    {
        return Err(CoreError::Validation(
            "staged import asset filename must be a portable basename".into(),
        ));
    }
    Ok(())
}

pub(super) fn document_format(source_path: &str) -> Option<&'static str> {
    let extension = Path::new(source_path)
        .extension()
        .and_then(|extension| extension.to_str())?;
    if extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown") {
        Some("markdown")
    } else if extension.eq_ignore_ascii_case("txt") {
        Some("plain_text")
    } else if extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm") {
        Some("html")
    } else if extension.eq_ignore_ascii_case("docx") {
        Some("docx")
    } else {
        None
    }
}

pub(super) fn document_title(source_path: &str) -> String {
    Path::new(source_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Untitled document")
        .to_owned()
}

pub(super) fn non_utf8_entry_label(relative_parts: &[String]) -> String {
    if relative_parts.is_empty() {
        "[non-utf8 entry]".into()
    } else {
        format!("{}/[non-utf8 entry]", relative_parts.join("/"))
    }
}

pub(super) fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}
