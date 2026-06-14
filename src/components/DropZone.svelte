<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { api } from "../lib/api";
  import { settings, queue } from "../lib/stores";
  import { tr } from "../lib/i18n";
  import type {
    MediaInfo,
    EncodeParams,
    QueueItem,
    SubtitleChoice,
  } from "../lib/types";

  let dragging = false;
  let busy = false;
  let errorMsg = "";

  const VIDEO_EXT = [
    "mp4", "mkv", "mov", "avi", "webm", "flv", "m4v", "ts", "m2ts", "wmv", "mpg", "mpeg",
  ];

  function isVideo(path: string): boolean {
    const dot = path.lastIndexOf(".");
    if (dot < 0) return false;
    return VIDEO_EXT.includes(path.slice(dot + 1).toLowerCase());
  }

  // 用 settings 默认值 + 媒体分析结果构造 EncodeParams
  function buildParams(media: MediaInfo): EncodeParams {
    const s = get(settings);
    const audioTrackIndex = media.audioTracks.length
      ? media.audioTracks[0].index
      : 0;
    const subtitle: SubtitleChoice = media.subtitleTracks.length
      ? { kind: "embedded", index: media.subtitleTracks[0].index }
      : { kind: "none" };
    return {
      resolution: s.defaultResolution,
      rateMode: { kind: "targetSize", gb: 8 },
      speed: s.defaultSpeed,
      hwaccel: true,
      tonemap: s.defaultTonemap,
      audioTrackIndex,
      audioBitrateK: s.defaultAudioBitrateK,
      subtitle,
    };
  }

  async function addPaths(paths: string[]) {
    const videos = paths.filter(isVideo);
    if (!videos.length) return;
    busy = true;
    errorMsg = "";
    for (const path of videos) {
      try {
        const media = await api.analyzeMedia(path);
        const item: QueueItem = {
          id: crypto.randomUUID(),
          media,
          params: buildParams(media),
          status: "pending",
          encodeProgress: 0,
          uploadProgress: 0,
          resultUrl: null,
          error: null,
        };
        queue.update((items) => [...items, item]);
      } catch (e) {
        errorMsg = `${path.split(/[\\/]/).pop()}：${e}`;
      }
    }
    busy = false;
  }

  async function pick() {
    if (busy) return;
    const selected = await open({
      multiple: true,
      filters: [{ name: tr("video") || "视频", extensions: VIDEO_EXT }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    await addPaths(paths);
  }

  // Tauri webview 文件拖放事件
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    try {
      unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        const t = event.payload.type;
        if (t === "over" || t === "enter") {
          dragging = true;
        } else if (t === "drop") {
          dragging = false;
          const paths = (event.payload as { paths?: string[] }).paths ?? [];
          void addPaths(paths);
        } else {
          dragging = false;
        }
      });
    } catch {
      // 非 Tauri 环境（如纯浏览器预览）忽略
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });
</script>

<button
  type="button"
  class="dropzone"
  class:dragging
  class:busy
  on:click={pick}
  aria-busy={busy}
>
  <div class="icon" aria-hidden="true">
    <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12 16V4" />
      <path d="m7 9 5-5 5 5" />
      <path d="M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
    </svg>
  </div>
  <div class="hint">{tr("dropHint")}</div>
  {#if busy}
    <div class="sub">分析中…</div>
  {:else}
    <div class="sub">支持 MP4 / MKV / MOV 等常见格式</div>
  {/if}
  {#if errorMsg}
    <div class="err">{errorMsg}</div>
  {/if}
</button>

<style>
  .dropzone {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.55rem;
    padding: 2.6rem 1.5rem;
    border-radius: 14px;
    border: 1.5px dashed var(--border);
    background: var(--surface);
    backdrop-filter: blur(12px);
    color: var(--text);
    cursor: pointer;
    text-align: center;
    transition:
      border-color 0.15s ease,
      background 0.15s ease,
      transform 0.15s ease;
  }
  .dropzone:hover {
    background: var(--surface-hover);
    border-color: rgba(124, 58, 237, 0.5);
  }
  .dropzone.dragging {
    border-color: var(--accent-purple);
    border-style: solid;
    background: rgba(124, 58, 237, 0.12);
    transform: translateY(-1px);
  }
  .dropzone.busy {
    cursor: progress;
  }
  .icon {
    color: var(--accent-cyan);
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: rgba(6, 182, 212, 0.1);
  }
  .dropzone.dragging .icon {
    color: var(--accent-purple);
    background: rgba(124, 58, 237, 0.18);
  }
  .hint {
    font-size: 1.05rem;
    font-weight: 600;
  }
  .sub {
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  .err {
    margin-top: 0.4rem;
    font-size: 0.8rem;
    color: var(--danger);
  }
</style>
