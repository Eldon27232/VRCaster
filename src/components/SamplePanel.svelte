<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api } from "../lib/api";
  import { t } from "../lib/i18n";
  import type {
    MediaInfo,
    EncodeParams,
    SampleSpec,
    SampleResult,
  } from "../lib/types";

  export let media: MediaInfo;
  export let params: EncodeParams;

  const dispatch = createEventDispatcher<{ applyParams: EncodeParams }>();

  // 样片区间：起点(秒) / 时长(秒)。默认 await defaultSampleSpec（正中间 2 分钟）。
  let startSecs = 0;
  let durationSecs = 120;
  let specReady = false;

  let loading = false;
  let result: SampleResult | null = null;
  let errorMsg: string | null = null;
  let previewSrc: string | null = null;
  let previewFailed = false;

  onMount(async () => {
    try {
      const spec = await api.defaultSampleSpec(media.durationSecs);
      startSecs = Math.max(0, Math.round(spec.startSecs));
      durationSecs = Math.max(1, Math.round(spec.durationSecs));
    } catch {
      // 后端不可用时给个合理的本地默认（正中间 2 分钟）。
      durationSecs = Math.min(120, Math.max(1, Math.round(media.durationSecs)));
      startSecs = Math.max(0, Math.round(media.durationSecs / 2 - durationSecs / 2));
    } finally {
      specReady = true;
    }
  });

  // 约束输入：起点不能为负，且 起点+时长 不超过源时长。
  function clampSpec() {
    if (!Number.isFinite(startSecs) || startSecs < 0) startSecs = 0;
    if (!Number.isFinite(durationSecs) || durationSecs < 1) durationSecs = 1;
    const total = media.durationSecs;
    if (startSecs > total) startSecs = Math.floor(total);
    if (startSecs + durationSecs > total) {
      durationSecs = Math.max(1, Math.floor(total - startSecs));
    }
  }

  async function encodeSample() {
    if (loading) return;
    clampSpec();
    loading = true;
    errorMsg = null;
    result = null;
    previewSrc = null;
    previewFailed = false;
    const spec: SampleSpec = { startSecs, durationSecs };
    try {
      result = await api.encodeSample(media, params, spec);
      try {
        previewSrc = convertFileSrc(result.outputPath);
      } catch {
        previewFailed = true;
      }
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function applySuggested() {
    if (result?.fullEstimate.suggested) {
      dispatch("applyParams", result.fullEstimate.suggested);
    }
  }

  // ---- 格式化工具 ----
  function fmtSize(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return "0 MB";
    const mb = bytes / (1024 * 1024);
    if (mb < 1024) return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`;
    return `${(mb / 1024).toFixed(2)} GB`;
  }

  function fmtBitrate(kbps: number): string {
    if (!Number.isFinite(kbps) || kbps <= 0) return "0 kbps";
    if (kbps < 1000) return `${Math.round(kbps)} kbps`;
    return `${(kbps / 1000).toFixed(2)} Mbps`;
  }

  function fmtDuration(secs: number): string {
    if (!Number.isFinite(secs) || secs < 0) secs = 0;
    const s = Math.round(secs);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) return `${h}小时${m}分${sec}秒`;
    if (m > 0) return `${m}分${sec}秒`;
    return `${sec}秒`;
  }
</script>

<div class="panel">
  <div class="panel-head">
    <span class="title">{$t("encodeSample")}</span>
    <span class="hint">用与全片完全相同的设定试压一段，反推全片</span>
  </div>

  <!-- 区间设置 -->
  <div class="spec">
    <label class="field">
      <span class="lbl">起点（秒）</span>
      <input
        type="number"
        min="0"
        max={Math.floor(media.durationSecs)}
        step="1"
        bind:value={startSecs}
        on:change={clampSpec}
        disabled={loading || !specReady}
      />
    </label>
    <label class="field">
      <span class="lbl">时长（秒）</span>
      <input
        type="number"
        min="1"
        step="1"
        bind:value={durationSecs}
        on:change={clampSpec}
        disabled={loading || !specReady}
      />
    </label>
    <div class="range-note">
      区间 {fmtDuration(startSecs)} → {fmtDuration(startSecs + durationSecs)}
      <span class="muted">/ 全片 {fmtDuration(media.durationSecs)}</span>
    </div>
  </div>

  <button class="btn-primary" on:click={encodeSample} disabled={loading || !specReady}>
    {#if loading}
      <span class="spinner" aria-hidden="true"></span>
      压制中…
    {:else}
      {$t("encodeSample")}
    {/if}
  </button>

  {#if errorMsg}
    <div class="error" role="alert">{errorMsg}</div>
  {/if}

  {#if result}
    <!-- 样片实测 -->
    <div class="result">
      <div class="result-head">样片实测</div>
      <div class="stats">
        <div class="stat">
          <span class="stat-v">{fmtSize(result.sizeBytes)}</span>
          <span class="stat-k">实测大小</span>
        </div>
        <div class="stat">
          <span class="stat-v">{fmtBitrate(result.avgBitrateK)}</span>
          <span class="stat-k">平均码率</span>
        </div>
        <div class="stat">
          <span class="stat-v">{fmtDuration(result.elapsedSecs)}</span>
          <span class="stat-k">压制耗时</span>
        </div>
      </div>
    </div>

    <!-- 全片外推（突出） -->
    <div class="estimate">
      <div class="estimate-head">全片外推</div>
      <div class="stats">
        <div class="stat">
          <span class="stat-v big">{fmtSize(result.fullEstimate.sizeBytes)}</span>
          <span class="stat-k">预估大小</span>
        </div>
        <div class="stat">
          <span class="stat-v big">{fmtBitrate(result.fullEstimate.bitrateK)}</span>
          <span class="stat-k">预估码率</span>
        </div>
        <div class="stat">
          <span class="stat-v big">{fmtDuration(result.fullEstimate.elapsedSecs)}</span>
          <span class="stat-k">预估耗时</span>
        </div>
      </div>

      {#if result.fullEstimate.suggested}
        <button class="btn-suggest" on:click={applySuggested}>
          采用建议参数
        </button>
      {/if}
    </div>

    <!-- 预览 -->
    <div class="preview">
      {#if previewSrc && !previewFailed}
        <!-- svelte-ignore a11y-media-has-caption -->
        <video
          controls
          src={previewSrc}
          on:error={() => (previewFailed = true)}
        ></video>
      {:else}
        <div class="preview-fallback">
          <div class="muted">无法内嵌预览，样片已生成于：</div>
          <code class="path">{result.outputPath}</code>
        </div>
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
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }
  .title {
    font-weight: 600;
    font-size: 1.02rem;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.82rem;
  }

  .spec {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .lbl {
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  input[type="number"] {
    width: 110px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    padding: 8px 10px;
    font-size: 0.92rem;
    transition: border-color 0.15s ease, background 0.15s ease;
  }
  input[type="number"]:focus {
    outline: none;
    border-color: var(--accent-cyan);
    background: var(--surface-hover);
  }
  input[type="number"]:disabled {
    opacity: 0.5;
  }
  .range-note {
    font-size: 0.82rem;
    color: var(--text);
    padding-bottom: 8px;
  }
  .muted {
    color: var(--text-dim);
  }

  .btn-primary {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: var(--accent-grad);
    color: #fff;
    border: none;
    border-radius: 10px;
    padding: 9px 18px;
    font-size: 0.92rem;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.15s ease, box-shadow 0.15s ease, opacity 0.15s ease;
  }
  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 6px 18px rgba(124, 58, 237, 0.35);
  }
  .btn-primary:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.4);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: var(--danger);
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 0.85rem;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .result,
  .estimate {
    border-radius: 12px;
    padding: 12px 14px;
  }
  .result {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border);
  }
  /* 全片外推突出：渐变描边感 + 强调底色 */
  .estimate {
    background:
      linear-gradient(135deg, rgba(124, 58, 237, 0.12), rgba(6, 182, 212, 0.1));
    border: 1px solid rgba(124, 58, 237, 0.45);
  }

  .result-head,
  .estimate-head {
    font-size: 0.8rem;
    color: var(--text-dim);
    margin-bottom: 10px;
    letter-spacing: 0.02em;
  }
  .estimate-head {
    color: var(--text);
    font-weight: 600;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .stat-v {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 1rem;
  }
  .stat-v.big {
    font-size: 1.22rem;
    background: var(--accent-grad);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }
  .stat-k {
    font-size: 0.76rem;
    color: var(--text-dim);
  }

  .btn-suggest {
    margin-top: 12px;
    background: transparent;
    border: 1px solid var(--accent-cyan);
    color: var(--text);
    border-radius: 10px;
    padding: 8px 16px;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease, transform 0.15s ease;
  }
  .btn-suggest:hover {
    background: var(--surface-hover);
    transform: translateY(-1px);
  }

  .preview video {
    width: 100%;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: #000;
    display: block;
  }
  .preview-fallback {
    border: 1px dashed var(--border);
    border-radius: 12px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .path {
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 0.82rem;
    color: var(--text);
    word-break: break-all;
    background: var(--bg-2);
    padding: 6px 8px;
    border-radius: 8px;
  }
</style>
