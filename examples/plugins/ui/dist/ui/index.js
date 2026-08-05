const app = document.querySelector("#app");

if (!app) throw new Error("Ink Tools mount point is missing");

const pluginId = document.body.dataset.plugin;
const projectId = new URLSearchParams(window.location.search).get("project");
const { createBrowserPluginRpcTransport, createPluginRpcClient } = await import("./plugin-sdk.js");
const client = createPluginRpcClient(createBrowserPluginRpcTransport({ pluginId, projectId }));

app.innerHTML = `
  <div class="shell">
    <header class="topbar">
      <div class="brand-lockup">
        <span class="brand-mark" aria-hidden="true">✦</span>
        <div>
<p class="eyebrow">Daena Archive / Plugin example</p>
          <h1>Ink Tools</h1>
        </div>
      </div>
      <div class="runtime-pill"><span class="status-dot"></span> Isolated workspace</div>
    </header>

    <section class="hero panel">
      <div>
        <p class="eyebrow">A small room for unfinished ideas</p>
        <h2>Shape a scene before it hardens.</h2>
        <p class="hero-copy">Collect a few beats, keep the language loose, and let the next useful detail find its place.</p>
        <button class="primary-action" type="button" data-action="focus-note">Capture a scratch note <span aria-hidden="true">↗</span></button>
      </div>
      <div class="hero-stamp" aria-label="Sandboxed plugin">
        <span class="stamp-icon">◎</span>
        <strong>Sandboxed</strong>
        <small>Public SDK UI</small>
      </div>
    </section>

    <div class="content-grid">
      <section class="panel beats-panel">
        <div class="section-heading">
          <div>
            <p class="eyebrow">Working outline</p>
            <h3>Scene beats</h3>
          </div>
          <span class="count-badge">03</span>
        </div>
        <ol class="beat-list">
          <li><span class="beat-number">01</span><div><strong>The room remembers</strong><p>Give the setting one detail that has outlived its owner.</p></div><span class="beat-state">seed</span></li>
          <li><span class="beat-number">02</span><div><strong>A promise is tested</strong><p>Put a small cost in the way of the obvious choice.</p></div><span class="beat-state">shape</span></li>
          <li><span class="beat-number">03</span><div><strong>Leave a door open</strong><p>End with an image that creates a question, not an answer.</p></div><span class="beat-state">next</span></li>
        </ol>
      </section>

      <aside class="side-column">
        <section class="panel palette-panel">
          <div class="section-heading"><div><p class="eyebrow">Atmosphere</p><h3>Motif palette</h3></div><span class="palette-symbol" aria-hidden="true">◌</span></div>
          <div class="swatches" role="list" aria-label="Motif palette">
            <button type="button" class="swatch amber" data-motif="amber" aria-label="Amber motif"></button>
            <button type="button" class="swatch moss" data-motif="moss" aria-label="Moss motif"></button>
            <button type="button" class="swatch ink" data-motif="ink" aria-label="Ink motif"></button>
            <button type="button" class="swatch rose" data-motif="rose" aria-label="Rose motif"></button>
          </div>
          <p id="motif-label" class="muted-copy">Choose a color to set the mood of this draft.</p>
        </section>

        <section class="panel status-panel">
          <div class="section-heading"><div><p class="eyebrow">Runtime check</p><h3>Good fences</h3></div><span class="checkmark">✓</span></div>
          <ul class="status-list">
            <li><span>Host DOM</span><strong>Unavailable</strong></li>
            <li><span>Tauri APIs</span><strong>Unavailable</strong></li>
            <li><span>Local files</span><strong>Unavailable</strong></li>
            <li><span>Broker</span><strong class="available">Ready</strong></li>
          </ul>
        </section>
      </aside>
    </div>

    <section class="panel project-panel">
      <div class="section-heading">
        <div><p class="eyebrow">From the open project</p><h3>Nearby entries</h3></div>
        <span id="project-count" class="muted-copy">Connecting…</span>
      </div>
      <div id="project-entities" class="project-entities" aria-live="polite"><p class="muted-copy">Reading project entries through the broker…</p></div>
    </section>

    <section class="panel notes-panel">
      <div class="section-heading">
        <div><p class="eyebrow">Scratchpad</p><h3>Loose language</h3></div>
        <span id="note-count" class="muted-copy">0 notes</span>
      </div>
      <form id="note-form" class="note-form">
        <label class="sr-only" for="note-input">Add a scratch note</label>
        <input id="note-input" name="note" maxlength="140" autocomplete="off" placeholder="Add a detail worth keeping…">
        <button class="secondary-action" type="submit">Keep note</button>
      </form>
      <ul id="note-list" class="note-list" aria-live="polite"></ul>
      <p id="interaction-status" class="interaction-status" role="status">This local demo state stays inside the plugin view.</p>
    </section>
  </div>
`;

const noteForm = document.querySelector("#note-form");
const noteInput = document.querySelector("#note-input");
const noteList = document.querySelector("#note-list");
const noteCount = document.querySelector("#note-count");
const interactionStatus = document.querySelector("#interaction-status");
const projectCount = document.querySelector("#project-count");
const projectEntities = document.querySelector("#project-entities");
const runtimePill = document.querySelector(".runtime-pill");

