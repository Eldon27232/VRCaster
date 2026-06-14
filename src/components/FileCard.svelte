<script lang="ts">
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import { t } from "../lib/i18n";
  import { api } from "../lib/api";
  import { updateItem, removeItem, moveItem } from "../lib/stores";
  import type { QueueItem, EncodeParams } from "../lib/types";
  import ParamControls from "./ParamControls.svelte";
  import SamplePanel from "./SamplePanel.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  export let item: QueueItem;

  // —— 格式化 ——
  function fmtDuration(secs: number): string {
    if (!Number.isFinite(secs) || secs < 0) return "--:--";
    const s = Math.floor(secs % 60);
    const m = Math.floor((secs / 60) % 60);
    const h = Math.floor(secs / 3600);
    const pad = (n: number) => String(n).padStart(2, "0");
    return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
  }

  function fmtSize(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let v = bytes;
    let i = 0;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(v < 10 && i > 0 ? 2 : i > 0 ? 1 : 0)} ${units[i]}`;
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  // —— 子组件事件 ——
  function onParamsChange(e: CustomEvent<EncodeParams>) {
    updateItem(item.id, { params: e.detail });
  }

  function onApplyParams(e: CustomEvent<EncodeParams>) {
    updateItem(item.id, { params: e.detail });
  }

  // —— 动作：压全片（进度走 "progress" 事件，由上层订阅写回 store）——
  async function encodeFull() {
    updateItem(item.id, { status: "encoding", encodeProgress: 0, error: null });
    try {
      await api.startEncode(item.id, item.media, item.params);
      updateItem(item.id, { status: "encoded", encodeProgress: 100 });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      updateItem(item.id, { status: "error", error: msg });
    }
  }

  // —— 完成态 ——
  let copied = false;
  async function copyUrl() {
    if (!item.resultUrl) return;
    try {
      await navigator.clipboard.writeText(item.resultUrl);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      copied = false;
    }
  }

  async function openInBrowser() {
    if (!item.resultUrl) return;
    try {
      await openShell(item.resultUrl);
    } catch {
      /* 静默：打开失败不阻断 */
    }
  }

  // —— 状态徽章 ——
  $: statusText =
    item.status === "encoding"
      ? $t("encoding")
      : item.status === "uploading"
        ? $t("uploading")
        : item.status === "done"
          ? $t("done")
          : item.status === "error"
            ? $t("error")
            : item.status === "encoded"
              ? "已压制"
              : "等待中";

  $: busy = item.status === "encoding" || item.status === "uploading";
</script>

<div class="card" class:error={item.status === "error"}>
  <!-- 头部：文件名 + 状态 + 移动/移除 -->
  <header class="head">
    <div class="title-wrap">
      <span class="name" title={item.media.path}>{fileName(item.media.path)}</span
      >
      <span class="status status-{item.status}">{statusText}</span>
    </div>
    <div class="head-actions">
      <button
        class="icon-btn"
        title="上移"
        on:click={() => moveItem(item.id, -1)}
        disabled={busy}>↑</button
      >
      <button
        class="icon-btn"
        title="下移"
        on:click={() => moveItem(item.id, 1)}
        disabled={busy}>↓</button
      >
      <button
        class="icon-btn danger"
        title="移除"
        on:click={() => removeItem(item.id)}
        disabled={busy}>✕</button
      >
    </div>
  </header>

  <!-- 源信息标签行 -->
  <div class="tags">
    <span class="tag">{item.media.width}×{item.media.height}</span>
    <span class="tag">{fmtDuration(item.media.durationSecs)}</span>
    {#if item.media.isHdr}
      <span class="tag hdr">HDR</span>
    {/if}
    <span class="tag">🔊 {item.media.audioTracks.length}</span>
    <span class="tag">💬 {item.media.subtitleTracks.length}</span>
    <span class="tag">{fmtSize(item.media.sizeBytes)}</span>
  </div>

  <!-- 参数控件 -->
  <div class="section">
    <ParamControls
      media={item.media}
      params={item.params}
      on:change={onParamsChange}
    />
  </div>

  <!-- 样片面板 -->
  <div class="section">
    <SamplePanel
      media={item.media}
      params={item.params}
      on:applyParams={onApplyParams}
    />
  </div>

  <!-- 进度（压制 / 上传）-->
  {#if item.status === "encoding"}
    <ProgressBar
      percent={item.encodeProgress}
      label={$t("encoding")}
      active={true}
    />
  {:else if item.status === "uploading"}
    <ProgressBar
      percent={item.uploadProgress}
      label={$t("uploading")}
      active={true}
    />
  {/if}

  <!-- 完成态：URL + 复制 + 浏览器打开 -->
  {#if item.status === "done" && item.resultUrl}
    <div class="result">
      <span class="result-url" title={item.resultUrl}>{item.resultUrl}</span>
      <div class="result-actions">
        <button class="btn-secondary" on:click={copyUrl}>
          {copied ? "已复制" : $t("copyUrl")}
        </button>
        <button class="btn-secondary" on:click={openInBrowser}
          >{$t("openBrowser")}</button
        >
      </div>
    </div>
  {/if}

  <!-- 错误态 -->
  {#if item.status === "error" && item.error}
    <div class="err-box">{item.error}</div>
  {/if}

  <!-- 主动作 -->
  <footer class="actions">
    <button class="btn-secondary" on:click={encodeFull} disabled={busy}>
      {$t("encodeFull")}
    </button>
  </footer>
</div>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-radius: 14px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    transition:
      border-color 0.15s ease,
      box-shadow 0.15s ease;
  }
  .card.error {
    border-color: rgba(239, 68, 68, 0.45);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .title-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }
  .name {
    font-weight: 600;
    font-size: 15px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--bg-2);
    color: var(--text-dim);
    border: 1px solid var(--border);
  }
  .status-encoding,
  .status-uploading {
    color: var(--accent-cyan);
    border-color: rgba(6, 182, 212, 0.4);
  }
  .status-done {
    color: var(--success);
    border-color: rgba(74, 222, 128, 0.4);
  }
  .status-error {
    color: var(--danger);
    border-color: rgba(239, 68, 68, 0.4);
  }
  .status-encoded {
    color: var(--accent-purple);
    border-color: rgba(124, 58, 237, 0.4);
  }

  .head-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    line-height: 1;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .icon-btn.danger:hover:not(:disabled) {
    color: var(--danger);
    border-color: rgba(239, 68, 68, 0.4);
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }
  .tag {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 8px;
    background: var(--bg-2);
    color: var(--text-dim);
    border: 1px solid var(--border);
    white-space: nowrap;
  }
  .tag.hdr {
    color: #fff;
    background: var(--accent-grad);
    border-color: transparent;
    font-weight: 600;
  }

  .section {
    border-top: 1px solid var(--border);
    padding-top: 16px;
  }

  .result {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 12px;
  }
  .result-url {
    font-size: 13px;
    color: var(--accent-cyan);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .result-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .err-box {
    font-size: 13px;
    color: var(--danger);
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 10px;
    padding: 10px 12px;
    word-break: break-word;
    white-space: pre-wrap;
    max-height: 160px;
    overflow: auto;
  }

  .actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 10px;
    padding: 9px 16px;
    font-size: 14px;
    cursor: pointer;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      transform 0.15s ease;
  }
  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: var(--surface-hover);
  }
  .btn-secondary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
