import { createBrowserPluginRpcTransport, createPluginRpcClient } from "./plugin-sdk.js";

const pluginId = document.body.dataset.plugin;
const projectId = new URLSearchParams(window.location.search).get("project");
const status = document.querySelector("#status");
const form = document.querySelector("#form");
const dateInput = document.querySelector("#date");
const result = document.querySelector("#result");
const client = createPluginRpcClient(createBrowserPluginRpcTransport({ pluginId, projectId }));

status.textContent = "Ready. Timeline may be optional; a missing provider returns provider-unavailable.";

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  result.hidden = true;
  status.textContent = "Calling daena.timeline.resolve-date@1…";
  try {
    const resolved = await client.callService("daena.timeline.resolve-date", 1, { date: dateInput.value }, 1_000);
    result.hidden = false;
    result.textContent = JSON.stringify(resolved, null, 2);
    status.textContent = "Resolved through the host broker.";
  } catch (cause) {
    status.textContent = cause instanceof Error ? cause.message : String(cause);
  }
});
