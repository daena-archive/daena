// Markdown and Wiki export helpers.
use daena_plugin_api::PluginManifest;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn markdown_export_stem(value: &str) -> String {
    let mut stem = value
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    stem = stem.trim().trim_matches('.').to_string();
    if stem.is_empty() || stem == "." || stem == ".." {
        stem = "Untitled".into();
    }
    stem.chars().take(120).collect()
}

pub(super) fn markdown_export_target(filename: &str) -> String {
    let mut target = String::new();
    for byte in filename.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            target.push(byte as char);
        } else if byte == b' ' {
            target.push_str("%20");
        } else {
            target.push_str(&format!("%{byte:02X}"));
        }
    }
    target
}

pub(super) fn markdown_escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

pub(super) fn rewrite_markdown_entity_links(
    body: &str,
    filenames: &BTreeMap<String, String>,
) -> String {
    let mut output = String::with_capacity(body.len());
    let mut in_fence = false;
    for (line_index, line) in body.split('\n').enumerate() {
        if line_index > 0 {
            output.push('\n');
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            output.push_str(line);
            continue;
        }
        if in_fence {
            output.push_str(line);
            continue;
        }

        let mut cursor = 0;
        while cursor < line.len() {
            if line[cursor..].starts_with('`') {
                if let Some(end) = line[cursor + 1..].find('`') {
                    let end = cursor + end + 2;
                    output.push_str(&line[cursor..end]);
                    cursor = end;
                    continue;
                }
            }
            if line[cursor..].starts_with("[[") {
                if let Some(label_end) = line[cursor + 2..].find("]](") {
                    let label_start = cursor + 2;
                    let label_end = label_start + label_end;
                    let id_start = label_end + 3;
                    if let Some(id_end) = line[id_start..].find(')') {
                        let id_end = id_start + id_end;
                        let entity_id = &line[id_start..id_end];
                        if let Some(filename) = filenames.get(entity_id) {
                            let label = &line[label_start..label_end];
                            output.push('[');
                            output.push_str(&markdown_escape_label(label));
                            output.push_str("](");
                            output.push_str(&markdown_export_target(filename));
                            output.push(')');
                            cursor = id_end + 1;
                            continue;
                        }
                    }
                }
            }
            if line[cursor..].starts_with('[') && !line[cursor..].starts_with("[[") {
                if let Some(label_end) = line[cursor + 1..].find("](") {
                    let label_start = cursor + 1;
                    let label_end = label_start + label_end;
                    let target_start = label_end + 2;
                    if let Some(target_end) = line[target_start..].find(')') {
                        let target_end = target_start + target_end;
                        if let Some(entity_id) =
                            line[target_start..target_end].strip_prefix("daena://entity/")
                        {
                            if let Some(filename) = filenames.get(entity_id) {
                                let label = &line[label_start..label_end];
                                output.push('[');
                                output.push_str(&markdown_escape_label(label));
                                output.push_str("](");
                                output.push_str(&markdown_export_target(filename));
                                output.push(')');
                                cursor = target_end + 1;
                                continue;
                            }
                        }
                    }
                }
            }
            if let Some(next) = line[cursor..].chars().next() {
                output.push(next);
                cursor += next.len_utf8();
            } else {
                break;
            }
        }
    }
    output
}

pub(super) fn markdown_relationship_heading(value: &str) -> String {
    let label = value.replace(['_', '-'], " ");
    let mut characters = label.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => "Relationship".into(),
    }
}

