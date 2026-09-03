<script lang="ts">
import WorkspaceGuide from "./WorkspaceGuide.svelte";
import { dismissGuide, isGuideDismissed } from "./persist.ts";
import type { GuideMode, GuideStep } from "./types.ts";

let {
  guideId,
  stepsFor,
  active = true,
  resumeToken = "",
  hintNonce = 0,
  onPrimary,
}: {
  guideId: string;
  stepsFor: (mode: GuideMode) => GuideStep[];
  active?: boolean;
  resumeToken?: string;
  hintNonce?: number;
  onPrimary?: (step: GuideStep) => void | Promise<void>;
} = $props();

let open = $state(false);
let mode = $state<GuideMode>("tour");
let index = $state(0);
let paused = $state(false);
let booted = $state(false);
let lastHint = $state(0);
const steps = $derived(stepsFor(mode));

function finish() {
  open = false;
  paused = false;
  dismissGuide(guideId);
}

async function handlePrimary(step: GuideStep) {
  if (step.action === "pause") {
    open = false;
    paused = !isGuideDismissed(guideId);
  }
  await onPrimary?.(step);
}

$effect(() => {
  if (!active) {
    open = false;
    return;
  }
  if (booted) return;
  booted = true;
  if (!isGuideDismissed(guideId)) {
    mode = "tour";
    index = 0;
    open = true;
  }
});

$effect(() => {
  void resumeToken;
  if (!active || !paused || isGuideDismissed(guideId) || !resumeToken) return;
  paused = false;
  mode = "tour";
  index = 0;
  open = true;
});

$effect(() => {
  if (hintNonce <= lastHint) return;
  lastHint = hintNonce;
  if (!active) return;
  paused = false;
  mode = "hint";
  index = 0;
  open = true;
});
</script>

<WorkspaceGuide
  {open}
  {steps}
  stepIndex={index}
  onStepIndex={(next) => (index = next)}
  onDismiss={finish}
  onComplete={finish}
  onPrimary={handlePrimary} />
