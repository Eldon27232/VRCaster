<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { t, setLang } from "../lib/i18n";
  import {
    settings,
    profiles,
    activeProfileId,
  } from "../lib/stores";
  import type {
    AppSettings,
    ServerProfile,
    Resolution,
    Speed,
    Tonemap,
    QueueMode,
  } from "../lib/types";
  import ServerProfileEditor from "../components/ServerProfileEditor.svelte";

  let loading = true;
  let loadError = "";

  // 编辑器状态：null 关闭；'new' 新增；ServerProfile 编辑既有。
  let editing: ServerProfile | "new" | null = null;

  const resolutionOptions: { value: Resolution; label: string }[] = [
    { value: "p720", label: "720p" },
    { value: "p1080", label: "1080p" },
    { value: "p1440", label: "1440p" },
    { value: "p2160", label: "4K（部分客户端可能卡）" },
  ];
  const speedOptions: { value: Speed; label: string }[] = [
    { value: "fast", label: "快" },
    { value: "medium", label: "平衡" },
    { value: "slow", label: "质量" },
  ];
  const tonemapOptions: { value: Tonemap; label: string }[] = [
    { value: "hable", label: "hable" },
    { value: "mobius", label: "mobius" },
    { value: "reinhard", label: "reinhard" },
  ];
  const audioBitrateOptions = [128, 192, 256];

  onMount(async () => {
    try {
      const [s, ps] = await Promise.all([
        api.getSettings(),
        api.listProfiles(),
      ]);
      settings.set(s);
      profiles.set(ps);
      activeProfileId.set(s.activeProfileId);
      setLang(s.language);
    } catch (e) {
      loadError = `${e}`;
    } finally {
      loading = false;
    }
  });

  // 统一应用一次设置补丁：更新 store + 持久化。
  async function applySettings(patch: Partial<AppSettings>) {
    const next: AppSettings = { ...$settings, ...patch };
    settings.set(next);
    try {
      await api.setSettings(next);
    } catch (e) {
      loadError = `保存设置失败：${e}`;
    }
  }

  function onLanguage(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value as "zh" | "en";
    setLang(v);
    applySettings({ language: v });
  }

  function onSelect<K extends keyof AppSettings>(key: K) {
    return (e: Event) => {
      const raw = (e.currentTarget as HTMLSelectElement).value;
      const val = (key === "defaultAudioBitrateK"
        ? Number(raw)
        : raw) as AppSettings[K];
      applySettings({ [key]: val } as Partial<AppSettings>);
    };
  }

  // 服务器配置操作
  async function reloadProfiles() {
    profiles.set(await api.listProfiles());
  }

  function openNew() {
    editing = "new";
  }
  function openEdit(p: ServerProfile) {
    editing = p;
  }
  function closeEditor() {
    editing = null;
  }

  async function setActive(id: string) {
    activeProfileId.set(id);
    await applySettings({ activeProfileId: id });
  }

  async function onDelete(p: ServerProfile) {
    if (!confirm(`删除配置「${p.name}」？该操作不可撤销。`)) return;
    try {
      await api.deleteProfile(p.id);
      // 删除的是活动配置则清空 activeProfileId。
      if ($activeProfileId === p.id) {
        activeProfileId.set(null);
        await applySettings({ activeProfileId: null });
      }
      await reloadProfiles();
    } catch (e) {
      loadError = `删除失败：${e}`;
    }
  }

  async function onEditorSave(
    e: CustomEvent<{ profile: ServerProfile; secret: string | null }>,
  ) {
    const { profile, secret } = e.detail;
    try {
      await api.saveProfile(profile, secret);
      await reloadProfiles();
      // 首个配置自动设为活动。
      if ($activeProfileId === null) {
        await setActive(profile.id);
      }
      editing = null;
    } catch (err) {
      loadError = `保存配置失败：${err}`;
    }
  }
</script>