pub(super) fn rewrite_markdown_entity_links_as_labels(body: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut in_fence = false;
    for (line_index, line) in body.split('\n').enumerate() {
        if line_index > 0 {
            output.push('\n');
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            output.push_str(line);
            continue;
        }
        if in_fence {
            output.push_str(line);
            continue;
        }

        let mut cursor = 0;
        while cursor < line.len() {
            if line[cursor..].starts_with('`') {
                if let Some(end) = line[cursor + 1..].find('`') {
                    let end = cursor + end + 2;
                    output.push_str(&line[cursor..end]);
                    cursor = end;
                    continue;
                }
            }
            if line[cursor..].starts_with("[[") {
                if let Some(label_end) = line[cursor + 2..].find("]](") {
                    let label_start = cursor + 2;
                    let label_end = label_start + label_end;
                    if let Some(id_end) = line[label_end + 3..].find(')') {
                        output.push_str(&line[label_start..label_end]);
                        cursor = label_end + 3 + id_end + 1;
                        continue;
                    }
                }
            }
            if line[cursor..].starts_with('[') && !line[cursor..].starts_with("[[") {
                if let Some(label_end) = line[cursor + 1..].find("](") {
                    let label_start = cursor + 1;
                    let label_end = label_start + label_end;
                    let target_start = label_end + 2;
                    if let Some(target_end) = line[target_start..].find(')') {
                        let target_end = target_start + target_end;
                        if line[target_start..target_end].starts_with("daena://entity/") {
                            output.push_str(&line[label_start..label_end]);
                            cursor = target_end + 1;
                            continue;
                        }
                    }
                }
            }
            if let Some(next) = line[cursor..].chars().next() {
                output.push(next);
                cursor += next.len_utf8();
            } else {
                break;
            }
        }
    }
    output
}

pub(super) fn wiki_display_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => {
            if *value {
                "Yes".into()
            } else {
                "No".into()
            }
        }
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(values) => values
            .iter()
            .map(wiki_display_value)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        serde_json::Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

pub(super) fn wiki_entity_type_label(
    manifest: &PluginManifest,
    entity_type: Option<&str>,
) -> String {
    let Some(entity_type) = entity_type else {
        return "Uncategorized".into();
    };
    manifest
        .templates
        .iter()
        .find(|template| template.entity_type == entity_type)
        .map(|template| template.name.clone())
        .unwrap_or_else(|| markdown_relationship_heading(entity_type))
}

pub(super) fn wiki_field_label(
    manifest: &PluginManifest,
    namespace: &str,
    key: &str,
    entity_type: Option<&str>,
) -> Option<String> {
    manifest
        .schemas
        .iter()
        .find(|schema| schema.namespace == namespace)
        .and_then(|schema| schema.fields.iter().find(|field| field.key == key))
        .filter(|field| field.field_type != "relationship")
        .filter(|field| {
            field.entity_types.as_ref().is_none_or(|entity_types| {
                entity_types.is_empty()
                    || entity_type.is_some_and(|entity_type| {
                        entity_types
                            .iter()
                            .any(|candidate| candidate == entity_type)
                    })
            })
        })
        .map(|field| field.label.clone())
}

pub(super) fn wiki_relationship_label(
    manifest: &PluginManifest,
    relationship_type: &str,
) -> String {
    manifest
        .schemas
        .iter()
        .flat_map(|schema| schema.fields.iter())
        .find(|field| field.relationship_type.as_deref() == Some(relationship_type))
        .map(|field| field.label.clone())
        .unwrap_or_else(|| markdown_relationship_heading(relationship_type))
}

