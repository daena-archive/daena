// HTML import engine.
use super::*;
use dom_query::{Document, NodeRef};

pub(super) const MAX_HTML_DOM_NODES: usize = 100_000;
pub(super) const MAX_HTML_DOM_DEPTH: usize = 128;
pub(super) const MAX_HTML_MARKDOWN_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug)]
pub(super) struct HtmlConversion {
    pub(super) markdown: String,
    pub(super) title: Option<String>,
    pub(super) warnings: Vec<HtmlConversionWarning>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct HtmlConversionWarning {
    pub(super) code: &'static str,
    pub(super) message: String,
}

pub(super) struct HtmlMarkdownWriter {
    pub(super) output: String,
    pub(super) warnings: BTreeSet<HtmlConversionWarning>,
    pub(super) visited_nodes: usize,
    pub(super) pending_space: bool,
}

impl HtmlMarkdownWriter {
    pub(super) fn render(mut self, document: &Document) -> Result<HtmlConversion, CoreError> {
        for child in document.root().children() {
            self.render_node(child, 0, 0, false, false)?;
        }
        if self.output.len() > MAX_HTML_MARKDOWN_BYTES {
            return Err(CoreError::Validation(
                "converted HTML exceeds the Markdown output limit".into(),
            ));
        }
        let markdown = self.output.trim().to_owned() + "\n";
        let title = document
            .try_select("title")
            .map(|selection| {
                selection
                    .text()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|title| !title.is_empty());
        let title = if title
            .as_deref()
            .is_some_and(|title| title.chars().count() > 512)
        {
            self.warn(
                "html_title_ignored",
                "Ignored an HTML title longer than the 512-character import limit.",
            );
            None
        } else {
            title
        };
        if !document.errors.borrow().is_empty() {
            self.warn(
                "html_parser_recovered",
                format!(
                    "The HTML5 parser recovered from {} malformed construct(s).",
                    document.errors.borrow().len()
                ),
            );
        }
        Ok(HtmlConversion {
            markdown,
            title,
            warnings: self.warnings.into_iter().collect(),
        })
    }

    pub(super) fn render_node(
        &mut self,
        node: NodeRef<'_>,
        depth: usize,
        list_depth: usize,
        ordered_list: bool,
        preformatted: bool,
    ) -> Result<(), CoreError> {
        self.visited_nodes = self.visited_nodes.saturating_add(1);
        if self.visited_nodes > MAX_HTML_DOM_NODES || depth > MAX_HTML_DOM_DEPTH {
            return Err(CoreError::Validation(
                "HTML document exceeds the DOM complexity limit".into(),
            ));
        }
        if node.is_text() {
            let text = node.immediate_text();
            if preformatted {
                self.output.push_str(&text);
            } else {
                self.push_normalized_text(&text);
            }
            return Ok(());
        }
        let Some(name) = node.node_name().map(|name| name.to_string()) else {
            return self.render_children(node, depth, list_depth, ordered_list, preformatted);
        };
        if matches!(
            name.as_str(),
            "script"
                | "style"
                | "iframe"
                | "object"
                | "embed"
                | "applet"
                | "template"
                | "noscript"
                | "svg"
                | "math"
        ) {
            self.warn(
                "html_content_removed",
                format!("Removed active or non-document <{name}> content."),
            );
            return Ok(());
        }
        match name.as_str() {
            "head" | "title" | "meta" | "link" | "base" => Ok(()),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.ensure_blank_line();
                let level = name[1..].parse::<usize>().unwrap_or(1);
                self.output.push_str(&"#".repeat(level));
                self.output.push(' ');
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "p" | "article" | "section" | "main" | "header" | "footer" | "aside" | "nav"
            | "div" | "figure" | "figcaption" | "address" => {
                self.ensure_blank_line();
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "br" => {
                self.output.push_str("  \n");
                Ok(())
            }
            "hr" => {
                self.ensure_blank_line();
                self.output.push_str("---");
                self.ensure_blank_line();
                Ok(())
            }
            "strong" | "b" => {
                self.flush_pending_space();
                self.output.push_str("**");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push_str("**");
                Ok(())
            }
            "em" | "i" => {
                self.flush_pending_space();
                self.output.push('*');
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push('*');
                Ok(())
            }
            "del" | "s" | "strike" => {
                self.flush_pending_space();
                self.output.push_str("~~");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push_str("~~");
                Ok(())
            }
            "code" if !preformatted => {
                self.flush_pending_space();
                self.push_inline_code(node.text().trim());
                Ok(())
            }
            "pre" => {
                self.ensure_blank_line();
                self.push_fenced_code(node.text().trim_matches('\n'));
                self.ensure_blank_line();
                Ok(())
            }
            "a" => {
                let href = node.attr("href").map(|value| value.to_string());
                if let Some(href) = href.as_deref().and_then(safe_html_target) {
                    self.flush_pending_space();
                    self.output.push('[');
                    let before = self.output.len();
                    self.render_children(node, depth, list_depth, ordered_list, false)?;
                    if self.output.len() == before {
                        self.output.push_str(&escape_markdown_text(href));
                    }
                    self.output.push_str("](");
                    self.output.push_str(&markdown_destination(href));
                    self.output.push(')');
                } else {
                    if href.is_some() {
                        self.warn(
                            "html_unsafe_target_removed",
                            "Removed an unsafe HTML link target.",
                        );
                    }
                    self.render_children(node, depth, list_depth, ordered_list, false)?;
                }
                Ok(())
            }
            "img" => {
                let source = node.attr("src").map(|value| value.to_string());
                if let Some(source) = source.as_deref().and_then(safe_html_target) {
                    let alt = node
                        .attr("alt")
                        .map(|value| value.to_string())
                        .unwrap_or_default();
                    self.flush_pending_space();
                    self.output.push_str("![");
                    self.output.push_str(&escape_markdown_text(&alt));
                    self.output.push_str("](");
                    self.output.push_str(&markdown_destination(source));
                    self.output.push(')');
                } else if source.is_some() {
                    self.warn(
                        "html_unsafe_target_removed",
                        "Removed an unsafe HTML image target.",
                    );
                }
                Ok(())
            }
            "ul" | "ol" => {
                self.ensure_line_break();
                self.render_children(node, depth, list_depth + 1, name == "ol", false)?;
                self.ensure_line_break();
                Ok(())
            }
            "li" => {
                self.ensure_line_break();
                self.output
                    .push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                self.output
                    .push_str(if ordered_list { "1. " } else { "- " });
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_line_break();
                Ok(())
            }
            "blockquote" => {
                self.ensure_blank_line();
                self.output.push_str("> ");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "table" | "thead" | "tbody" | "tfoot" => {
                self.ensure_blank_line();
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "tr" => {
                let header_cells = node
                    .children()
                    .into_iter()
                    .filter(|child| {
                        child
                            .node_name()
                            .is_some_and(|name| name.to_string() == "th")
                    })
                    .count();
                self.ensure_line_break();
                self.output.push_str("| ");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_line_break();
                if header_cells > 0 {
                    self.output.push('|');
                    for _ in 0..header_cells {
                        self.output.push_str(" --- |");
                    }
                    self.ensure_line_break();
                }
                Ok(())
            }
            "th" | "td" => {
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push_str(" | ");
                Ok(())
            }
            _ => self.render_children(node, depth, list_depth, ordered_list, preformatted),
        }
    }

    pub(super) fn render_children(
        &mut self,
        node: NodeRef<'_>,
        depth: usize,
        list_depth: usize,
        ordered_list: bool,
        preformatted: bool,
    ) -> Result<(), CoreError> {
        for child in node.children() {
            self.render_node(child, depth + 1, list_depth, ordered_list, preformatted)?;
        }
        Ok(())
    }

    pub(super) fn push_normalized_text(&mut self, value: &str) {
        if value.chars().next().is_some_and(char::is_whitespace) {
            self.pending_space = true;
        }
        let mut emitted_word = false;
        for word in value.split_whitespace() {
            if emitted_word {
                self.pending_space = true;
            }
            self.flush_pending_space();
            self.output.push_str(&escape_markdown_text(word));
            emitted_word = true;
        }
        if emitted_word {
            self.pending_space = value.chars().last().is_some_and(char::is_whitespace);
        } else if value.chars().any(char::is_whitespace) {
            self.pending_space = true;
        }
    }

    pub(super) fn flush_pending_space(&mut self) {
        if self.pending_space
            && !self.output.is_empty()
            && !self.output.ends_with(char::is_whitespace)
        {
            self.output.push(' ');
        }
        self.pending_space = false;
    }

    pub(super) fn warn(&mut self, code: &'static str, message: impl Into<String>) {
        self.warnings.insert(HtmlConversionWarning {
            code,
            message: message.into(),
        });
    }

    pub(super) fn push_inline_code(&mut self, value: &str) {
        let delimiter = "`".repeat(longest_character_run(value, '`').saturating_add(1).max(1));
        let pad = value.starts_with(['`', ' ']) || value.ends_with(['`', ' ']);
        self.output.push_str(&delimiter);
        if pad {
            self.output.push(' ');
        }
        self.output.push_str(value);
        if pad {
            self.output.push(' ');
        }
        self.output.push_str(&delimiter);
    }

    pub(super) fn push_fenced_code(&mut self, value: &str) {
        let delimiter = "`".repeat(longest_character_run(value, '`').saturating_add(1).max(3));
        self.output.push_str(&delimiter);
        self.output.push('\n');
        self.output.push_str(value);
        self.output.push('\n');
        self.output.push_str(&delimiter);
    }

    pub(super) fn ensure_line_break(&mut self) {
        self.pending_space = false;
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

pub(super) fn convert_html_to_markdown(html: &str) -> Result<HtmlConversion, CoreError> {
    HtmlMarkdownWriter {
        output: String::new(),
        warnings: BTreeSet::new(),
        visited_nodes: 0,
        pending_space: false,
    }
    .render(&Document::from(html))
}

pub(super) fn safe_html_target(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty()
        || value.contains('\\')
        || value.contains(['<', '>', '"', '\''])
        || value.chars().any(char::is_control)
    {
        return None;
    }
    if value.starts_with("//") || value.starts_with('#') {
        return Some(value);
    }
    if value.starts_with('/') {
        return None;
    }
    if let Some((scheme, _)) = value.split_once(':') {
        return matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto"
        )
        .then_some(value);
    }
    Some(value)
}

pub(super) fn escape_markdown_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '|' | '<' | '>' | '#' | '+' | '-' | '!'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

pub(super) fn longest_character_run(value: &str, target: char) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for character in value.chars() {
        if character == target {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

pub(super) fn markdown_destination(value: &str) -> String {
    value
        .replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29")
}

pub(super) fn discover_markdown_links(body: &str) -> Vec<StagedLink> {
    Parser::new_ext(body, Options::all())
        .filter_map(|event| match event {
            Event::Start(Tag::Link { dest_url, .. }) => Some(StagedLink {
                kind: if is_external_markdown_target(&dest_url) {
                    StagedLinkKind::External
                } else {
                    StagedLinkKind::Internal
                },
                target: dest_url.to_string(),
                label: None,
                resolution: StagedLinkResolution::Unresolved,
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: None,
            }),
            Event::Start(Tag::Image { dest_url, .. }) => Some(StagedLink {
                kind: StagedLinkKind::Embed,
                target: dest_url.to_string(),
                label: None,
                resolution: StagedLinkResolution::Unresolved,
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: None,
            }),
            _ => None,
        })
        .collect()
}

pub(super) fn discover_obsidian_links(body: &str) -> Vec<StagedLink> {
    let mut links = Vec::new();
    let mut fence = None::<(u8, usize)>;
    let mut frontmatter = markdown_frontmatter(body).is_some();
    let mut first_line = true;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if frontmatter {
            if !first_line && matches!(trimmed, "---" | "...") {
                frontmatter = false;
            }
            first_line = false;
            continue;
        }
        first_line = false;
        if let Some((marker, opening_run)) = fence {
            let bytes = trimmed.as_bytes();
            let run = bytes.iter().take_while(|byte| **byte == marker).count();
            if run >= opening_run && bytes[run..].iter().all(u8::is_ascii_whitespace) {
                fence = None;
            }
            continue;
        }
        if let Some(marker @ (0x60 | b'~')) = trimmed.as_bytes().first().copied() {
            let run = trimmed
                .as_bytes()
                .iter()
                .take_while(|byte| **byte == marker)
                .count();
            if run >= 3 {
                fence = Some((marker, run));
                continue;
            }
        }
        let bytes = line.as_bytes();
        let mut index = 0;
        let mut inline_code = None::<usize>;
        while index < bytes.len() {
            if bytes[index] == 0x60 {
                let run = bytes[index..]
                    .iter()
                    .take_while(|byte| **byte == 0x60)
                    .count();
                if !obsidian_syntax_is_escaped(bytes, index) {
                    match inline_code {
                        Some(opening_run) if opening_run == run => inline_code = None,
                        None => inline_code = Some(run),
                        _ => {}
                    }
                }
                index += run;
                continue;
            }
            if inline_code.is_some() {
                index += 1;
                continue;
            }
            let (embed, open) = if bytes[index] == b'!'
                && bytes.get(index + 1) == Some(&b'[')
                && bytes.get(index + 2) == Some(&b'[')
            {
                (true, index + 1)
            } else if bytes[index] == b'[' && bytes.get(index + 1) == Some(&b'[') {
                (false, index)
            } else {
                index += 1;
                continue;
            };
            let raw_start = if embed { open - 1 } else { open };
            if obsidian_syntax_is_escaped(bytes, raw_start) {
                index = open + 2;
                continue;
            }
            let content_start = open + 2;
            let Some(relative_end) = line[content_start..].find("]]") else {
                break;
            };
            let content_end = content_start + relative_end;
            let content = line[content_start..content_end].trim();
            let raw_end = content_end + 2;
            let raw = line[raw_start..raw_end].to_owned();
            let (target, label) = content
                .split_once('|')
                .map(|(target, label)| (target.trim(), Some(label.trim())))
                .unwrap_or((content, None));
            if !target.is_empty() {
                links.push(StagedLink {
                    kind: if embed {
                        StagedLinkKind::Embed
                    } else {
                        StagedLinkKind::Internal
                    },
                    target: target.into(),
                    label: label.filter(|label| !label.is_empty()).map(str::to_owned),
                    resolution: StagedLinkResolution::Unresolved,
                    resolved_object_id: None,
                    candidate_object_ids: Vec::new(),
                    raw: Some(raw),
                });
            }
            index = raw_end;
        }
    }
    links
}

pub(super) fn obsidian_syntax_is_escaped(bytes: &[u8], index: usize) -> bool {
    let preceding_backslashes = bytes[..index]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count();
    preceding_backslashes % 2 == 1
}
