<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import MainView from "./views/MainView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import HistoryView from "./components/HistoryView.svelte";
  import Segmented from "./components/Segmented.svelte";
  import { api, events } from "./lib/api";
  import { settings, queueMode, activeProfileId, updateItem, setProgress, pushLog } from "./lib/stores";
  import { lang, setLang, tr } from "./lib/i18n";
  import type { Lang } from "./lib/i18n";
  import type { ProgressEvent, LogEvent } from "./lib/types";

  type View = "main" | "settings" | "history";
  let view: View = "main";

  // 完成通知（Web Notification），按 itemId 去重避免重复弹。
  const notified = new Set<string>();
  function notify(body: string) {
    try {
      if (typeof Notification !== "undefined" && Notification.permission === "granted") {
        new Notification("VRCaster", { body });
      }
    } catch {
      /* 忽略通知失败 */
    }
  }

  const langOptions = [
    { value: "zh", label: "中文" },
    { value: "en", label: "EN" },
  ];

  function onLangChange(e: CustomEvent<string>) {
    setLang(e.detail as Lang);
    settings.update((s) => ({ ...s, language: e.detail as Lang }));
  }

  // progress 事件 → 更新对应 item 的进度与状态
  function applyProgress(e: ProgressEvent) {
    setProgress(e);
    if (e.stage === "encode") {
      updateItem(e.itemId, {
        encodeProgress: e.percent,
        status: e.percent >= 100 ? "encoded" : "encoding",
      });
    } else {
      updateItem(e.itemId, {
        uploadProgress: e.percent,
        status: e.percent >= 100 ? "done" : "uploading",
      });
      if (e.percent >= 100 && e.itemId !== "__sample__" && !notified.has(e.itemId)) {
        notified.add(e.itemId);
        notify("上传完成，播放链接已就绪");
      }
    }
  }

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenLog: UnlistenFn | null = null;

  onMount(async () => {
    try {
      unlistenProgress = await events.onProgress(applyProgress);
      unlistenLog = await events.onLog((e: LogEvent) => pushLog(e));
    } catch {
      // 非 Tauri 环境忽略
    }
    // 请求通知权限（用于完成提醒）。
    try {
      if (typeof Notification !== "undefined" && Notification.permission === "default") {
        await Notification.requestPermission();
      }
    } catch {
      /* 忽略权限请求失败 */
    }
    try {
      const s = await api.getSettings();
      settings.set(s);
      setLang(s.language);
      queueMode.set(s.defaultQueueMode);
      activeProfileId.set(s.activeProfileId);
    } catch {
      // 后端未就绪：保留 stores 默认值
    }
  });

  onDestroy(() => {
    if (unlistenProgress) unlistenProgress();
    if (unlistenLog) unlistenLog();
  });
</script>

<div class="app">
  <header class="brand">
    <button
      type="button"
      class="logo-btn"
      on:click={() => (view = "main")}
      title={tr("appTitle")}
    >
      <span class="logo">{tr("appTitle")}</span>
      <span class="tagline">{tr("tagline")}</span>
    </button>

    <div class="brand-right">
      <Segmented options={langOptions} value={$lang} on:change={onLangChange} />
      <button
        type="button"
        class="gear"
        class:active={view === "history"}
        title="历史记录"
        aria-label="历史记录"
        on:click={() => (view = view === "history" ? "main" : "history")}
      >
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 3v5h5" />
          <path d="M3.05 13A9 9 0 1 0 6 5.3L3 8" />
          <path d="M12 7v5l3 2" />
        </svg>
      </button>
      <button
        type="button"
        class="gear"
        class:active={view === "settings"}
        title={tr("settings")}
        aria-label={tr("settings")}
        on:click={() => (view = view === "settings" ? "main" : "settings")}
      >
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </div>
  </header>

  <main class="body">
    {#if view === "settings"}
      <SettingsView />
    {:else if view === "history"}
      <HistoryView />
    {:else}
      <MainView />
    {/if}
  </main>
</div>

<style>
  .app {
    min-height: 100vh;
  }
  .brand {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.85rem 1.6rem;
    background: rgba(11, 11, 16, 0.66);
    border-bottom: 1px solid var(--border);
    backdrop-filter: blur(16px);
  }
  .logo-btn {
    appearance: none;
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    text-align: left;
  }
  .logo {
    font-size: 1.4rem;
    font-weight: 800;
    letter-spacing: 0.5px;
    background: var(--accent-grad);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }
  .tagline {
    font-size: 0.76rem;
    color: var(--text-dim);
  }
  .brand-right {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .gear {
    appearance: none;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-dim);
    cursor: pointer;
    transition:
      color 0.15s ease,
      background 0.15s ease,
      transform 0.15s ease;
  }
  .gear:hover {
    color: var(--text);
    background: var(--surface-hover);
    transform: translateY(-1px);
  }
  .gear.active {
    color: #fff;
    border-color: transparent;
    background: var(--accent-grad);
  }
</style>
