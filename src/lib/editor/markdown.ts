const SAFE_URL = /^(?:https?:|mailto:|#)/i;

function escapeHtml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

function safeUrl(value: string): string {
  const trimmed = value.trim();
  return SAFE_URL.test(trimmed) ? trimmed : "";
}

function inlineMarkdown(value: string): string {
  const tokens: string[] = [];
  const protect = (html: string) => {
    const index = tokens.push(html) - 1;
    return `\u0000${index}\u0000`;
  };
  let text = value.replace(/<(u|mark)>([\s\S]*?)<\/\1>/gi, (_, tag: string, body: string) =>
    protect(`<${tag.toLowerCase()}>${escapeHtml(body)}</${tag.toLowerCase()}>`),
  );
  text = text.replace(/`([^`\n]+)`/g, (_, code: string) => protect(`<code>${escapeHtml(code)}</code>`));
  text = escapeHtml(text);
  text = text.replace(
    /!\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/g,
    (_, alt: string, rawUrl: string, title?: string) => {
      const url = safeUrl(rawUrl);
      return url
        ? `<img src="${escapeHtml(url)}" alt="${escapeHtml(alt)}"${title ? ` title="${escapeHtml(title)}"` : ""}>`
        : escapeHtml(alt);
    },
  );
  text = text.replace(/\[([^\]]+)\]\(\s*javascript:[^\n]*\)/gi, "$1");
  text = text.replace(
    /\[([^\]]+)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/g,
    (_, label: string, rawUrl: string, title?: string) => {
      const url = safeUrl(rawUrl);
      return url ? `<a href="${escapeHtml(url)}">${label}</a>` : label;
    },
  );
  text = text.replace(
    /\*\*([^*\n]+)\*\*|__([^_\n]+)__/g,
    (_, strong: string, alternate: string) => `<strong>${strong ?? alternate}</strong>`,
  );
  text = text.replace(/~~([^~\n]+)~~/g, "<s>$1</s>");
  text = text.replace(
    /(?<!\w)\*([^*\n]+)\*(?!\w)|(?<!\w)_([^_\n]+)_(?!\w)/g,
    (_, italic: string, alternate: string) => `<em>${italic ?? alternate}</em>`,
  );
  return text.replace(/\u0000(\d+)\u0000/g, (_, index: string) => tokens[Number(index)] ?? "");
}

function renderBlock(lines: string[]): string {
  const output: string[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }
    const aligned = line.match(/^\s*<div\s+align=["'](left|center|right)["']>([\s\S]*)<\/div>\s*$/i);
    if (aligned) {
      output.push(`<p style="text-align: ${aligned[1].toLowerCase()}">${inlineMarkdown(aligned[2])}</p>`);
      index += 1;
      continue;
    }
    const fence = line.match(/^\s*```(\w*)\s*$/);
    if (fence) {
      const body: string[] = [];
      index += 1;
      while (index < lines.length && !/^\s*```\s*$/.test(lines[index])) body.push(lines[index++]);
      if (index < lines.length) index += 1;
      output.push(
        `<pre><code${fence[1] ? ` data-language="${escapeHtml(fence[1])}"` : ""}>${escapeHtml(body.join("\n"))}</code></pre>`,
      );
      continue;
    }
    const heading = line.match(/^\s{0,3}(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (heading) {
      output.push(`<h${heading[1].length}>${inlineMarkdown(heading[2])}</h${heading[1].length}>`);
      index += 1;
      continue;
    }
    if (/^\s{0,3}(?:\*\s*){3,}$|^\s{0,3}(?:-\s*){3,}$|^\s{0,3}(?:_\s*){3,}$/.test(line)) {
      output.push("<hr>");
      index += 1;
      continue;
    }
    if (/^\s*>/.test(line)) {
      const quote: string[] = [];
      while (index < lines.length && /^\s*>/.test(lines[index])) quote.push(lines[index++].replace(/^\s*>\s?/, ""));
      output.push(`<blockquote>${renderBlock(quote)}</blockquote>`);
      continue;
    }
    const list = line.match(/^\s*([-+*])\s+(.+)$/) ?? line.match(/^\s*(\d+)[.)]\s+(.+)$/);
    if (list) {
      const ordered = /^\d/.test(list[1]);
      const items: string[] = [];
      while (index < lines.length) {
        const next = lines[index].match(ordered ? /^\s*\d+[.)]\s+(.+)$/ : /^\s*[-+*]\s+(.+)$/);
        if (!next) break;
        items.push(`<li>${inlineMarkdown(next[2] ?? next[1] ?? "")}</li>`);
        index += 1;
      }
      output.push(`<${ordered ? "ol" : "ul"}>${items.join("")}</${ordered ? "ol" : "ul"}>`);
      continue;
    }
    const paragraph: string[] = [line];
    index += 1;
    while (
      index < lines.length &&
      lines[index].trim() &&
      !/^\s*(?:```|#{1,6}\s|>|[-+*]\s+|\d+[.)]\s+)/.test(lines[index])
    )
      paragraph.push(lines[index++]);
    output.push(`<p>${inlineMarkdown(paragraph.join("\n")).replaceAll("\n", "<br>")}</p>`);
  }
  return output.join("");
}

