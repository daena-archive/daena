import type { GuideMode, GuideStep } from "../../../../src/lib/guides/types.ts";

export const LANGUAGE_GUIDE_ID = "language";

export type LanguagePane = "overview" | "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples";

const createStep: GuideStep = {
  id: "create",
  title: "Create a language",
  body: "Click New in the workspace header, or select a language from the list. A name is enough to start.",
  target: '[data-guide="workspace-new"]',
  waitForTarget: true,
  primaryLabel: "Got it",
  action: "pause",
};

const overviewStep: GuideStep = {
  id: "overview",
  title: "Overview",
  body: "Name, family, and notes live here. Use the tabs for words, sounds, and grammar.",
  target: "#language-tab-overview",
  primaryLabel: "Add a word",
  action: "lexicon",
};

const addWordStep: GuideStep = {
  id: "add-word",
  title: "Add a word",
  body: "Open the word form and give it a lemma and a meaning. The guide will step aside so you can write.",
  target: '[data-guide="language-add-word"]',
  waitForTarget: true,
  primaryLabel: "Done",
  action: "complete",
};

const addSoundStep: GuideStep = {
  id: "add-sound",
  title: "Add a sound",
  body: "Add a phoneme, then fill place or manner so it appears on the charts.",
  target: '[data-guide="language-add-sound"]',
  waitForTarget: true,
  primaryLabel: "Done",
  action: "complete",
};

const writingStep: GuideStep = {
  id: "writing",
  title: "Writing",
  body: "Map sounds to marks here when you are ready to design an orthography.",
  target: "#language-tab-writing",
};

const starterStep: GuideStep = {
  id: "starter",
  title: "Try the starter",
  body: "Start configures real grammar systems. Skip it anytime and edit them yourself.",
  target: '[data-guide="language-grammar-starter"]',
  waitForTarget: true,
  primaryLabel: "Done",
  action: "complete",
};

const grammarHintStep: GuideStep = {
  id: "grammar",
  title: "Grammar",
  body: "Open a system to configure it, or use the starter if it is still offered.",
  target: "#language-tab-grammar",
};

const formsStep: GuideStep = {
  id: "forms",
  title: "Morphology",
  body: "Build paradigms and rules after you have a few words and grammar choices.",
  target: "#language-tab-forms",
};

const samplesStep: GuideStep = {
  id: "samples",
  title: "Samples",
  body: "Analyze a sentence here to test the language in context.",
  target: "#language-tab-samples",
};

export function languageGuideSteps(opts: { hasLanguage: boolean; pane: LanguagePane; mode: GuideMode }): GuideStep[] {
  if (!opts.hasLanguage) return [createStep];
  if (opts.mode === "tour") return [overviewStep, addWordStep];
  switch (opts.pane) {
    case "lexicon":
      return [addWordStep];
    case "sounds":
      return [addSoundStep];
    case "writing":
      return [writingStep];
    case "grammar":
      return [starterStep, grammarHintStep];
    case "forms":
      return [formsStep];
    case "samples":
      return [samplesStep];
    default:
      return [overviewStep];
  }
}
