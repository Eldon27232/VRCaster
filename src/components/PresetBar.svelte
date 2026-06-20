<script lang="ts">
  import { onMount } from "svelte";
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import { presets } from "../lib/stores";
  import type { EncodeParams, Preset } from "../lib/types";

  export let params: EncodeParams;

  const dispatch = createEventDispatcher<{ apply: EncodeParams }>();

  let list: Preset[] = [];
  const unsub = presets.subscribe((p) => (list = p));

  let adding = false;
  let newName = "";
  let saving = false;
  let nameInput: HTMLInputElement | undefined;

  async function refresh() {
    presets.set(await api.listPresets());
  }

  onMount(() => {
    refresh();
    return unsub;
  });

  function apply(p: Preset) {
    dispatch("apply", p.params);
  }

  function startAdd() {
    adding = true;
    newName = "";
    // 等待输入框渲染后聚焦
    queueMicrotask(() => nameInput?.focus());
  }

  function cancelAdd() {
    adding = false;
    newName = "";
  }

  async function confirmAdd() {
    const name = newName.trim();
    if (!name || saving) return;
    saving = true;
    try {
      await api.savePreset({ id: crypto.randomUUID(), name, params });
      await refresh();
      adding = false;
      newName = "";
    } finally {
      saving = false;
    }
  }

  async function remove(p: Preset, e: MouseEvent) {
    e.stopPropagation();
    await api.deletePreset(p.id);
    await refresh();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") confirmAdd();
    else if (e.key === "Escape") cancelAdd();
  }
</script>

<div class="preset-bar">
  <span class="label">预设</span>

  <div class="chips">
    {#if list.length === 0}
      <span class="empty">暂无预设</span>
    {/if}
    {#each list as p (p.id)}
      <div class="chip" role="button" tabindex="0" title="应用预设：{p.name}"
        on:click={() => apply(p)}
        on:keydown={(e) => (e.key === "Enter" || e.key === " ") && apply(p)}
      >
        <span class="chip-name">{p.name}</span>
        <button
          type="button"
          class="chip-del"
          title="删除预设"
          aria-label="删除预设 {p.name}"
          on:click={(e) => remove(p, e)}
        >×</button>
      </div>
    {/each}
  </div>

  <div class="actions">
    {#if adding}
      <div class="add-inline">
        <input
          bind:this={nameInput}
          bind:value={newName}
          class="name-input"
          type="text"
          placeholder="预设名"
          maxlength="40"
          on:keydown={onKey}
        />
        <button
          type="button"
          class="btn-save"
          disabled={!newName.trim() || saving}
          on:click={confirmAdd}
        >保存</button>
        <button type="button" class="btn-cancel" on:click={cancelAdd}>取消</button>
      </div>
    {:else}
      <button type="button" class="btn-add" on:click={startAdd}>+ 存为预设</button>
    {/if}
  </div>
</div>

<style>
  .preset-bar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
    padding: 0.5rem 0.7rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    backdrop-filter: blur(12px);
  }

  .label {
    font-size: 0.8rem;
    color: var(--text-dim);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .chips {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    flex: 1 1 auto;
    min-width: 0;
  }

  .empty {
    font-size: 0.8rem;
    color: var(--text-dim);
    opacity: 0.7;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.28rem 0.4rem 0.28rem 0.6rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text);
    font-size: 0.82rem;
    cursor: pointer;
    white-space: nowrap;
    transition:
      border-color 0.15s ease,
      background 0.15s ease,
      transform 0.1s ease;
  }
  .chip:hover {
    background: var(--surface-hover);
    border-color: var(--accent-cyan);
  }
  .chip:active {
    transform: scale(0.97);
  }
  .chip:focus-visible {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 0 2px rgba(6, 182, 212, 0.35);
  }

  .chip-name {
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip-del {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--text-dim);
    width: 1.1rem;
    height: 1.1rem;
    line-height: 1;
    border-radius: 50%;
    font-size: 0.95rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition:
      color 0.15s ease,
      background 0.15s ease;
  }
  .chip-del:hover {
    color: #fff;
    background: var(--danger);
  }

  .actions {
    flex-shrink: 0;
    margin-left: auto;
  }

  .add-inline {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .name-input {
    width: 9rem;
    padding: 0.35rem 0.55rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 9px;
    color: var(--text);
    font-size: 0.82rem;
    outline: none;
    transition: border-color 0.15s ease;
  }
  .name-input::placeholder {
    color: var(--text-dim);
  }
  .name-input:focus {
    border-color: var(--accent-cyan);
  }

  .btn-add,
  .btn-save,
  .btn-cancel {
    appearance: none;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    padding: 0.35rem 0.7rem;
    border-radius: 9px;
    font-size: 0.82rem;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      opacity 0.15s ease;
  }
  .btn-add:hover,
  .btn-cancel:hover {
    background: var(--surface-hover);
    border-color: var(--accent-cyan);
  }

  .btn-save {
    border: none;
    color: #fff;
    background: var(--accent-grad);
    box-shadow: 0 2px 8px rgba(124, 58, 237, 0.3);
  }
  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    box-shadow: none;
  }
</style>
