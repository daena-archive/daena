export function field(label: string, control: HTMLElement) {
  const wrapper = document.createElement("label");
  wrapper.className = "language-field";
  const title = document.createElement("span");
  title.textContent = label;
  wrapper.append(title, control);
  return wrapper;
}

export function input(name: string, value = "", list?: string) {
  const control = document.createElement("input");
  control.name = name;
  control.value = value;
  if (list) control.setAttribute("list", list);
  return control;
}

export function textarea(name: string, value = "", rows = 3) {
  const control = document.createElement("textarea");
  control.name = name;
  control.value = value;
  control.rows = rows;
  return control;
}

export function button(label: string, className: string, onclick: () => void) {
  const control = document.createElement("button");
  control.type = "button";
  control.className = className;
  control.textContent = label;
  control.onclick = onclick;
  return control;
}

export function groupHead(title: string, add: () => void) {
  const head = document.createElement("div");
  head.className = "language-group-head";
  const heading = document.createElement("h3");
  heading.textContent = title;
  head.append(heading, button("Add", "language-button secondary", add));
  return head;
}

export function row(fields: HTMLElement[], remove: () => void) {
  const wrap = document.createElement("div");
  wrap.className = "language-inline";
  wrap.append(...fields, button("Remove", "language-button secondary language-danger", remove));
  return wrap;
}

export function replaceEditor(current: HTMLElement, next: HTMLElement, focus = "[name=lemma]") {
  current.replaceWith(next);
  next.querySelector<HTMLInputElement>(focus)?.focus();
}

export function alertMessage(text: string) {
  const message = document.createElement("p");
  message.className = "language-status error";
  message.setAttribute("role", "alert");
  message.textContent = text;
  return message;
}

export function emptyMessage(text: string) {
  const message = document.createElement("p");
  message.className = "language-empty";
  message.textContent = text;
  return message;
}
