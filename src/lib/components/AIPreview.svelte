<div class="preview-overlay">
  <div class="preview-header">
    <span class="preview-title">AI Result Preview</span>
  </div>
  <div class="preview-canvas-container">
    <canvas
      bind:this={previewCanvas}
      class="preview-canvas"
      width={512}
      height={512}
    ></canvas>
  </div>
  <div class="preview-actions">
    <button class="action-btn discard" onclick={onDiscard}>Discard</button>
    <button class="action-btn apply" onclick={onApply}>Apply to Terrain</button>
  </div>
</div>

<script lang="ts">
  import { onMount } from "svelte";

  let {
    resultImage,
    onApplyResult,
    onDiscardResult,
  }: {
    resultImage: Uint8Array;
    onApplyResult: () => void;
    onDiscardResult: () => void;
  } = $props();

  let previewCanvas: HTMLCanvasElement;

  onMount(() => {
    const ctx = previewCanvas.getContext("2d")!;
    const blob = new Blob([resultImage], { type: "image/png" });
    const url = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      ctx.drawImage(img, 0, 0, 512, 512);
      URL.revokeObjectURL(url);
    };
    img.src = url;
  });

  function onDiscard() {
    onDiscardResult();
  }

  function onApply() {
    onApplyResult();
  }
</script>

<style>
  .preview-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 16px;
  }

  .preview-header {
    width: 512px;
    margin-bottom: 8px;
  }

  .preview-title {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .preview-canvas-container {
    width: 512px;
    height: 512px;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  .preview-canvas {
    width: 100%;
    height: 100%;
  }

  .preview-actions {
    width: 512px;
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .action-btn {
    flex: 1;
    padding: 8px;
    font-size: 13px;
    margin-top: 0;
  }

  .action-btn.discard {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
  }

  .action-btn.discard:hover {
    background: var(--border);
  }

  .action-btn.apply {
    background: var(--accent);
  }
</style>
