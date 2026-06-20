<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-shell";
  import { api } from "../lib/api";
  import { history } from "../lib/stores";
  import type { HistoryEntry } from "../lib/types";

  let loading = true;
  let error: string | null = null;
  let copiedId: string | null = null;
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    loading = true;
    error = null;
    try {
      const list = await api.listHistory();
      history.set(list);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function formatSize(bytes: number): string {
    if (!bytes || bytes <= 0) return "0 MB";
    const mb = bytes / (1024 * 1024);
    if (mb >= 1024) {
      return (mb / 1024).toFixed(2) + " GB";
    }
    return mb.toFixed(1) + " MB";
  }

  function formatDate(iso: string): string {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return iso;
    const pad = (n: number) => String(n).padStart(2, "0");
    return (
      `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
      `${pad(d.getHours())}:${pad(d.getMinutes())}`
    );
  }

  async function copyUrl(entry: HistoryEntry) {
    try {
      await navigator.clipboard.writeText(entry.url);
      copiedId = entry.id;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => {
        copiedId = null;
      }, 1500);
    } catch (e) {
      error = "复制失败：" + String(e);
    }
  }

  async function openUrl(entry: HistoryEntry) {
    try {
      await open(entry.url);
    } catch (e) {
      error = "打开失败：" + String(e);
    }
  }

  async function remove(entry: HistoryEntry) {
    try {
      await api.deleteHistory(entry.id);
      await refresh();
    } catch (e) {
      error = "删除失败：" + String(e);
    }
  }

  async function clearAll() {
    if (!confirm("确定要清空全部历史记录吗？此操作不可撤销。")) return;
    try {
      await api.clearHistory();
      await refresh();
    } catch (e) {
      error = "清空失败：" + String(e);
    }
  }
</script>

<section class="history">
  <header class="head">
    <div class="title-wrap">
      <h2 class="title">历史记录</h2>
      {#if $history.length > 0}
        <span class="count">{$history.length}</span>
      {/if}
    </div>
    <button
      type="button"
      class="clear-btn"
      on:click={clearAll}
      disabled={loading || $history.length === 0}
    >
      清空
    </button>
  </header>

  {#if error}
    <div class="banner error" role="alert">{error}</div>
  {/if}

  {#if loading}
    <div class="empty">
      <p class="empty-text">加载中…</p>
    </div>
  {:else if $history.length === 0}
    <div class="empty">
      <div class="empty-icon">🗂</div>
      <p class="empty-title">暂无历史记录</p>
      <p class="empty-text">完成压制并上传后，结果会自动出现在这里。</p>
    </div>
  {:else}
    <ul class="list">
      {#each $history as entry (entry.id)}
        <li class="card">
          <div class="info">
            <div class="row-1">
              <span class="file" title={entry.fileName}>{entry.fileName}</span>
              <span class="size">{formatSize(entry.sizeBytes)}</span>
            </div>
            <div class="meta">
              <span class="date">{formatDate(entry.createdAt)}</span>
              {#if entry.profileName}
                <span class="dot">·</span>
                <span class="profile">{entry.profileName}</span>
              {/if}
            </div>
            <div class="url" title={entry.url}>{entry.url}</div>
          </div>
          <div class="actions">
            <button
              type="button"
              class="act"
              class:copied={copiedId === entry.id}
              on:click={() => copyUrl(entry)}
            >
              {copiedId === entry.id ? "已复制" : "复制 URL"}
            </button>
            <button type="button" class="act" on:click={() => openUrl(entry)}>
              浏览器打开
            </button>
            <button
              type="button"
              class="act danger"
              on:click={() => remove(entry)}
            >
              删除
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }
  .title-wrap {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .title {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 650;
    color: var(--text);
  }
  .count {
    font-size: 0.78rem;
    color: var(--text-dim);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.1rem 0.55rem;
    line-height: 1.4;
  }

  .clear-btn {
    appearance: none;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-dim);
    padding: 0.42rem 1rem;
    border-radius: 10px;
    font-size: 0.85rem;
    cursor: pointer;
    backdrop-filter: blur(12px);
    transition:
      color 0.15s ease,
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .clear-btn:hover:not(:disabled) {
    color: var(--danger);
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.08);
  }
  .clear-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .banner {
    padding: 0.6rem 0.9rem;
    border-radius: 10px;
    font-size: 0.85rem;
    border: 1px solid var(--border);
  }
  .banner.error {
    color: var(--danger);
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.3);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    text-align: center;
    padding: 3rem 1.5rem;
    background: var(--surface);
    border: 1px dashed var(--border);
    border-radius: 16px;
    backdrop-filter: blur(12px);
  }
  .empty-icon {
    font-size: 2.2rem;
    opacity: 0.7;
    margin-bottom: 0.2rem;
  }
  .empty-title {
    margin: 0;
    font-size: 1rem;
    color: var(--text);
    font-weight: 600;
  }
  .empty-text {
    margin: 0;
    font-size: 0.86rem;
    color: var(--text-dim);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }

  .card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.9rem 1rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    backdrop-filter: blur(14px);
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .card:hover {
    background: var(--surface-hover);
    border-color: rgba(255, 255, 255, 0.14);
  }

  .info {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
  }
  .row-1 {
    display: flex;
    align-items: baseline;
    gap: 0.7rem;
  }
  .file {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .size {
    flex: none;
    font-size: 0.8rem;
    color: var(--accent-cyan);
    font-variant-numeric: tabular-nums;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .dot {
    opacity: 0.5;
  }
  .url {
    font-size: 0.78rem;
    color: var(--text-dim);
    font-family: ui-monospace, "Cascadia Code", monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }

  .actions {
    flex: none;
    display: flex;
    gap: 0.4rem;
  }
  .act {
    appearance: none;
    border: 1px solid var(--border);
    background: var(--bg-2);
    color: var(--text-dim);
    padding: 0.4rem 0.75rem;
    border-radius: 9px;
    font-size: 0.8rem;
    cursor: pointer;
    white-space: nowrap;
    transition:
      color 0.15s ease,
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .act:hover {
    color: var(--text);
    background: var(--surface-hover);
    border-color: rgba(255, 255, 255, 0.18);
  }
  .act.copied {
    color: var(--success);
    border-color: rgba(74, 222, 128, 0.4);
  }
  .act.danger:hover {
    color: var(--danger);
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.08);
  }

  @media (max-width: 640px) {
    .card {
      flex-direction: column;
      align-items: stretch;
    }
    .actions {
      justify-content: flex-end;
    }
  }
</style>
