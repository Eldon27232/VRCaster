<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { t } from "../lib/i18n";
  import type {
    MediaInfo,
    EncodeParams,
    Resolution,
    Speed,
    Tonemap,
    RateMode,
    SubtitleChoice,
    AudioTrack,
  } from "../lib/types";
  import Segmented from "./Segmented.svelte";

  export let media: MediaInfo;
  export let params: EncodeParams;

  const dispatch = createEventDispatcher<{ change: EncodeParams }>();

  // 任一改动都构造完整的新 EncodeParams 并派发
  function emit(patch: Partial<EncodeParams>) {
    const next: EncodeParams = { ...params, ...patch };
    params = next;
    dispatch("change", next);
  }

  // —— 分辨率 ——
  const resolutionOptions: { value: Resolution; label: string }[] = [
    { value: "p720", label: "720p" },
    { value: "p1080", label: "1080p" },
    { value: "p1440", label: "1440p" },
    { value: "p2160", label: "4K" },
  ];
  function onResolution(e: CustomEvent<string>) {
    emit({ resolution: e.detail as Resolution });
  }

  // —— 码率控制：目标大小 ↔ 质量档 ——
  // 切换 kind 时，保留另一种模式上次的取值（记忆），无记忆则给默认。
  let lastGb = 4;
  let lastCrf = 23;
  $: if (params.rateMode.kind === "targetSize") lastGb = params.rateMode.gb;
  $: if (params.rateMode.kind === "quality") lastCrf = params.rateMode.crf;

  const rateModeKindOptions = [
    { value: "targetSize", label: $t("targetSize") },
    { value: "quality", label: $t("quality") },
  ];
  function onRateModeKind(e: CustomEvent<string>) {
    const kind = e.detail;
    if (kind === "targetSize") {
      emit({ rateMode: { kind: "targetSize", gb: lastGb } });
    } else {
      emit({ rateMode: { kind: "quality", crf: lastCrf } });
    }
  }
  function onGbInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (Number.isFinite(v) && v > 0) {
      emit({ rateMode: { kind: "targetSize", gb: v } });
    }
  }
  // 质量档：低/中/高画质 对应 CRF 高/中/低（数值越低画质越好）
  const crfOptions = [
    { value: "28", label: "低" },
    { value: "23", label: "中" },
    { value: "18", label: "高" },
  ];
  $: crfSegValue =
    params.rateMode.kind === "quality" ? String(params.rateMode.crf) : "23";
  function onCrf(e: CustomEvent<string>) {
    emit({ rateMode: { kind: "quality", crf: parseInt(e.detail, 10) } });
  }

  // —— 编码速度 ——
  const speedOptions: { value: Speed; label: string }[] = [
    { value: "fast", label: $t("fast") },
    { value: "medium", label: $t("medium") },
    { value: "slow", label: $t("slow") },
  ];
  function onSpeed(e: CustomEvent<string>) {
    emit({ speed: e.detail as Speed });
  }

  // —— HDR tonemap（仅 isHdr 时显示）——
  const tonemapOptions: { value: Tonemap; label: string }[] = [
    { value: "hable", label: "hable" },
    { value: "mobius", label: "mobius" },
    { value: "reinhard", label: "reinhard" },
  ];
  function onTonemap(e: CustomEvent<string>) {
    emit({ tonemap: e.detail as Tonemap });
  }

  // —— 音轨 ——
  function audioLabel(tr: AudioTrack): string {
    const parts: string[] = [`#${tr.index}`];
    const name = tr.title || tr.language;
    if (name) parts.push(name);
    parts.push(`${tr.channels}ch`);
    if (tr.codec) parts.push(tr.codec);
    return parts.join(" · ");
  }
  function onAudioTrack(e: Event) {
    const idx = parseInt((e.target as HTMLSelectElement).value, 10);
    if (Number.isFinite(idx)) emit({ audioTrackIndex: idx });
  }

  // —— 音频码率 ——
  const audioBitrateOptions = [
    { value: "128", label: "128k" },
    { value: "192", label: "192k" },
    { value: "256", label: "256k" },
  ];
  function onAudioBitrate(e: CustomEvent<string>) {
    emit({ audioBitrateK: parseInt(e.detail, 10) });
  }

  // —— 字幕 ——
  // select 值编码：none | emb:<index> | ext
  $: subtitleSelectValue = (() => {
    const s = params.subtitle;
    if (s.kind === "embedded") return `emb:${s.index}`;
    if (s.kind === "external") return "ext";
    return "none";
  })();

  async function onSubtitleSelect(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    if (val === "none") {
      emit({ subtitle: { kind: "none" } });
      return;
    }
    if (val.startsWith("emb:")) {
      const idx = parseInt(val.slice(4), 10);
      emit({ subtitle: { kind: "embedded", index: idx } });
      return;
    }
    if (val === "ext") {
      // 打开文件选择；取消则恢复原选项（select 已变，需还原）
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "Subtitle", extensions: ["ass", "srt", "ssa", "vtt", "sub"] },
        ],
      });
      if (typeof picked === "string") {
        emit({ subtitle: { kind: "external", path: picked } });
      } else {
        // 取消选择：强制刷新 select 回到当前 params 对应值
        params = { ...params };
      }
    }
  }

  // 外挂文件名（仅显示尾部）
  $: externalName =
    params.subtitle.kind === "external"
      ? params.subtitle.path.split(/[\\/]/).pop() ?? params.subtitle.path
      : "";
