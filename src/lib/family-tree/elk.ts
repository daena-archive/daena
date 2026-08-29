import ElkConstructor from "elkjs/lib/elk-api.js";
import type { ELK } from "elkjs/lib/elk-api.js";
import workerUrl from "elkjs/lib/elk-worker.min.js?url";
import type { ElkNodeInput } from "./layout.ts";

let client: ELK | null = null;

export function requestElkLayout(graph: ElkNodeInput): Promise<ElkNodeInput> {
  if (!client) client = new ElkConstructor({ workerUrl });
  return client.layout(graph) as Promise<ElkNodeInput>;
}

export function terminateElkLayout() {
  client?.terminateWorker();
  client = null;
}
