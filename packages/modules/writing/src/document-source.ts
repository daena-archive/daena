/**
 * Reads one entity's document through the broker for the Writing webview. Free
 * of DOM and `$lib` imports so scripts/writing-plugin-source.test.mjs can run
 * it against a fake broker; the Markdown normaliser is injected.
 */

export interface WritingDocumentRpc {
  call<T>(method: string, payload: unknown): Promise<T>;
}

interface RawEntity {
  id: string;
  name: string;
  entity_type?: string | null;
  deleted?: boolean;
  revision?: string;
}

interface RawDocument {
  id: string;
  entity_id: string;
  format: string;
  body: string;
  revision: string;
}

export interface WritingDocument {
  entityId: string;
  /** The entity's name, shown as the heading. */
  title: string;
  /** Document text as Markdown. */
  markdown: string;
  /** Revision of the document, or the empty string while the entity has none. */
  revision: string;
}

/** Entity ids that Markdown text links to as `daena://entity/<id>`, in first-seen order. */
export function entityIdsInMarkdown(markdown: string): string[] {
  const ids = new Set<string>();
  for (const match of markdown.matchAll(/daena:\/\/entity\/([^)\s"'<>]+)/g)) {
    try {
      ids.add(decodeURIComponent(match[1]));
    } catch {
      // A malformed escape is not an id.
    }
  }
  return [...ids];
}

/**
 * Loads the entity and its first document. Resolves null when the entity is gone or deleted. The shell reads
 * the same first document (`listDocuments(id)[0]`) when it opens an entry.
 */
export async function loadWritingDocument(
  rpc: WritingDocumentRpc,
  entityId: string,
  normalize: (body: string, format?: string) => string,
): Promise<WritingDocument | null> {
  const [entity, documents] = await Promise.all([
    rpc.call<RawEntity | null>("entity.get", { id: entityId }),
    rpc.call<RawDocument[]>("document.list", { entityId }),
  ]);
  if (!entity || entity.deleted === true) return null;
  const document = documents[0];
  return {
    entityId,
    title: entity.name,
    markdown: normalize(document?.body ?? "", document?.format),
    revision: document?.revision ?? "",
  };
}
