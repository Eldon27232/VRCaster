<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let options: { value: string; label: string }[] = [];
  export let value: string;

  const dispatch = createEventDispatcher<{ change: string }>();

  function select(v: string) {
    if (v === value) return;
    value = v;
    dispatch("change", v);
  }
</script>

<div class="segmented" role="tablist">
  {#each options as opt (opt.value)}
    <button
      type="button"
      role="tab"
      aria-selected={opt.value === value}
      class="seg"
      class:active={opt.value === value}
      on:click={() => select(opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    backdrop-filter: blur(12px);
  }
  .seg {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--text-dim);
    padding: 0.42rem 0.95rem;
    border-radius: 9px;
    font-size: 0.88rem;
    cursor: pointer;
    white-space: nowrap;
    transition:
      color 0.15s ease,
      background 0.15s ease;
  }
  .seg:hover:not(.active) {
    color: var(--text);
    background: var(--surface-hover);
  }
  .seg.active {
    color: #fff;
    background: var(--accent-grad);
    box-shadow: 0 2px 8px rgba(124, 58, 237, 0.3);
  }
</style>
