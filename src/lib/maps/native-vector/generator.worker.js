// @ts-nocheck
import { generateCandidates } from "./generator";

self.onmessage = (event) => {
  const message = event.data;
  if (message?.type !== "generate") return;
  try {
    const candidates = generateCandidates(message.settings);
    self.postMessage({ type: "result", requestId: message.requestId, candidates });
  } catch (cause) {
    self.postMessage({
      type: "error",
      requestId: message.requestId,
      message: cause instanceof Error ? cause.message : String(cause),
    });
  }
};