<div class="settings">
  {#if loading}
    <p class="muted">加载设置中…</p>
  {:else}
    {#if loadError}
      <p class="error">{loadError}</p>
    {/if}

    <!-- 通用 -->
    <section class="card">
      <h2>通用</h2>
      <div class="grid">
        <label class="field">
          <span>{$t("language")}</span>
          <select value={$settings.language} on:change={onLanguage}>
            <option value="zh">中文</option>
            <option value="en">English</option>
          </select>
        </label>

        <label class="field">
          <span>默认{$t("resolution")}</span>
          <select
            value={$settings.defaultResolution}
            on:change={onSelect("defaultResolution")}
          >
            {#each resolutionOptions as o}
              <option value={o.value}>{o.label}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>默认{$t("speed")}</span>
          <select
            value={$settings.defaultSpeed}
            on:change={onSelect("defaultSpeed")}
          >
            {#each speedOptions as o}
              <option value={o.value}>{o.label}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>默认{$t("tonemap")}</span>
          <select
            value={$settings.defaultTonemap}
            on:change={onSelect("defaultTonemap")}
          >
            {#each tonemapOptions as o}
              <option value={o.value}>{o.label}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>默认{$t("audioTrack")}码率</span>
          <select
            value={$settings.defaultAudioBitrateK}
            on:change={onSelect("defaultAudioBitrateK")}
          >
            {#each audioBitrateOptions as b}
              <option value={b}>{b}k</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>默认{$t("queueMode")}</span>
          <select
            value={$settings.defaultQueueMode}
            on:change={onSelect("defaultQueueMode")}
          >
            <option value="pipeline">{$t("modePipeline")}</option>
            <option value="parallelEncodeSerialUpload">{$t("modeParallelSerial")}</option>
            <option value="parallelAll">{$t("modeParallelAll")}</option>
          </select>
        </label>
      </div>
    </section>

    <!-- 服务器配置 -->
    <section class="card">
      <div class="card-head">
        <h2>{$t("serverProfiles")}</h2>
        <button class="btn-primary" on:click={openNew}>{$t("addProfile")}</button>
      </div>

      {#if $profiles.length === 0}
        <p class="muted">尚无服务器配置，点击「{$t("addProfile")}」新增。</p>
      {:else}
        <ul class="profile-list">
          {#each $profiles as p (p.id)}
            <li class="profile" class:active={$activeProfileId === p.id}>
              <div class="p-main">
                <div class="p-name">
                  {p.name}
                  {#if $activeProfileId === p.id}
                    <span class="badge">活动</span>
                  {/if}
                </div>
                <div class="p-sub">
                  {p.username}@{p.host}:{p.port}
                  · {p.authKind === "password" ? "密码" : "私钥"}
                  · {p.remoteDir}
                </div>
              </div>
              <div class="p-actions">
                {#if $activeProfileId !== p.id}
                  <button class="btn-ghost" on:click={() => setActive(p.id)}>设为活动</button>
                {/if}
                <button class="btn-ghost" on:click={() => openEdit(p)}>编辑</button>
                <button class="btn-ghost danger" on:click={() => onDelete(p)}>删除</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if editing !== null}
      <section class="editor-slot">
        <ServerProfileEditor
          profile={editing === "new" ? null : editing}
          on:save={onEditorSave}
          on:cancel={closeEditor}
        />
      </section>
    {/if}
  {/if}
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 22px;
    max-width: 820px;
    margin: 0 auto;
    padding: 8px 0 40px;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    backdrop-filter: blur(12px);
    padding: 20px 22px;
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  h2 {
    margin: 0 0 16px;
    font-size: 1.1rem;
  }
  .card-head h2 {
    margin: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px 16px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field > span {
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  select {
    width: 100%;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    padding: 0.55rem 0.7rem;
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.15s ease;
  }
  select:focus {
    border-color: var(--accent-purple);
  }
  .muted {
    color: var(--text-dim);
    margin: 0;
    font-size: 0.9rem;
  }
  .error {
    color: var(--danger);
    margin: 0;
    font-size: 0.88rem;
  }
  .profile-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .profile {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 12px;
    transition: border-color 0.15s ease;
  }
  .profile.active {
    border-color: var(--accent-purple);
  }
  .p-main {
    min-width: 0;
  }
  .p-name {
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: #fff;
    background: var(--accent-grad);
    border-radius: 6px;
    padding: 1px 7px;
  }
  .p-sub {
    margin-top: 3px;
    color: var(--text-dim);
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .p-actions {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .btn-primary {
    border: none;
    color: #fff;
    background: var(--accent-grad);
    border-radius: 10px;
    padding: 0.5rem 1.1rem;
    font-size: 0.88rem;
    cursor: pointer;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }
  .btn-primary:hover {
    transform: translateY(-1px);
  }
  .btn-ghost {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    border-radius: 9px;
    padding: 0.42rem 0.8rem;
    font-size: 0.82rem;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .btn-ghost:hover {
    background: var(--surface-hover);
    border-color: var(--accent-cyan);
  }
  .btn-ghost.danger {
    color: var(--danger);
  }
  .btn-ghost.danger:hover {
    border-color: var(--danger);
    background: rgba(239, 68, 68, 0.1);
  }
</style>
