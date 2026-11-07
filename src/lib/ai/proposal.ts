/**
 * Checks an AI proposal before it is inserted into a document. The rewrite and generate flow inserts plain text
 * (inline Markdown at most), so proposals that bring block structure or markup are sent back to the user to edit.
 * Shared by the shell's dialog and the Writing webview's, which must apply the same rule.
 * Returns the message to show, or null when the proposal can be accepted.
 */
export function validateAiProposal(value: string): string | null {
  if (!value.trim()) return "The AI provider returned an empty proposal.";
  if (/(^|\n)\s*(#{1,6}\s|>\s|[-*+]\s|\d+\.\s|```|~~~)/.test(value)) {
    return "The proposal contains block-level Markdown. Edit it to plain text before accepting.";
  }
  if (/<\/?[a-z][^>]*>/i.test(value))
    return "The proposal contains HTML markup. Edit it to plain text before accepting.";
  return null;
}
