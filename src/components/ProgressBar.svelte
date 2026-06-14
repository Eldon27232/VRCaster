<script lang="ts">
  export let percent: number = 0;
  export let label: string | undefined = undefined;
  export let active: boolean = false;

  $: clamped = Math.max(0, Math.min(100, percent || 0));
</script>

<div class="wrap">
  {#if label !== undefined}
    <div class="head">
      <span class="label">{label}</span>
      <span class="pct">{clamped.toFixed(0)}%</span>
    </div>
  {/if}
  <div class="track">
    <div
      class="fill"
      class:active
      style="width: {clamped}%"
    ></div>
  </div>
</div>

<style>
  .wrap {
    width: 100%;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.3rem;
    font-size: 0.78rem;
  }
  .label {
    color: var(--text-dim);
  }
  .pct {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .track {
    width: 100%;
    height: 7px;
    border-radius: 999px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 999px;
    background: var(--accent-grad);
    transition: width 0.25s ease;
  }
  .fill.active {
    background-image: linear-gradient(
      110deg,
      var(--accent-purple) 0%,
      var(--accent-cyan) 40%,
      rgba(255, 255, 255, 0.55) 50%,
      var(--accent-cyan) 60%,
      var(--accent-purple) 100%
    );
    background-size: 220% 100%;
    animation: flow 1.4s linear infinite;
  }
  @keyframes flow {
    from {
      background-position: 220% 0;
    }
    to {
      background-position: 0 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fill.active {
      animation: none;
    }
  }
</style>
