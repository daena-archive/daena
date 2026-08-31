/// <reference lib="webworker" />
import ELK from "elkjs/lib/elk.bundled.js";
import type { LayoutRequest, LayoutResponse } from "./layout.ts";

const elk = new ELK();

self.onmessage = async (event: MessageEvent<LayoutRequest>) => {
  const { generation, graph } = event.data;
  try {
    const laidOut = (await elk.layout(graph)) as LayoutRequest["graph"];
    const response: LayoutResponse = { generation, ok: true, graph: laidOut };
    self.postMessage(response);
  } catch (cause) {
    const response: LayoutResponse = {
      generation,
      ok: false,
      message: cause instanceof Error ? cause.message : String(cause),
    };
    self.postMessage(response);
  }
};
