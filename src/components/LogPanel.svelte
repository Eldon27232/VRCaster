<script lang="ts">
  import { afterUpdate, tick } from "svelte";
  import { logs, clearLogs } from "../lib/stores";

  // 折叠状态：默认展开。无 props，状态自管理。
  let collapsed = false;

  // 日志列表容器，用于自动滚到底。
  let listEl: HTMLDivElement | null = null;

  // 仅当用户已基本停在底部时才自动跟随，避免打断向上翻看历史。
  let stickToBottom = true;

  function onScroll() {
    if (!listEl) return;
    const dist = listEl.scrollHeight - listEl.scrollTop - listEl.clientHeight;
    stickToBottom = dist < 24;
  }

  // 切到展开时，下一帧把视图拉到底部。
  async function toggle() {
    collapsed = !collapsed;
    if (!collapsed) {
      stickToBottom = true;
      await tick();
      if (listEl) listEl.scrollTop = listEl.scrollHeight;
    }
  }

  // 新日志到达且处于展开/跟随状态时滚到底。
  afterUpdate(() => {
    if (!collapsed && stickToBottom && listEl) {
      listEl.scrollTop = listEl.scrollHeight;
    }
  });

  const levelLabel: Record<string, string> = {
    info: "信息",
    warn: "警告",
    error: "错误",
  };
</script>

<div class="panel">
  <div class="head">
    <button
      type="button"
      class="head-btn"
      aria-expanded={!collapsed}
      on:click={toggle}
    >
      <span class="chevron" class:collapsed aria-hidden="true">▾</span>
      <span class="title">日志</span>
      <span class="count">{$logs.length}</span>
    </button>
    <button
      type="button"
      class="clear-btn"
      on:click={clearLogs}
      disabled={$logs.length === 0}
    >
      清空
    </button>
  </div>

  {#if !collapsed}
    <div class="list" bind:this={listEl} on:scroll={onScroll}>
      {#if $logs.length === 0}
        <div class="empty">暂无日志</div>
      {:else}
        {#each $logs as log, i (i)}
          <div class="line {log.level}">
            <span class="lvl">{levelLabel[log.level] ?? log.level}</span>
            {#if log.itemId}
              <span class="item-id">[{log.itemId}]</span>
            {/if}
            <span class="msg">{log.message}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-radius: 14px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .head-btn {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--text);
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    cursor: pointer;
    font-size: 1.02rem;
    font-weight: 600;
  }

  .chevron {
    color: var(--text-dim);
    font-size: 0.8rem;
    transition: transform 0.15s ease;
  }
  .chevron.collapsed {
    transform: rotate(-90deg);
  }

  .title {
    font-weight: 600;
  }

  .count {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-dim);
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 8px;
    font-variant-numeric: tabular-nums;
  }

  .clear-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 9px;
    padding: 5px 14px;
    font-size: 0.82rem;
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease,
      opacity 0.15s ease;
  }
  .clear-btn:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.1);
    border-color: var(--danger);
    color: var(--danger);
  }
  .clear-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .list {
    height: 180px;
    overflow-y: auto;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-family: ui-monospace, "Cascadia Code", "Consolas", monospace;
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .empty {
    color: var(--text-dim);
    font-size: 0.82rem;
    margin: auto;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 6px;
    color: var(--text-dim);
    word-break: break-word;
    white-space: pre-wrap;
  }
  .line.info {
    color: var(--text-dim);
  }
  .line.warn {
    color: #fbbf24;
  }
  .line.error {
    color: var(--danger);
  }

  .lvl {
    flex: 0 0 auto;
    font-size: 0.66rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.85;
  }

  .item-id {
    flex: 0 0 auto;
    color: var(--accent-cyan);
    opacity: 0.9;
  }

  .msg {
    flex: 1 1 auto;
    color: inherit;
    min-width: 0;
  }
</style>
