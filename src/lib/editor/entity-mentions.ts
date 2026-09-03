export type MentionTrigger = {
  query: string;
  length: number;
};

export type MentionLabel = {
  text: string;
  isCustom: boolean;
};

const PRECEDING_OK = /[\s([{]/;

export function findMentionTrigger(textBeforeCursor: string): MentionTrigger | null {
  const trigger = textBeforeCursor.match(/@([^\s@]*)$/);
  if (!trigger) return null;
  const length = trigger[0].length;
  const fromOffset = textBeforeCursor.length - length;
  const preceding = fromOffset > 0 ? textBeforeCursor[fromOffset - 1] : "";
  if (preceding && !PRECEDING_OK.test(preceding)) return null;
  return { query: trigger[1] ?? "", length };
}

export function mentionTriggerDocRange(cursorPos: number, trigger: MentionTrigger) {
  return { from: cursorPos - trigger.length, to: cursorPos };
}

export function mentionLabelForInsert(options: {
  entityName: string;
  selectedText: string;
  keepLabel: boolean;
  requestedLabel?: string;
  requestedCustom?: boolean;
}): MentionLabel {
  const name = options.entityName.trim();
  if (options.requestedCustom === true) {
    const requested = (options.requestedLabel ?? "").trim();
    if (requested && requested !== name) return { text: requested, isCustom: true };
    return { text: name, isCustom: false };
  }
  if (options.requestedCustom === false) return { text: name, isCustom: false };
  if (options.keepLabel) {
    const selected = options.selectedText.trim();
    if (selected.startsWith("@")) return { text: name, isCustom: false };
    if (selected && selected !== name) return { text: selected, isCustom: true };
  }
  return { text: name, isCustom: false };
}

export function mentionRangeStillSelected(
  selection: { from: number; to: number },
  range: { from: number; to: number } | null,
  keepLabel: boolean,
) {
  if (!keepLabel || !range) return false;
  return selection.from === range.from && selection.to === range.to && selection.from !== selection.to;
}
