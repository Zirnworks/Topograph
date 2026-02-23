<div class="section">
  <div class="section-title">AI</div>

  <button onclick={onOpenEditor} disabled={aiRunning}>
    {aiRunning ? aiStatusText : "AI Terrain Editor"}
  </button>

  {#if aiRunning}
    <div class="ai-status">{aiStatusText}</div>
  {/if}

  {#if aiError}
    <div class="ai-error">{aiError}</div>
  {/if}
</div>

<script lang="ts">
  let {
    aiRunning = false,
    aiStatusText = "",
    aiError = "",
    onOpenEditor,
  }: {
    aiRunning: boolean;
    aiStatusText: string;
    aiError: string;
    onOpenEditor: () => void;
  } = $props();
</script>

<style>
  .ai-error {
    color: #ff6b6b;
    font-size: 0.75rem;
    margin-top: 6px;
    word-break: break-word;
  }

  .ai-status {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 4px;
  }
</style>