</script>

<div class="params">
  <!-- 分辨率 -->
  <div class="field">
    <span class="lbl">{$t("resolution")}</span>
    <Segmented
      options={resolutionOptions}
      value={params.resolution}
      on:change={onResolution}
    />
    {#if params.resolution === "p2160"}
      <span class="hint warn">部分客户端可能卡</span>
    {/if}
  </div>

  <!-- 码率控制 -->
  <div class="field">
    <span class="lbl">{$t("rateMode")}</span>
    <Segmented
      options={rateModeKindOptions}
      value={params.rateMode.kind}
      on:change={onRateModeKind}
    />
    <div class="rate-detail">
      {#if params.rateMode.kind === "targetSize"}
        <div class="num-input">
          <input
            type="number"
            min="0.1"
            step="0.1"
            value={params.rateMode.gb}
            on:input={onGbInput}
          />
          <span class="unit">GB</span>
        </div>
      {:else}
        <Segmented options={crfOptions} value={crfSegValue} on:change={onCrf} />
      {/if}
    </div>
  </div>

  <!-- 编码速度 -->
  <div class="field">
    <span class="lbl">{$t("speed")}</span>
    <Segmented options={speedOptions} value={params.speed} on:change={onSpeed} />
  </div>

  <!-- HDR tonemap：仅 HDR 源 -->
  {#if media.isHdr}
    <div class="field">
      <span class="lbl">{$t("tonemap")}</span>
      <Segmented
        options={tonemapOptions}
        value={params.tonemap}
        on:change={onTonemap}
      />
    </div>
  {/if}

  <!-- 音轨 + 音频码率 -->
  <div class="field row">
    <div class="sub">
      <span class="lbl">{$t("audioTrack")}</span>
      {#if media.audioTracks.length > 0}
        <select
          class="select"
          value={String(params.audioTrackIndex)}
          on:change={onAudioTrack}
        >
          {#each media.audioTracks as tr (tr.index)}
            <option value={String(tr.index)}>{audioLabel(tr)}</option>
          {/each}
        </select>
      {:else}
        <span class="hint">无音轨</span>
      {/if}
    </div>
    <div class="sub">
      <span class="lbl">音频码率</span>
      <Segmented
        options={audioBitrateOptions}
        value={String(params.audioBitrateK)}
        on:change={onAudioBitrate}
      />
    </div>
  </div>

  <!-- 字幕 -->
  <div class="field">
    <span class="lbl">{$t("subtitle")}</span>
    <select
      class="select"
      value={subtitleSelectValue}
      on:change={onSubtitleSelect}
    >
      <option value="none">{$t("none")}</option>
      {#each media.subtitleTracks as st (st.index)}
        <option value={`emb:${st.index}`}>
          #{st.index}
          {#if st.title || st.language}· {st.title || st.language}{/if}
          {#if st.codec}· {st.codec}{/if}
        </option>
      {/each}
      <option value="ext">{$t("external")}…</option>
    </select>
    {#if params.subtitle.kind === "external"}
      <span class="hint" title={params.subtitle.path}>{externalName}</span>
    {/if}
  </div>
</div>

<style>
  .params {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .field.row {
    flex-direction: row;
    gap: 18px;
    flex-wrap: wrap;
  }

  .field.row .sub {
    display: flex;
    flex-direction: column;
    gap: 7px;
    min-width: 0;
    flex: 1 1 200px;
  }

  .lbl {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-dim);
    text-transform: uppercase;
  }

  .rate-detail {
    margin-top: 2px;
  }

  .num-input {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0 12px;
    width: max-content;
    transition: border-color 0.15s ease;
  }
  .num-input:focus-within {
    border-color: var(--accent-purple);
  }
  .num-input input {
    background: transparent;
    border: none;
    outline: none;
    color: var(--text);
    font-size: 15px;
    width: 72px;
    padding: 9px 0;
  }
  .num-input input::-webkit-outer-spin-button,
  .num-input input::-webkit-inner-spin-button {
    opacity: 0.5;
  }
  .num-input .unit {
    color: var(--text-dim);
    font-size: 13px;
  }

  .select {
    appearance: none;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 10px;
    padding: 9px 32px 9px 12px;
    font-size: 14px;
    cursor: pointer;
    max-width: 100%;
    background-image: linear-gradient(45deg, transparent 50%, var(--text-dim) 50%),
      linear-gradient(135deg, var(--text-dim) 50%, transparent 50%);
    background-position:
      calc(100% - 16px) calc(50% - 2px),
      calc(100% - 11px) calc(50% - 2px);
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
    transition: border-color 0.15s ease;
  }
  .select:hover {
    border-color: var(--surface-hover);
  }
  .select:focus {
    outline: none;
    border-color: var(--accent-purple);
  }

  .hint {
    font-size: 12px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  .hint.warn {
    color: #f59e0b;
  }
</style>
