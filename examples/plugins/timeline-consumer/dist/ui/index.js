import { createPluginI18n, createBrowserPluginRpcTransport, createPluginRpcClient } from "./plugin-sdk.js";

const pluginId = document.body.dataset.plugin;
const projectId = new URLSearchParams(window.location.search).get("project");
const status = document.querySelector("#status");
const form = document.querySelector("#form");
const dateInput = document.querySelector("#date");
const result = document.querySelector("#result");
const client = createPluginRpcClient(createBrowserPluginRpcTransport({ pluginId, projectId }));

const i18n = createPluginI18n();
let statusMessage = "Ready. Timeline may be optional; a missing provider returns provider-unavailable.";

let serviceError = null;
i18n.subscribe(() => {
  for (const element of document.querySelectorAll("[data-i18n]")) {
    element.textContent = i18n.t(element.dataset.i18n);
  }
  status.textContent = serviceError ?? i18n.t(statusMessage);
});

function showStatus(message) {
  statusMessage = message;
  serviceError = null;
  status.textContent = i18n.t(message);
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  result.hidden = true;
  showStatus("Calling daena.timeline.resolve-date@1…");
  try {
    const resolved = await client.callService("daena.timeline.resolve-date", 1, { date: dateInput.value }, 1_000);
    result.hidden = false;
    result.textContent = JSON.stringify(resolved, null, 2);
    showStatus("Resolved through the host broker.");
  } catch (cause) {
    serviceError = cause instanceof Error ? cause.message : String(cause);
    status.textContent = serviceError;
  }
});
