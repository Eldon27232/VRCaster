<script lang="ts">
  import { get } from "svelte/store";
  import DropZone from "../components/DropZone.svelte";
  import Segmented from "../components/Segmented.svelte";
  import FileCard from "../components/FileCard.svelte";
  import { queue, queueMode, activeProfileId } from "../lib/stores";
  import { api } from "../lib/api";
  import { tr } from "../lib/i18n";
  import type { QueueMode } from "../lib/types";

  const modeOptions = [
    { value: "pipeline", label: tr("modePipeline") },
    { value: "parallelEncodeSerialUpload", label: tr("modeParallelSerial") },
    { value: "parallelAll", label: tr("modeParallelAll") },
  ];

  const modeHints: Record<QueueMode, string> = {
    pipeline: "压制串行满载算力，上传与下个压制重叠满载带宽（推荐）",
    parallelEncodeSerialUpload: "同时压制会抢算力，通常不会更快",
    parallelAll: "跨境多流也突破不了总带宽上限",
  };

  let running = false;
  let runError = "";

  function onModeChange(e: CustomEvent<string>) {
    queueMode.set(e.detail as QueueMode);
  }

  async function start() {
    const items = get(queue);
    if (!items.length || running) return;
    running = true;
    runError = "";
    try {
      await api.runQueue(items, get(queueMode), get(activeProfileId) ?? "");
    } catch (e) {
      runError = `${e}`;
    } finally {
      running = false;
    }
  }
</script>

<div class="main">
  <section class="drop">
    <DropZone />
  </section>

  <section class="list">
    {#if $queue.length === 0}
      <div class="empty">
        <div class="empty-emoji" aria-hidden="true">🎬</div>
        <p class="empty-title">{tr("queue")}为空</p>
        <p class="empty-sub">拖入或选择视频文件，开始你的第一个压制任务</p>
      </div>
    {:else}
      {#each $queue as it (it.id)}
        <FileCard item={it} />
      {/each}
    {/if}
  </section>

  <footer class="toolbar">
    <div class="mode">
      <span class="mode-label">{tr("queueMode")}</span>
      <Segmented options={modeOptions} value={$queueMode} on:change={onModeChange} />
      <span class="mode-hint">{modeHints[$queueMode]}</span>
    </div>
    <div class="actions">
      {#if runError}
        <span class="run-err">{runError}</span>
      {/if}
      <button
        type="button"
        class="start"
        disabled={$queue.length === 0 || running}
        on:click={start}
      >
        {running ? "运行中…" : tr("start")}
      </button>
    </div>
  </footer>
</div>

<style>
  .main {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
    max-width: 980px;
    margin: 0 auto;
    padding: 1.4rem 1.6rem 6.5rem;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    padding: 3rem 1rem;
    border-radius: 14px;
    border: 1px solid var(--border);
    background: var(--surface);
    backdrop-filter: blur(12px);
    text-align: center;
  }
  .empty-emoji {
    font-size: 2.4rem;
    opacity: 0.85;
  }
  .empty-title {
    margin: 0.3rem 0 0;
    font-size: 1.05rem;
    font-weight: 600;
  }
  .empty-sub {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-dim);
  }
  .toolbar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    padding: 0.85rem 1.6rem;
    background: rgba(11, 11, 16, 0.72);
    border-top: 1px solid var(--border);
    backdrop-filter: blur(16px);
  }
  .mode {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex-wrap: wrap;
  }
  .mode-label {
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  .mode-hint {
    font-size: 0.76rem;
    color: var(--text-dim);
    opacity: 0.85;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .run-err {
    font-size: 0.78rem;
    color: var(--danger);
    max-width: 280px;
  }
  .start {
    appearance: none;
    border: none;
    color: #fff;
    background: var(--accent-grad);
    padding: 0.6rem 1.8rem;
    border-radius: 10px;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition:
      transform 0.15s ease,
      opacity 0.15s ease;
  }
  .start:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .start:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
