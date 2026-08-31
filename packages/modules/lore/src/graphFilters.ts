export function isHouseGraphRelation(type: string) {
  return type.startsWith("family_");
}

export function defaultHiddenGraphRelations(types: Iterable<string>) {
  return new Set([...types].filter(isHouseGraphRelation));
}

export function relationTypeLabel(type: string) {
  if (type === "family_parent_of") return "Parent";
  if (type === "family_partner_with") return "Partner";
  if (type === "family_member_of") return "House member";
  return type.split("_").join(" ");
}
