// 全局状态：队列、设置、服务器配置。
import { writable } from "svelte/store";
import type {
  QueueItem,
  AppSettings,
  ServerProfile,
  QueueMode,
  ProgressEvent,
  LogEvent,
  HistoryEntry,
  Preset,
} from "./types";

export const queue = writable<QueueItem[]>([]);

// 每个 item 最新一帧的详细进度（ffmpeg/上传字段），按 itemId 索引。
export const progressMap = writable<Record<string, ProgressEvent>>({});

export function setProgress(e: ProgressEvent) {
  progressMap.update((m) => ({ ...m, [e.itemId]: e }));
}

// 日志：压制/上传/队列的 log 事件累积（上限 500 条）。
export const logs = writable<LogEvent[]>([]);
export function pushLog(e: LogEvent) {
  logs.update((l) => {
    const n = [...l, e];
    return n.length > 500 ? n.slice(-500) : n;
  });
}
export function clearLogs() {
  logs.set([]);
}

// 历史记录 / 参数预设（从后端加载后缓存）。
export const history = writable<HistoryEntry[]>([]);
export const presets = writable<Preset[]>([]);

export const settings = writable<AppSettings>({
  language: "zh",
  defaultResolution: "p1440",
  defaultSpeed: "medium",
  defaultTonemap: "hable",
  defaultAudioBitrateK: 192,
  defaultQueueMode: "pipeline",
  activeProfileId: null,
  autoDeleteSample: true,
});

export const profiles = writable<ServerProfile[]>([]);
export const activeProfileId = writable<string | null>(null);
export const queueMode = writable<QueueMode>("pipeline");

// 队列操作辅助
export function updateItem(id: string, patch: Partial<QueueItem>) {
  queue.update((items) =>
    items.map((it) => (it.id === id ? { ...it, ...patch } : it)),
  );
}

export function removeItem(id: string) {
  queue.update((items) => items.filter((it) => it.id !== id));
}

export function moveItem(id: string, dir: -1 | 1) {
  queue.update((items) => {
    const i = items.findIndex((it) => it.id === id);
    if (i < 0) return items;
    const j = i + dir;
    if (j < 0 || j >= items.length) return items;
    const next = items.slice();
    [next[i], next[j]] = [next[j], next[i]];
    return next;
  });
}
