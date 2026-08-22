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
