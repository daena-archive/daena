/**
 * True while Svelte runs a reactive update (an effect, a template block, or a derived),
 * for example when removing a DOM subtree blurs a focused editor. False inside plain
 * DOM event handlers, where Svelte has no active reaction.
 */
export function insideReactiveUpdate(): boolean {
  return $effect.tracking();
}
