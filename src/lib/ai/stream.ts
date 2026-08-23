import type { AiStreamEvent } from "$lib/project/client";

export interface TextGenerationStreamState {
  streamText: string;
  proposal: string;
  progressMessage: string;
}

function fullerOutput(streamText: string, terminalOutput: string | null): string {
  if (terminalOutput === null) return streamText;
  return terminalOutput.length >= streamText.length ? terminalOutput : streamText;
}

export function reduceTextGenerationEvent(
  state: TextGenerationStreamState,
  event: AiStreamEvent,
): TextGenerationStreamState {
  if (event.phase === "started") return { ...state, progressMessage: "Preparing model…" };
  if (event.phase === "reasoning") return { ...state, progressMessage: "Model is thinking…" };
  if (event.phase === "delta" && event.delta) {
    return {
      ...state,
      streamText: state.streamText + event.delta,
      progressMessage: "Writing proposal…",
    };
  }
  if (
    event.phase === "completed" ||
    event.phase === "cancelled" ||
    event.phase === "deadline_exceeded" ||
    event.phase === "failed"
  ) {
    const proposal = fullerOutput(state.streamText, event.output);
    return {
      streamText: state.streamText,
      proposal: proposal || state.proposal,
      progressMessage: "",
    };
  }
  return state;
}
