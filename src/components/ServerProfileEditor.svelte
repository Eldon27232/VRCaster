<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import { t } from "../lib/i18n";
  import type { ServerProfile, AuthKind } from "../lib/types";
  import NginxDeploy from "./NginxDeploy.svelte";

  // null 表示新增；非空表示编辑既有配置。
  export let profile: ServerProfile | null = null;

  const dispatch = createEventDispatcher<{
    save: { profile: ServerProfile; secret: string | null };
    cancel: void;
  }>();

  function genId(): string {
    return `p_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
  }

  // 表单字段（与既有 profile 解耦，避免直接改 store 对象）。
  let id = profile?.id ?? genId();
  let name = profile?.name ?? "";
  let host = profile?.host ?? "";
  let port: number = profile?.port ?? 22;
  let username = profile?.username ?? "root";
  let authKind: AuthKind = profile?.authKind ?? "password";
  let privateKeyPath: string = profile?.privateKeyPath ?? "";
  let remoteDir = profile?.remoteDir ?? "/var/www/html";
  let urlPrefix = profile?.urlPrefix ?? "";
  let accessKey = profile?.accessKey ?? "";

  // 密码 / 私钥口令：编辑时不回显已存 keyring 的明文，留空表示不修改。
  let secret = "";

  let genningKey = false;
  let saving = false;
  let errorMsg = "";

  // 是否已是已保存的配置（决定能否部署 nginx）。
  $: isPersisted = profile !== null;

  async function pickKey() {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        title: "选择私钥文件",
      });
      if (typeof picked === "string") privateKeyPath = picked;
    } catch (e) {
      errorMsg = `选择文件失败：${e}`;
    }
  }

  async function genKey() {
    genningKey = true;
    try {
      accessKey = await api.generateAccessKey();
    } catch (e) {
      errorMsg = `生成 key 失败：${e}`;
    } finally {
      genningKey = false;
    }
  }

  function validate(): string {
    if (!name.trim()) return "请填写配置名称。";
    if (!host.trim()) return "请填写主机。";
    if (!username.trim()) return "请填写用户名。";
    if (!Number.isFinite(port) || port < 1 || port > 65535)
      return "端口需在 1–65535 之间。";
    if (authKind === "privatekey" && !privateKeyPath.trim())
      return "请选择私钥文件。";
    if (!remoteDir.trim()) return "请填写远程目录。";
    if (!urlPrefix.trim()) return "请填写 URL 前缀。";
    return "";
  }

  function onSave() {
    const err = validate();
    if (err) {
      errorMsg = err;
      return;
    }
    errorMsg = "";
    saving = true;
    const built: ServerProfile = {
      id,
      name: name.trim(),
      host: host.trim(),
      port,
      username: username.trim(),
      authKind,
      privateKeyPath: authKind === "privatekey" ? privateKeyPath.trim() : null,
      remoteDir: remoteDir.trim(),
      urlPrefix: urlPrefix.trim().replace(/\/+$/, ""),
      accessKey: accessKey.trim(),
    };
    // 留空 secret 不覆盖 keyring 中已存的口令。
    dispatch("save", { profile: built, secret: secret ? secret : null });
    saving = false;
  }

  function onCancel() {
    dispatch("cancel");
  }
</script>

<div class="editor">
  <header class="head">
    <h3>{profile ? "编辑服务器配置" : $t("addProfile")}</h3>
  </header>

  <div class="grid">
    <label class="field span2">
      <span>配置名称</span>
      <input bind:value={name} placeholder="例如：我的 LA VPS" />
    </label>

    <label class="field">
      <span>主机</span>
      <input bind:value={host} placeholder="example.com 或 1.2.3.4" />
    </label>

    <label class="field">
      <span>端口</span>
      <input type="number" min="1" max="65535" bind:value={port} />
    </label>

    <label class="field">
      <span>用户名</span>
      <input bind:value={username} placeholder="root" />
    </label>

    <label class="field">
      <span>认证方式</span>
      <select bind:value={authKind}>
        <option value="password">密码</option>
        <option value="privatekey">私钥</option>
      </select>
    </label>

    {#if authKind === "password"}
      <label class="field span2">
        <span>密码{#if isPersisted}<em class="hint">（留空保留已保存的）</em>{/if}</span>
        <input
          type="password"
          bind:value={secret}
          placeholder={isPersisted ? "••••（留空不修改）" : "服务器密码"}
          autocomplete="off"
        />
      </label>
    {:else}
      <label class="field span2">
        <span>私钥文件</span>
        <div class="row">
          <input
            bind:value={privateKeyPath}
            placeholder="C:\\Users\\you\\.ssh\\id_rsa"
            readonly
          />
          <button class="btn-ghost" type="button" on:click={pickKey}>选择文件</button>
        </div>
      </label>
      <label class="field span2">
        <span>私钥口令{#if isPersisted}<em class="hint">（无口令或不修改可留空）</em>{/if}</span>
        <input
          type="password"
          bind:value={secret}
          placeholder="私钥口令（可选）"
          autocomplete="off"
        />
      </label>
    {/if}

    <label class="field">
      <span>远程目录</span>
      <input bind:value={remoteDir} placeholder="/var/www/html" />
    </label>

    <label class="field">
      <span>URL 前缀</span>
      <input bind:value={urlPrefix} placeholder="http://video.example.com" />
    </label>

    <label class="field span2">
      <span>访问 key（?key= 的值）</span>
      <div class="row">
        <input bind:value={accessKey} placeholder="鉴权 key" />
        <button class="btn-ghost" type="button" on:click={genKey} disabled={genningKey}>
          {genningKey ? "生成中…" : $t("generateKey")}
        </button>
      </div>
    </label>
  </div>

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}

  <div class="actions">
    <button class="btn-ghost" type="button" on:click={onCancel}>取消</button>
    <button class="btn-primary" type="button" on:click={onSave} disabled={saving}>
      保存
    </button>
  </div>

  <div class="nginx-wrap">
    {#if isPersisted}
      <NginxDeploy profileId={id} />
    {:else}
      <div class="nginx-locked">
        <div class="title">{$t("deployNginx")}</div>
        <p>保存配置后即可在此一键部署 nginx 鉴权站点。</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .editor {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    backdrop-filter: blur(12px);
    padding: 20px 22px;
  }
  .head {
    margin-bottom: 16px;
  }
  h3 {
    margin: 0;
    font-size: 1.05rem;
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
    min-width: 0;
  }
  .field.span2 {
    grid-column: 1 / -1;
  }
  .field > span {
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  .hint {
    font-style: normal;
    opacity: 0.75;
  }
  input,
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
  input:focus,
  select:focus {
    border-color: var(--accent-purple);
  }
  input[readonly] {
    color: var(--text-dim);
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row input {
    flex: 1;
  }
  .error {
    margin: 14px 0 0;
    color: var(--danger);
    font-size: 0.85rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 18px;
  }
  .btn-primary {
    border: none;
    color: #fff;
    background: var(--accent-grad);
    border-radius: 10px;
    padding: 0.55rem 1.4rem;
    font-size: 0.9rem;
    cursor: pointer;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }
  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .btn-ghost {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    border-radius: 10px;
    padding: 0.55rem 1rem;
    font-size: 0.9rem;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: var(--accent-cyan);
  }
  .btn-ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .nginx-wrap {
    margin-top: 20px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  .nginx-locked {
    border: 1px dashed var(--border);
    border-radius: 12px;
    padding: 14px 16px;
    color: var(--text-dim);
  }
  .nginx-locked .title {
    font-weight: 600;
    color: var(--text);
    margin-bottom: 4px;
  }
  .nginx-locked p {
    margin: 0;
    font-size: 0.85rem;
  }
</style>
