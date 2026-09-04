import type { GuideMode, GuideStep } from "./types.ts";

export const TIMELINE_GUIDE_ID = "timeline";

const createStep: GuideStep = {
  id: "create",
  title: "Create an event",
  body: "Click New. Date it when you know when it happens.",
  target: '[data-guide="workspace-new"]',
  waitForTarget: true,
  action: "pause",
};

const eventsStep: GuideStep = {
  id: "events",
  title: "Events",
  body: "Events, encounters, and eras live in this list.",
  target: '[data-guide="workspace-view-events"]',
  primaryLabel: "Show Calendars",
  action: "calendars",
};

const calendarsStep: GuideStep = {
  id: "calendars",
  title: "Calendars",
  body: "Calendars are optional ways to name years, months, and seasons.",
  target: '[data-guide="workspace-view-calendars"]',
  primaryLabel: "Show Timeline",
  action: "timeline",
};

const plotStep: GuideStep = {
  id: "timeline",
  title: "Timeline",
  body: "The plot view lays dated items on a line. Open any item to edit it.",
  target: '[data-guide="workspace-view-timeline"]',
  primaryLabel: "Done",
  action: "complete",
};

export function timelineGuideSteps(opts: { hasCollection: boolean; view: string; mode: GuideMode }): GuideStep[] {
  if (!opts.hasCollection && opts.view !== "calendars" && opts.view !== "timeline") return [createStep];
  if (opts.mode === "hint") {
    if (opts.view === "calendars") return [calendarsStep];
    if (opts.view === "timeline") return [plotStep];
    return [eventsStep];
  }
  return [eventsStep, calendarsStep, plotStep];
}
