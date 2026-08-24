export type QuickOpenCategory = "Results" | "Recent" | "Destinations" | "Create" | "Commands";

export type QuickOpenAction =
  | { kind: "entity"; entityId: string }
  | { kind: "destination"; destination: string }
  | { kind: "create"; templateKey: string }
  | { kind: "command"; command: "template-gallery" | "snapshots" | "settings" | "plugins" };

export interface QuickOpenItem {
  id: string;
  category: QuickOpenCategory;
  label: string;
  description: string;
  keywords?: string[];
  action: QuickOpenAction;
}

const categoryOrder: QuickOpenCategory[] = ["Results", "Recent", "Destinations", "Create", "Commands"];

function normalized(value: string) {
  return value.trim().toLocaleLowerCase();
}

function itemScore(item: QuickOpenItem, query: string) {
  if (!query) return 1;
  const label = normalized(item.label);
  const description = normalized(item.description);
  const keywords = normalized(item.keywords?.join(" ") ?? "");
  if (label === query) return 100;
  if (label.startsWith(query)) return 80;
  if (label.includes(query)) return 60;
  if (keywords.includes(query)) return 40;
  if (description.includes(query)) return 20;
  const terms = query.split(/\s+/).filter(Boolean);
  const haystack = `${label} ${description} ${keywords}`;
  return terms.every((term) => haystack.includes(term)) ? 10 : 0;
}

export function rankQuickOpenItems(items: QuickOpenItem[], query: string, limit = 40) {
  const needle = normalized(query);
  return items
    .map((item, index) => ({ item, index, score: itemScore(item, needle) }))
    .filter(({ score }) => score > 0)
    .sort(
      (left, right) =>
        right.score - left.score ||
        categoryOrder.indexOf(left.item.category) - categoryOrder.indexOf(right.item.category) ||
        left.index - right.index,
    )
    .slice(0, limit)
    .map(({ item }) => item);
}

export function moveQuickOpenIndex(current: number, direction: number, itemCount: number) {
  if (itemCount <= 0) return -1;
  const normalizedCurrent = current < 0 || current >= itemCount ? 0 : current;
  return (normalizedCurrent + direction + itemCount) % itemCount;
}

export function groupQuickOpenItems(items: QuickOpenItem[]) {
  return categoryOrder
    .map((category) => ({ category, items: items.filter((item) => item.category === category) }))
    .filter((group) => group.items.length > 0);
}
