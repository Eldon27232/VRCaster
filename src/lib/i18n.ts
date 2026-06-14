// 极简 i18n：中英双语字典 + 当前语言 store + t() 取词。
import { writable, derived, get } from "svelte/store";

export type Lang = "zh" | "en";

export const lang = writable<Lang>("zh");

type Dict = Record<string, { zh: string; en: string }>;

export const dict: Dict = {
  appTitle: { zh: "VRCaster", en: "VRCaster" },
  tagline: { zh: "VRChat 视频压制 · 上传", en: "Compress & Upload for VRChat" },
  dropHint: { zh: "拖入视频文件，或点击选择", en: "Drop videos here, or click to choose" },
  queue: { zh: "队列", en: "Queue" },
  settings: { zh: "设置", en: "Settings" },
  start: { zh: "开始", en: "Start" },
  encodeSample: { zh: "压样片", en: "Encode Sample" },
  encodeFull: { zh: "压全片", en: "Encode Full" },
  resolution: { zh: "分辨率", en: "Resolution" },
  rateMode: { zh: "码率控制", en: "Bitrate" },
  targetSize: { zh: "目标大小", en: "Target Size" },
  quality: { zh: "质量档", en: "Quality" },
  speed: { zh: "编码速度", en: "Speed" },
  fast: { zh: "快", en: "Fast" },
  medium: { zh: "平衡", en: "Balanced" },
  slow: { zh: "质量", en: "Quality" },
  tonemap: { zh: "HDR 映射", en: "HDR Tonemap" },
  audioTrack: { zh: "音轨", en: "Audio Track" },
  subtitle: { zh: "字幕", en: "Subtitle" },
  none: { zh: "关闭", en: "None" },
  external: { zh: "外挂文件", en: "External File" },
  server: { zh: "服务器", en: "Server" },
  serverProfiles: { zh: "服务器配置", en: "Server Profiles" },
  addProfile: { zh: "新增配置", en: "Add Profile" },
  deployNginx: { zh: "一键部署 nginx", en: "Deploy nginx" },
  generateKey: { zh: "生成 key", en: "Generate Key" },
  language: { zh: "语言", en: "Language" },
  queueMode: { zh: "并发模式", en: "Concurrency" },
  modePipeline: { zh: "顺序流水线", en: "Pipeline" },
  modeParallelSerial: { zh: "同步处理 + 顺序上传", en: "Parallel encode, serial upload" },
  modeParallelAll: { zh: "同步处理 + 同步上传", en: "Parallel encode & upload" },
  copyUrl: { zh: "复制链接", en: "Copy URL" },
  openBrowser: { zh: "浏览器打开", en: "Open in Browser" },
  encoding: { zh: "压制中", en: "Encoding" },
  uploading: { zh: "上传中", en: "Uploading" },
  done: { zh: "完成", en: "Done" },
  error: { zh: "出错", en: "Error" },
};

export const t = derived(lang, ($lang) => (key: string) => {
  const entry = dict[key];
  if (!entry) return key;
  return entry[$lang];
});

export function setLang(l: Lang) {
  lang.set(l);
}

export function tr(key: string): string {
  const entry = dict[key];
  if (!entry) return key;
  return entry[get(lang)];
}