pub(super) fn markdown_to_safe_html(markdown: &str) -> String {
    let mut output = String::new();
    for event in Parser::new_ext(markdown, Options::all()) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => output.push_str("<p>"),
                Tag::Heading { level, .. } => output.push_str(&format!("<{level}>")),
                Tag::BlockQuote(_) => output.push_str("<blockquote>"),
                Tag::CodeBlock(kind) => {
                    output.push_str("<pre><code");
                    if let CodeBlockKind::Fenced(language) = kind {
                        if !language.is_empty() {
                            output.push_str(" class=\"language-");
                            output.push_str(&html_escape(&language));
                            output.push('"');
                        }
                    }
                    output.push('>');
                }
                Tag::HtmlBlock | Tag::MetadataBlock(_) => {}
                Tag::List(Some(start)) => output.push_str(&format!("<ol start=\"{start}\">")),
                Tag::List(None) => output.push_str("<ul>"),
                Tag::Item => output.push_str("<li>"),
                Tag::FootnoteDefinition(label) => {
                    output.push_str(&format!("<aside id=\"footnote-{}\">", html_escape(&label)))
                }
                Tag::DefinitionList => output.push_str("<dl>"),
                Tag::DefinitionListTitle => output.push_str("<dt>"),
                Tag::DefinitionListDefinition => output.push_str("<dd>"),
                Tag::Table(_) => output.push_str("<table>"),
                Tag::TableHead => output.push_str("<thead>"),
                Tag::TableRow => output.push_str("<tr>"),
                Tag::TableCell => output.push_str("<td>"),
                Tag::Emphasis => output.push_str("<em>"),
                Tag::Strong => output.push_str("<strong>"),
                Tag::Strikethrough => output.push_str("<del>"),
                Tag::Link {
                    dest_url, title, ..
                } => {
                    let destination = dest_url.trim();
                    let destination = if destination.to_ascii_lowercase().starts_with("javascript:")
                    {
                        "#"
                    } else {
                        destination
                    };
                    output.push_str("<a href=\"");
                    output.push_str(&html_escape(destination));
                    if !title.is_empty() {
                        output.push_str("\" title=\"");
                        output.push_str(&html_escape(&title));
                    }
                    output.push_str("\">");
                }
                Tag::Image { .. } => output.push_str("<span class=\"image-alt\">"),
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => output.push_str("</p>"),
                TagEnd::Heading(level) => output.push_str(&format!("</{level}>")),
                TagEnd::BlockQuote(_) => output.push_str("</blockquote>"),
                TagEnd::CodeBlock => output.push_str("</code></pre>"),
                TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => {}
                TagEnd::List(true) => output.push_str("</ol>"),
                TagEnd::List(false) => output.push_str("</ul>"),
                TagEnd::Item => output.push_str("</li>"),
                TagEnd::FootnoteDefinition => output.push_str("</aside>"),
                TagEnd::DefinitionList => output.push_str("</dl>"),
                TagEnd::DefinitionListTitle => output.push_str("</dt>"),
                TagEnd::DefinitionListDefinition => output.push_str("</dd>"),
                TagEnd::Table => output.push_str("</table>"),
                TagEnd::TableHead => output.push_str("</thead><tbody>"),
                TagEnd::TableRow => output.push_str("</tr>"),
                TagEnd::TableCell => output.push_str("</td>"),
                TagEnd::Emphasis => output.push_str("</em>"),
                TagEnd::Strong => output.push_str("</strong>"),
                TagEnd::Strikethrough => output.push_str("</del>"),
                TagEnd::Link => output.push_str("</a>"),
                TagEnd::Image => output.push_str("</span>"),
            },
            Event::Text(value) => output.push_str(&html_escape(&value)),
            Event::Code(value) => {
                output.push_str("<code>");
                output.push_str(&html_escape(&value));
                output.push_str("</code>");
            }
            Event::InlineMath(value) => {
                output.push_str("<code class=\"math\">");
                output.push_str(&html_escape(&value));
                output.push_str("</code>");
            }
            Event::DisplayMath(value) => {
                output.push_str("<pre class=\"math\"><code>");
                output.push_str(&html_escape(&value));
                output.push_str("</code></pre>");
            }
            Event::Html(value) | Event::InlineHtml(value) => output.push_str(&html_escape(&value)),
            Event::FootnoteReference(label) => output.push_str(&format!(
                "<sup><a href=\"#footnote-{}\">{}</a></sup>",
                html_escape(&label),
                html_escape(&label)
            )),
            Event::SoftBreak => output.push('\n'),
            Event::HardBreak => output.push_str("<br>\n"),
            Event::Rule => output.push_str("<hr>"),
            Event::TaskListMarker(checked) => output.push_str(if checked {
                "<input type=\"checkbox\" checked disabled>"
            } else {
                "<input type=\"checkbox\" disabled>"
            }),
        }
    }
    if output.contains("<tbody>") && !output.contains("</tbody>") {
        output = output.replace("</table>", "</tbody></table>");
    }
    output
}

pub(super) fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn wiki_export_target(destination: &Path, stem: &str, extension: &str) -> PathBuf {
    let mut target = destination.join(format!("{stem}.{extension}"));
    let mut suffix = 2;
    while target.exists() {
        target = destination.join(format!("{stem}-{suffix}.{extension}"));
        suffix += 1;
    }
    target
}
