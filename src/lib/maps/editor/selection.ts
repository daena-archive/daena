export type SelectionState = {
  featureIds: string[];
};

export function emptySelection(): SelectionState {
  return { featureIds: [] };
}

export function selectionFromIds(ids: Iterable<string>): SelectionState {
  const unique = [...new Set(ids)].sort((left, right) => left.localeCompare(right));
  return { featureIds: unique };
}

export function selectionHas(selection: SelectionState, id: string): boolean {
  return selection.featureIds.includes(id);
}

export function toggleSelectionId(selection: SelectionState, id: string): SelectionState {
  if (selection.featureIds.includes(id)) {
    return { featureIds: selection.featureIds.filter((item) => item !== id) };
  }
  return selectionFromIds([...selection.featureIds, id]);
}
