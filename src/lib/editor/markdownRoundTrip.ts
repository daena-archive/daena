import { detectTextDirection } from "../markdown/direction.ts";

export function taskListsForEditor(html: string): string {
  if (typeof document === "undefined") return html;
  const template = document.createElement("template");
  template.innerHTML = html;
  for (const list of template.content.querySelectorAll<HTMLElement>("ul.contains-task-list")) {
    list.dataset.type = "taskList";
  }
  for (const item of template.content.querySelectorAll<HTMLElement>("li.task-list-item")) {
    const checkbox = item.querySelector<HTMLInputElement>('input[type="checkbox"]');
    item.dataset.type = "taskItem";
    item.dataset.checked = checkbox?.checked || checkbox?.hasAttribute("checked") ? "true" : "false";
    checkbox?.remove();
    const content = document.createElement("div");
    while (item.firstChild) content.appendChild(item.firstChild);
    item.appendChild(content);
  }
  return template.innerHTML;
}

export function taskListsForMarkdown(html: string): string {
  if (typeof document === "undefined") return html;
  const template = document.createElement("template");
  template.innerHTML = html;
  for (const list of template.content.querySelectorAll<HTMLElement>('ul[data-type="taskList"]')) {
    list.classList.add("contains-task-list");
    list.removeAttribute("data-type");
  }
  for (const item of template.content.querySelectorAll<HTMLElement>('li[data-type="taskItem"]')) {
    const checked = item.getAttribute("data-checked") === "true";
    item.classList.add("task-list-item");
    item.removeAttribute("data-type");
    item.removeAttribute("data-checked");
    item.querySelector(":scope > label")?.remove();
    const content = item.querySelector<HTMLElement>(":scope > div");
    if (content) content.replaceWith(...content.childNodes);
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.disabled = true;
    checkbox.checked = checked;
    if (checked) checkbox.setAttribute("checked", "");
    item.prepend(checkbox);
  }
  return template.innerHTML;
}

/**
 * Markdown stores a block's direction only when it overrides its text. The editor expects every
 * paragraph and heading to carry the direction its text implies, so restore it on load.
 */
export function directionsForEditor(html: string): string {
  if (typeof document === "undefined") return html;
  const template = document.createElement("template");
  template.innerHTML = html;
  for (const block of template.content.querySelectorAll<HTMLElement>(
    "p:not([dir]), h1:not([dir]), h2:not([dir]), h3:not([dir]), h4:not([dir]), h5:not([dir]), h6:not([dir])",
  )) {
    const dir = detectTextDirection(block.textContent ?? "");
    if (dir) block.setAttribute("dir", dir);
  }
  return template.innerHTML;
}
