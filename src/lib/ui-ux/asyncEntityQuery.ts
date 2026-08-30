/**
 * Shared helpers for backend-paged entity search (Slice 2).
 * Pickers must not filter a full in-memory entity list.
 */

export type AsyncEntitySortField = "name" | "created_at" | "updated_at" | "createdAt" | "updatedAt" | "relevance";
export type AsyncEntitySortDirection = "asc" | "desc";

export type AsyncEntityOption = {
  id: string;
  name: string;
  entityType?: string | null;
  deleted?: boolean;
  revision?: string;
};

export type AsyncEntitySearchQuery = {
  text: string;
  offset: number;
  limit: number;
  entityTypes?: string[];
  excludedEntityTypes?: string[];
  excludeIds?: string[];
  sortField?: AsyncEntitySortField;
  sortDirection?: AsyncEntitySortDirection;
};

export type AsyncEntitySearchPage = {
  items: AsyncEntityOption[];
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
};

export type AsyncEntitySearchFn = (query: AsyncEntitySearchQuery) => Promise<AsyncEntitySearchPage>;

export type AsyncEntityResolveFn = (ids: string[]) => Promise<AsyncEntityOption[]>;

/** Monotonic request tokens so slower responses cannot overwrite newer results. */
export function createRequestGate() {
  let token = 0;
  return {
    next() {
      token += 1;
      return token;
    },
    isCurrent(request: number) {
      return request === token;
    },
    get current() {
      return token;
    },
  };
}

export function filterExcludedOptions<T extends { id: string; deleted?: boolean }>(
  items: T[],
  excludeIds: Iterable<string> = [],
): T[] {
  const excluded = new Set(excludeIds);
  return items.filter((item) => !item.deleted && !excluded.has(item.id));
}

export function emptyAsyncEntityPage(limit = 20): AsyncEntitySearchPage {
  return { items: [], total: 0, offset: 0, limit, hasMore: false };
}

/**
 * Normalize shell EntityPage / module EntityPage shapes into the picker page contract.
 * Prefer the server's has_more/hasMore flag; only fall back using the unfiltered page size.
 */
export function toAsyncEntityPage(
  page: {
    items: Array<{
      id: string;
      name: string;
      entity_type?: string | null;
      type?: string | null;
      deleted?: boolean;
      revision?: string;
    }>;
    total: number;
    offset: number;
    limit: number;
    has_more?: boolean;
    hasMore?: boolean;
  },
  options?: { excludeIds?: Iterable<string> },
): AsyncEntitySearchPage {
  const serverCount = page.items.length;
  const items = filterExcludedOptions(
    page.items.map((item) => ({
      id: item.id,
      name: item.name,
      entityType: item.entity_type ?? item.type ?? null,
      deleted: Boolean(item.deleted),
      revision: item.revision,
    })),
    options?.excludeIds,
  );
  const serverHasMore = page.has_more ?? page.hasMore;
  const hasMore = serverHasMore ?? page.offset + serverCount < page.total;
  return {
    items,
    total: page.total,
    offset: page.offset,
    limit: page.limit,
    hasMore,
  };
}

/** Map picker sort fields onto the shell/module query vocabulary. */
export function toShellSortField(field?: AsyncEntitySortField): "name" | "created_at" | "updated_at" | "relevance" {
  if (field === "createdAt" || field === "created_at") return "created_at";
  if (field === "updatedAt" || field === "updated_at") return "updated_at";
  if (field === "relevance") return "relevance";
  return "name";
}

export function toShellSortDirection(direction?: AsyncEntitySortDirection): "asc" | "desc" {
  return direction === "desc" ? "desc" : "asc";
}

export async function runAsyncEntitySearch(
  gate: ReturnType<typeof createRequestGate>,
  search: AsyncEntitySearchFn,
  query: AsyncEntitySearchQuery,
): Promise<
  | { request: number; page: AsyncEntitySearchPage }
  | { request: number; stale: true }
  | { request: number; error: unknown }
> {
  const request = gate.next();
  try {
    const page = await search(query);
    if (!gate.isCurrent(request)) return { request, stale: true };
    return {
      request,
      page: {
        ...page,
        items: filterExcludedOptions(page.items, query.excludeIds),
      },
    };
  } catch (error) {
    if (!gate.isCurrent(request)) return { request, stale: true };
    return { request, error };
  }
}
