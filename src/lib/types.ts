// 与 src-tauri/src/types.rs 一一对应（serde rename_all = "camelCase"）。
// 修改此处务必同步 Rust 端，见 docs/contract.md。

export interface AudioTrack {
  index: number;
  codec: string;
  channels: number;
  language: string | null;
  title: string | null;
}

export interface SubtitleTrack {
  index: number;
  codec: string;
  language: string | null;
  title: string | null;
}

export interface MediaInfo {
  path: string;
  durationSecs: number;
  width: number;
  height: number;
  fps: number;
  isHdr: boolean;
  colorTransfer: string | null;
  sizeBytes: number;
  audioTracks: AudioTrack[];
  subtitleTracks: SubtitleTrack[];
}

export type Resolution = "p720" | "p1080" | "p1440" | "p2160";
export type Speed = "fast" | "medium" | "slow";
export type Tonemap = "hable" | "mobius" | "reinhard";

export type RateMode =
  | { kind: "targetSize"; gb: number }
  | { kind: "quality"; crf: number };

export type SubtitleChoice =
  | { kind: "embedded"; index: number }
  | { kind: "external"; path: string }
  | { kind: "none" };

export interface EncodeParams {
  resolution: Resolution;
  rateMode: RateMode;
  speed: Speed;
  hwaccel: boolean;
  tonemap: Tonemap;
  audioTrackIndex: number;
  audioBitrateK: number;
  subtitle: SubtitleChoice;
}

export interface SampleSpec {
  startSecs: number;
  durationSecs: number;
}

export interface FullEstimate {
  sizeBytes: number;
  bitrateK: number;
  elapsedSecs: number;
  suggested: EncodeParams | null;
}

export interface SampleResult {
  outputPath: string;
  sizeBytes: number;
  avgBitrateK: number;
  elapsedSecs: number;
  fullEstimate: FullEstimate;
}

export type AuthKind = "password" | "privatekey";

export interface ServerProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authKind: AuthKind;
  privateKeyPath: string | null;
  remoteDir: string;
  urlPrefix: string;
  accessKey: string;
}

export type ItemStatus =
  | "pending"
  | "encoding"
  | "encoded"
  | "uploading"
  | "done"
  | "error";

export interface QueueItem {
  id: string;
  media: MediaInfo;
  params: EncodeParams;
  status: ItemStatus;
  encodeProgress: number;
  uploadProgress: number;
  resultUrl: string | null;
  error: string | null;
}

export type QueueMode =
  | "pipeline"
  | "parallelEncodeSerialUpload"
  | "parallelAll";

export interface RemoteSpace {
  freeBytes: number;
  totalBytes: number;
}

export interface FfmpegPaths {
  ffmpeg: string;
  ffprobe: string;
}

export interface AppSettings {
  language: "zh" | "en";
  defaultResolution: Resolution;
  defaultSpeed: Speed;
  defaultTonemap: Tonemap;
  defaultAudioBitrateK: number;
  defaultQueueMode: QueueMode;
  activeProfileId: string | null;
}

// 事件载荷
export interface ProgressEvent {
  itemId: string;
  stage: "encode" | "upload";
  percent: number;
  speed: string;
  etaSecs: number | null;
}

export interface LogEvent {
  itemId: string | null;
  level: "info" | "warn" | "error";
  message: string;
}

export interface DownloadEvent {
  percent: number;
  message: string;
}