function setRuntimeStatus(message, isError = false) {
  if (runtimePill) {
    runtimePill.classList.toggle("error", isError);
    runtimePill.innerHTML = `<span class="status-dot"></span> ${message}`;
  }
}

function updateNoteCount() {
  const count = noteList?.children.length ?? 0;
  if (noteCount) noteCount.textContent = `${count} ${count === 1 ? "note" : "notes"}`;
}

function addNote(text, saved = false) {
  if (!noteList || !text.trim()) return;
  const item = document.createElement("li");
  item.className = "note-item";
  item.innerHTML = `<span class="note-pin">•</span><span></span><small>${saved ? "saved" : "draft"}</small>`;
  item.querySelector("span:nth-child(2)").textContent = text.trim();
  noteList.prepend(item);
  updateNoteCount();
  if (interactionStatus && !saved) interactionStatus.textContent = "Note kept in this sandbox view.";
}

noteForm?.addEventListener("submit", (event) => {
  event.preventDefault();
  void saveNote();
});

async function saveNote() {
  if (!(noteInput instanceof HTMLInputElement) || !noteInput.value.trim()) return;
  const text = noteInput.value.trim();
  const motif = document.querySelector("[data-motif].selected")?.getAttribute("data-motif") ?? "amber";
  const button = noteForm?.querySelector("button");
  if (button instanceof HTMLButtonElement) button.disabled = true;
  if (interactionStatus) interactionStatus.textContent = "Saving note to the project…";
  try {
    await client.call("entity.create", {
      name: text.slice(0, 80),
      type: "scratch-note",
      fields: [
        { namespace: "ink-tools", key: "body", value: text },
        { namespace: "ink-tools", key: "motif", value: motif },
      ],
    });
    addNote(text, true);
    noteInput.value = "";
    if (interactionStatus) interactionStatus.textContent = "Saved to the Ink Tools project namespace.";
  } catch (cause) {
    if (interactionStatus) interactionStatus.textContent = `Could not save: ${cause instanceof Error ? cause.message : String(cause)}`;
  } finally {
    if (button instanceof HTMLButtonElement) button.disabled = false;
    noteInput.focus();
  }
}

function entityType(entity) {
  return entity.entity_type ?? entity.entityType ?? "entry";
}

function renderProjectEntities(entities) {
  if (!projectEntities || !projectCount) return;
  projectCount.textContent = `${entities.length} shown`;
  projectEntities.replaceChildren();
  if (!entities.length) {
    const empty = document.createElement("p");
    empty.className = "muted-copy";
    empty.textContent = "No project entries are available yet.";
    projectEntities.append(empty);
    return;
  }
  for (const entity of entities.slice(0, 6)) {
    const item = document.createElement("article");
    item.className = "project-entity";
    const name = document.createElement("strong");
    name.textContent = entity.name ?? "Untitled entry";
    const type = document.createElement("span");
    type.textContent = entityType(entity);
    item.append(name, type);
    projectEntities.append(item);
  }
}

async function loadProjectData() {
  const entities = await client.call("entity.list", {});
  const scratchNotes = entities.filter((entity) => entityType(entity) === "scratch-note");
  const storedNotes = await Promise.all(scratchNotes.slice(-12).map(async (entity) => {
    const fields = await client.call("field.list", { entityId: entity.id, namespace: "ink-tools" });
    return fields.find((field) => field.key === "body")?.value;
  }));
  for (const note of storedNotes.filter((value) => typeof value === "string")) addNote(note, true);
  renderProjectEntities(entities.filter((entity) => entityType(entity) !== "scratch-note"));
  setRuntimeStatus("Broker connected");
}

void loadProjectData().catch((cause) => {
  setRuntimeStatus("Broker unavailable", true);
  if (projectCount) projectCount.textContent = "Unavailable";
  if (projectEntities) {
    projectEntities.replaceChildren();
    const message = document.createElement("p");
    message.className = "muted-copy error-copy";
    message.textContent = cause instanceof Error ? cause.message : String(cause);
    projectEntities.append(message);
  }
  if (interactionStatus) interactionStatus.textContent = "The scratchpad needs an active project grant to save notes.";
});

document.querySelector('[data-action="focus-note"]')?.addEventListener("click", () => {
  noteInput?.focus();
  if (interactionStatus) interactionStatus.textContent = "The scratchpad is ready for a new detail.";
});

const motifNames = { amber: "Warm light", moss: "Old growth", ink: "Night ink", rose: "Hidden tenderness" };
document.querySelectorAll("[data-motif]").forEach((button) => {
  button.addEventListener("click", () => {
    const motif = button.getAttribute("data-motif");
    document.querySelectorAll("[data-motif]").forEach((candidate) => candidate.classList.toggle("selected", candidate === button));
    const label = document.querySelector("#motif-label");
    if (label && motif) label.textContent = `${motifNames[motif] ?? "New"} — a quiet direction for this draft.`;
  });
});

// The host injects the broker transport; no Tauri API or local path is used.
