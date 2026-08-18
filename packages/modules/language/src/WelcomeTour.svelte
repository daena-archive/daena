<script lang="ts">
let {
  onComplete,
  onDismiss,
}: {
  onComplete: () => void;
  onDismiss: () => void;
} = $props();

interface TourStep {
  id: string;
  title: string;
  description: string;
  icon: string;
}

const TOUR_STEPS: TourStep[] = [
  {
    id: "welcome",
    title: "Welcome to Language Studio",
    description: "This is your workspace for creating and documenting fictional languages. Let's take a quick tour to get you started.",
    icon: " ",
  },
  {
    id: "create",
    title: "Create a Language",
    description: "Start by creating a new language from the sidebar. You only need a name to begin - everything else is optional.",
    icon: "✏️",
  },
  {
    id: "overview",
    title: "Overview Pane",
    description: "Add details about your language like its family, writing system, and notes. This information helps organize your project.",
    icon: " ",
  },
  {
    id: "lexicon",
    title: "Build Your Lexicon",
    description: "Add words with their meanings, pronunciations, and examples. Search and filter to find what you need.",
    icon: " ",
  },
  {
    id: "sounds",
    title: "Document Sounds",
    description: "Define the phonemes in your language with IPA symbols and features. Visual charts help you see the full inventory.",
    icon: " ",
  },
  {
    id: "grammar",
    title: "Sketch Grammar",
    description: "Document word order, noun/verb systems, and other grammatical features. Use the starter to begin with the basics.",
    icon: " ️",
  },
  {
    id: "ready",
    title: "You're Ready!",
    description: "That's the basics! You can always access this tour again from the help menu. Happy conlanging!",
    icon: " ",
  },
];

let currentStep = $state(0);
const step = $derived(TOUR_STEPS[currentStep]);
const isFirst = $derived(currentStep === 0);
const isLast = $derived(currentStep === TOUR_STEPS.length - 1);
const progress = $derived(((currentStep + 1) / TOUR_STEPS.length) * 100);

function nextStep() {
  if (currentStep < TOUR_STEPS.length - 1) {
    currentStep++;
  } else {
    onComplete();
  }
}

function prevStep() {
  if (currentStep > 0) {
    currentStep--;
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    onDismiss();
  } else if (event.key === "ArrowRight" || event.key === "Enter") {
    nextStep();
  } else if (event.key === "ArrowLeft") {
    prevStep();
  }
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="tour-backdrop" onkeydown={handleKeydown}>
  <div class="tour-modal" role="dialog" aria-modal="true" aria-label="Welcome tour">
    <div class="tour-progress">
      <div class="tour-progress-bar" style="width: {progress}%"></div>
    </div>
    
    <div class="tour-content">
      <div class="tour-icon">{step.icon}</div>
      <h2 class="tour-title">{step.title}</h2>
      <p class="tour-description">{step.description}</p>
      
      <div class="tour-dots">
        {#each TOUR_STEPS as _, i (i)}
          <button
            type="button"
            class="tour-dot"
            class:active={i === currentStep}
            onclick={() => (currentStep = i)}
            aria-label="Go to step {i + 1}"></button>
        {/each}
      </div>
    </div>

    <div class="tour-actions">
      <div class="tour-actions-left">
        {#if !isFirst}
          <button type="button" class="tour-button secondary" onclick={prevStep}>Back</button>
        {/if}
      </div>
      <div class="tour-actions-right">
        <button type="button" class="tour-button text" onclick={onDismiss}>Skip tour</button>
        <button type="button" class="tour-button primary" onclick={nextStep}>
          {isLast ? "Get started" : "Next"}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
.tour-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  padding: 20px;
}

.tour-modal {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 480px;
  background: var(--surface);
  border-radius: 16px;
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.tour-progress {
  height: 4px;
  background: var(--line);
}

.tour-progress-bar {
  height: 100%;
  background: var(--accent);
  transition: width 0.3s ease;
}

.tour-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 40px 32px 24px;
  text-align: center;
}

.tour-icon {
  font-size: 48px;
  line-height: 1;
}

.tour-title {
  margin: 0;
  font-family: var(--font-display);
  font-size: 24px;
  font-weight: 600;
  color: var(--ink);
}

.tour-description {
  margin: 0;
  font-size: 15px;
  line-height: 1.6;
  color: var(--ink-soft);
  max-width: 360px;
}

.tour-dots {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.tour-dot {
  width: 8px;
  height: 8px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: var(--line);
  cursor: pointer;
  transition: background 0.2s, transform 0.2s;
}

.tour-dot:hover {
  background: var(--ink-faint);
}

.tour-dot.active {
  background: var(--accent);
  transform: scale(1.25);
}

.tour-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--line);
  background: var(--surface-muted);
}

.tour-actions-left,
.tour-actions-right {
  display: flex;
  gap: 8px;
}

.tour-button {
  padding: 8px 16px;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.tour-button.primary {
  background: var(--accent-dark);
  color: #fff;
}

.tour-button.primary:hover {
  filter: brightness(1.06);
}

.tour-button.secondary {
  background: transparent;
  border-color: var(--line);
  color: var(--ink);
}

.tour-button.secondary:hover {
  background: var(--surface);
}

.tour-button.text {
  background: transparent;
  color: var(--ink-soft);
}

.tour-button.text:hover {
  color: var(--ink);
}
</style>
