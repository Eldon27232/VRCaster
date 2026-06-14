<script lang="ts">
  import { api } from "../lib/api";
  import { t } from "../lib/i18n";

  // 服务器配置 id；该 profile 必须已保存后才能部署。
  export let profileId: string;

  let deploying = false;
  let log = "";
  let failed = false;

  async function deploy() {
    if (!profileId) {
      failed = true;
      log = "请先保存服务器配置后再部署。";
      return;
    }
    deploying = true;
    failed = false;
    log = "";
    try {
      log = await api.deployNginx(profileId);
    } catch (e) {
      failed = true;
      log = `${e}`;
    } finally {
      deploying = false;
    }
  }
</script>

<div class="nginx">
  <div class="head">
    <div class="info">
      <div class="title">{$t("deployNginx")}</div>
      <p class="desc">
        通过 SSH 在服务器自动安装 nginx，并写入带 <code>?key=</code> 鉴权的站点配置，随后
        <code>nginx -t</code> 校验并 reload。需先填好并保存上方配置。
      </p>
    </div>
    <button
      class="btn-primary"
      on:click={deploy}
      disabled={deploying || !profileId}
    >
      {deploying ? "部署中…" : $t("deployNginx")}
    </button>
  </div>

  {#if log}
    <pre class="log" class:failed>{log}</pre>
  {/if}
</div>

<style>
  .nginx {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--bg-1);
    padding: 14px 16px;
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    justify-content: space-between;
  }
  .info {
    min-width: 0;
  }
  .title {
    font-weight: 600;
    margin-bottom: 4px;
  }
  .desc {
    margin: 0;
    color: var(--text-dim);
    font-size: 0.85rem;
    line-height: 1.5;
  }
  code {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 5px;
    font-size: 0.8em;
  }
  .btn-primary {
    flex: none;
    border: none;
    color: #fff;
    background: var(--accent-grad);
    border-radius: 10px;
    padding: 0.55rem 1.1rem;
    font-size: 0.9rem;
    cursor: pointer;
    white-space: nowrap;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }
  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .log {
    margin: 12px 0 0;
    padding: 10px 12px;
    background: var(--bg-0);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-dim);
    font-size: 0.8rem;
    line-height: 1.45;
    max-height: 240px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .log.failed {
    color: var(--danger);
    border-color: rgba(239, 68, 68, 0.4);
  }
</style>
