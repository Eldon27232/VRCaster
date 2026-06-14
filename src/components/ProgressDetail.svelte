<script lang="ts">
  import type { ProgressEvent } from "../lib/types";
  import ProgressBar from "./ProgressBar.svelte";
  import { tr } from "../lib/i18n";

  // event：后端最新一帧详细进度；percent/label 为无 event 时的回退。
  export let event: ProgressEvent | undefined = undefined;
  export let label: string = "";
  export let percent: number = 0;

  $: pct = event?.percent ?? percent;
  $: stage = event?.stage ?? "encode";

  function fmtTime(secs: number | null | undefined): string {
    if (secs == null || !Number.isFinite(secs) || secs < 0) return "--:--";
    const s = Math.floor(secs % 60);
    const m = Math.floor((secs / 60) % 60);
    const h = Math.floor(secs / 3600);
    const pad = (n: number) => String(n).padStart(2, "0");
    return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
  }

  function fmtSize(bytes: number | null | undefined): string {
    if (bytes == null || !Number.isFinite(bytes) || bytes <= 0) return "—";
    const u = ["B", "KB", "MB", "GB", "TB"];
    let v = bytes;
    let i = 0;
    while (v >= 1024 && i < u.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(i > 1 ? 2 : i > 0 ? 1 : 0)} ${u[i]}`;
  }

  // ffmpeg 形如 "22782.2kbits/s" → "22.8 Mbps"
  function fmtBitrate(b: string | null | undefined): string {
    if (!b) return "—";
    const m = b.match(/([\d.]+)\s*kbits\/s/i);
    if (m) {
      const k = parseFloat(m[1]);
      return k >= 1000 ? `${(k / 1000).toFixed(1)} Mbps` : `${k.toFixed(0)} kbps`;
    }
    return b;
  }

  function fmtNum(n: number | null | undefined): string {
    return n == null ? "—" : Math.round(n).toLocaleString();
  }

  $: cells =
    stage === "upload"
      ? [
          {
            k: tr("sent"),
            v: `${fmtSize(event?.transferred)} / ${fmtSize(event?.totalSize)}`,
          },
          { k: tr("speedLabel"), v: event?.speed || "—" },
          { k: tr("remaining"), v: fmtTime(event?.etaSecs) },
        ]
      : [
          {
            k: tr("frame"),
            v: event?.totalFrames
              ? `${fmtNum(event?.frame)} / ${fmtNum(event?.totalFrames)}`
              : fmtNum(event?.frame),
          },
          { k: tr("fpsLabel"), v: event?.fps != null ? event.fps.toFixed(1) : "—" },
          { k: tr("speedLabel"), v: event?.speed || "—" },
          { k: tr("bitrateLabel"), v: fmtBitrate(event?.bitrate) },
          {
            k: tr("encoded"),
            v: `${fmtTime(event?.outTimeSecs)} / ${fmtTime(event?.totalSecs)}`,
          },
          { k: tr("output"), v: fmtSize(event?.curSize) },
          { k: tr("remaining"), v: fmtTime(event?.etaSecs) },
          { k: tr("qualityQ"), v: event?.q != null ? event.q.toFixed(1) : "—" },
        ];
</script>

<div class="detail">
  <div class="bar-row">
    <ProgressBar percent={pct} {label} active={true} />
  </div>
  <div class="grid" class:upload={stage === "upload"}>
    {#each cells as c}
      <div class="cell">
        <span class="k">{c.k}</span>
        <span class="v">{c.v}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .grid.upload {
    grid-template-columns: repeat(3, 1fr);
  }
  .cell {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 8px 10px;
    border-radius: 9px;
    background: var(--surface);
    border: 1px solid var(--border);
    min-width: 0;
  }
  .k {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-dim);
  }
  .v {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  @media (max-width: 560px) {
    .grid,
    .grid.upload {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
