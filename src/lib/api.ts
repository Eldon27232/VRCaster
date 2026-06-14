// Tauri 命令封装。命令名与 src-tauri/src/lib.rs 注册一致，见 docs/contract.md。
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  MediaInfo,
  EncodeParams,
  SampleSpec,
  SampleResult,
  ServerProfile,
  QueueItem,
  QueueMode,
  RemoteSpace,
  FfmpegPaths,
  AppSettings,
  ProgressEvent,
  LogEvent,
  DownloadEvent,
} from "./types";

export const api = {
  analyzeMedia: (path: string) => invoke<MediaInfo>("analyze_media", { path }),
  defaultSampleSpec: (durationSecs: number) =>
    invoke<SampleSpec>("default_sample_spec", { durationSecs }),
  encodeSample: (media: MediaInfo, params: EncodeParams, spec: SampleSpec) =>
    invoke<SampleResult>("encode_sample", { media, params, spec }),
  startEncode: (itemId: string, media: MediaInfo, params: EncodeParams) =>
    invoke<string>("start_encode", { itemId, media, params }),
  startUpload: (itemId: string, localPath: string, profileId: string) =>
    invoke<string>("start_upload", { itemId, localPath, profileId }),
  runQueue: (items: QueueItem[], mode: QueueMode, profileId: string) =>
    invoke<void>("run_queue", { items, mode, profileId }),

  listProfiles: () => invoke<ServerProfile[]>("list_profiles"),
  saveProfile: (profile: ServerProfile, secret: string | null) =>
    invoke<void>("save_profile", { profile, secret }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),
  generateAccessKey: () => invoke<string>("generate_access_key"),
  checkRemoteSpace: (profileId: string) =>
    invoke<RemoteSpace>("check_remote_space", { profileId }),
  deployNginx: (profileId: string) =>
    invoke<string>("deploy_nginx", { profileId }),

  ensureFfmpeg: () => invoke<FfmpegPaths>("ensure_ffmpeg"),
  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) =>
    invoke<void>("set_settings", { settings }),
  exportConfig: (path: string) => invoke<void>("export_config", { path }),
  importConfig: (path: string) => invoke<void>("import_config", { path }),
};

// 事件订阅
export const events = {
  onProgress: (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("progress", (e) => cb(e.payload)),
  onLog: (cb: (e: LogEvent) => void): Promise<UnlistenFn> =>
    listen<LogEvent>("log", (e) => cb(e.payload)),
  onFfmpegDownload: (cb: (e: DownloadEvent) => void): Promise<UnlistenFn> =>
    listen<DownloadEvent>("ffmpeg-download", (e) => cb(e.payload)),
};