export function markdownToHtml(markdown: string): string {
  return renderBlock(markdown.replace(/\r\n?/g, "\n").split("\n"));
}

function escapeMarkdown(value: string): string {
  return value.replace(/[\\`*_[\]~]/g, "\\$&");
}

function serializeNode(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) return escapeMarkdown(node.nodeValue ?? "");
  if (!(node instanceof HTMLElement)) return [...node.childNodes].map(serializeNode).join("");
  const content = [...node.childNodes].map(serializeNode).join("");
  const alignment = node.style.textAlign || node.getAttribute("align") || "";
  if ((alignment === "center" || alignment === "right") && node.tagName.toLowerCase() !== "pre") {
    return `<div align="${alignment}">${content}</div>\n\n`;
  }
  switch (node.tagName.toLowerCase()) {
    case "strong":
    case "b":
      return `**${content}**`;
    case "em":
    case "i":
      return `*${content}*`;
    case "u":
      return `<u>${content}</u>`;
    case "s":
    case "strike":
    case "del":
      return `~~${content}~~`;
    case "code":
      return node.parentElement?.tagName.toLowerCase() === "pre" ? content : `\`${content}\``;
    case "a": {
      const href = safeUrl(node.getAttribute("href") ?? "");
      return href ? `[${content}](${href})` : content;
    }
    case "br":
      return "\n";
    case "h1":
    case "h2":
    case "h3":
    case "h4":
    case "h5":
    case "h6":
      return `${"#".repeat(Number(node.tagName[1]))} ${content.trim()}\n\n`;
    case "p":
    case "div":
      return `${content.trim()}\n\n`;
    case "blockquote":
      return (
        content
          .trim()
          .split("\n")
          .map((line) => `> ${line}`)
          .join("\n") + "\n\n"
      );
    case "hr":
      return "---\n\n";
    case "pre": {
      const code = node.querySelector("code");
      const language =
        code?.getAttribute("data-language") ?? code?.className.match(/(?:^|\s)language-([\w-]+)/)?.[1] ?? "";
      return `\`\`\`${language}\n${content.replace(/^\n|\n$/g, "")}\n\`\`\`\n\n`;
    }
    case "ul":
      return (
        [...node.children].map((item) => `- ${[...item.childNodes].map(serializeNode).join("").trim()}\n`).join("") +
        "\n"
      );
    case "ol":
      return (
        [...node.children]
          .map((item, index) => `${index + 1}. ${[...item.childNodes].map(serializeNode).join("").trim()}\n`)
          .join("") + "\n"
      );
    case "li":
      return content;
    case "img": {
      const src = safeUrl(node.getAttribute("src") ?? "");
      return src ? `![${node.getAttribute("alt") ?? ""}](${src})` : "";
    }
    default:
      return content;
  }
}

export function htmlToMarkdown(html: string): string {
  if (typeof DOMParser === "undefined") return html.trim() ? `${html.trim()}\n` : "";
  const document = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
  const body = document.body;
  const markdown = [...body.childNodes]
    .map(serializeNode)
    .join("")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
  return markdown ? `${markdown}\n` : "";
}
