export interface LanguageTemplate {
  id: string;
  name: string;
  description: string;
  icon: string;
  fields: Record<string, string>;
  starterSystems?: string[];
}

export const LANGUAGE_TEMPLATES: LanguageTemplate[] = [
  {
    id: "minimal",
    name: "Minimal Language",
    description: "Start with just the basics. Perfect for a quick sketch or first conlang.",
    icon: " ",
    fields: {
      status: "draft",
    },
  },
  {
    id: "naturalistic",
    name: "Naturalistic Language",
    description: "Inspired by real-world languages. Includes common grammatical features.",
    icon: " ",
    fields: {
      status: "active",
      family: "Constructed",
      notes: "A naturalistic constructed language with features inspired by real-world languages.",
    },
    starterSystems: ["syntax.basic-word-order", "nouns.number", "verbs.marking-strategy", "verbs.tense"],
  },
  {
    id: "artistic",
    name: "Artistic Language",
    description: "Focused on aesthetics and unique sound systems.",
    icon: " ",
    fields: {
      status: "active",
      notes: "An artistic constructed language focused on aesthetic qualities and unique features.",
    },
    starterSystems: ["syntax.basic-word-order"],
  },
  {
    id: "engineered",
    name: "Engineered Language",
    description: "Logical and precise. Good for philosophical or technical purposes.",
    icon: "⚙️",
    fields: {
      status: "active",
      notes: "An engineered language designed for logical precision and clarity.",
    },
    starterSystems: ["syntax.basic-word-order", "nouns.case"],
  },
  {
    id: "proto-language",
    name: "Proto-Language",
    description: "A starting point for a language family. Evolve into daughter languages.",
    icon: " ",
    fields: {
      status: "draft",
      notes: "A proto-language that will evolve into a family of daughter languages.",
    },
  },
];

export function getTemplate(id: string): LanguageTemplate | undefined {
  return LANGUAGE_TEMPLATES.find((t) => t.id === id);
}
