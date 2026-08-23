<script lang="ts">
import IpaInput from "./IpaInput.svelte";
import type { OrthographySound, PhonemeOption } from "./orthography";

let {
  sounds = $bindable([]),
  phonemes,
  label = "Sound(s)",
}: {
  sounds: OrthographySound[];
  phonemes: PhonemeOption[];
  label?: string;
} = $props();

let selectedPhonemeId = $state("");
let adHocIpa = $state("");

const phonemeById = $derived(new Map(phonemes.map((phoneme) => [phoneme.id, phoneme])));

function display(sound: OrthographySound) {
  if (sound.kind === "ipa") return sound.value;
  const phoneme = phonemeById.get(sound.phonemeId);
  return phoneme?.ipa || phoneme?.symbol || sound.symbol || "Unknown sound";
}

function addPhoneme(event: Event) {
  const select = event.currentTarget as HTMLSelectElement;
  const phoneme = phonemeById.get(select.value);
  if (!phoneme) return;
  sounds = [...sounds, { kind: "phoneme", phonemeId: phoneme.id, symbol: phoneme.ipa || phoneme.symbol }];
  selectedPhonemeId = "";
}

function addIpa() {
  const value = adHocIpa.trim();
  if (!value) return;
  sounds = [...sounds, { kind: "ipa", value }];
  adHocIpa = "";
}

function remove(index: number) {
  sounds = sounds.filter((_, itemIndex) => itemIndex !== index);
}

function move(index: number, offset: -1 | 1) {
  const destination = index + offset;
  if (destination < 0 || destination >= sounds.length) return;
  const next = [...sounds];
  [next[index], next[destination]] = [next[destination], next[index]];
  sounds = next;
}
</script>

<div class="sound-editor">
  <span class="sound-label">{label}</span>
  {#if sounds.length > 0}
    <ol class="sound-sequence" aria-label="Ordered sound sequence">
      {#each sounds as sound, index (`${sound.kind}-${sound.kind === "phoneme" ? sound.phonemeId : sound.value}-${index}`)}
        <li class:missing={sound.kind === "phoneme" && !phonemeById.has(sound.phonemeId)}>
          <span class="sound-kind">{sound.kind === "phoneme" ? "Sound" : "IPA"}</span>
          <strong>/{display(sound)}/</strong>
          {#if sound.kind === "phoneme" && !phonemeById.has(sound.phonemeId)}
            <small>Sound removed</small>
          {/if}
          <span class="sound-actions">
            <button
              type="button"
              onclick={() => move(index, -1)}
              disabled={index === 0}
              aria-label={`Move ${display(sound)} earlier`}>↑</button>
            <button
              type="button"
              onclick={() => move(index, 1)}
              disabled={index === sounds.length - 1}
              aria-label={`Move ${display(sound)} later`}>↓</button>
            <button type="button" class="remove" onclick={() => remove(index)} aria-label={`Remove ${display(sound)}`}
              >×</button>
          </span>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="sound-empty">No sounds assigned. This is allowed.</p>
  {/if}

  <div class="sound-add">
    <label>
      <span>Use an existing Sound</span>
      <select bind:value={selectedPhonemeId} onchange={addPhoneme}>
        <option value="">Select a sound…</option>
        {#each phonemes as phoneme (phoneme.id)}
          <option value={phoneme.id}>/{phoneme.ipa || phoneme.symbol}/</option>
        {/each}
      </select>
    </label>
    <div class="sound-ipa">
      <IpaInput label="Or add ad-hoc IPA" bind:value={adHocIpa} placeholder="e.g. ʃ or kʰ" />
      <button type="button" onclick={addIpa} disabled={!adHocIpa.trim()}>Add IPA value</button>
    </div>
  </div>
</div>

<style>
.sound-editor {
  display: grid;
  gap: 7px;
  min-width: 0;
}
.sound-label,
.sound-add label > span {
  color: var(--ink-soft);
  font-size: 11px;
}
.sound-sequence {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.sound-sequence li {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 6px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
.sound-sequence li.missing {
  border-color: #c98779;
}
.sound-kind {
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}
.sound-sequence strong {
  font-size: 13px;
}
.sound-sequence small {
  color: #a14f42;
  font-size: 9px;
}
.sound-actions {
  display: inline-flex;
  gap: 2px;
}
.sound-actions button {
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  cursor: pointer;
}
.sound-actions button.remove {
  color: #a14f42;
}
.sound-actions button:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.sound-empty {
  margin: 0;
  color: var(--ink-faint);
  font-size: 11px;
}
.sound-add {
  display: grid;
  grid-template-columns: minmax(150px, 0.7fr) minmax(230px, 1.3fr);
  gap: 8px;
  align-items: end;
}
.sound-add label {
  display: grid;
  gap: 6px;
}
.sound-add select {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}
.sound-ipa {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 6px;
  align-items: end;
}
.sound-ipa > button {
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--accent-dark);
  cursor: pointer;
}
.sound-ipa > button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
@media (max-width: 720px) {
  .sound-add,
  .sound-ipa {
    grid-template-columns: 1fr;
  }
}
</style>
