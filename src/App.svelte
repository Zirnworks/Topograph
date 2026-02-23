<div class="app-layout">
  <Sidebar>
    <BrushControls
      bind:brushOp
      bind:brushRadius
      bind:brushStrength
    />
    <GenerationControls onGenerated={handleGenerate} />
    <ErosionControls
      {eroding}
      {erosionProgress}
      onThermalErode={handleThermal}
      onHydraulicErode={handleHydraulic}
      onAbortErosion={handleAbort}
    />
    <AIControls
      {aiRunning}
      {aiStatusText}
      {aiError}
      onOpenEditor={handleOpenAIEditor}
    />
  </Sidebar>
  <div class="viewer-container">
    <TerrainViewer
      bind:this={viewer}
      {brushOp}
      {brushRadius}
      {brushStrength}
    />
    {#if aiMode === "painting"}
      <MaskPainter
        terrainImage={capturedTerrain!}
        onSubmit={handleInpaint}
        onClose={handleCloseAI}
      />
    {/if}
    {#if aiMode === "preview" && inpaintResult}
      <AIPreview
        resultImage={inpaintResult}
        onApplyResult={handleApplyResult}
        onDiscardResult={handleCloseAI}
      />
    {/if}
    {#if aiMode === "running"}
      <div class="ai-loading-overlay">
        <div class="ai-loading-content">
          <div class="spinner"></div>
          <div class="ai-loading-text">{aiStatusText}</div>
        </div>
      </div>
    {/if}
  </div>
</div>

<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import TerrainViewer from "./lib/components/TerrainViewer.svelte";
  import BrushControls from "./lib/components/BrushControls.svelte";
  import GenerationControls from "./lib/components/GenerationControls.svelte";
  import ErosionControls from "./lib/components/ErosionControls.svelte";
  import AIControls from "./lib/components/AIControls.svelte";
  import MaskPainter from "./lib/components/MaskPainter.svelte";
  import AIPreview from "./lib/components/AIPreview.svelte";
  import {
    getHeightmap,
    generateTerrain,
    runThermalErosion,
    runHydraulicErosion,
    abortErosion,
    runDepthEstimation,
    runInpainting,
  } from "./lib/tauri";
  import type { BrushOp, NoiseParams, ThermalParams, HydraulicParams } from "./lib/types";

  let viewer: ReturnType<typeof TerrainViewer>;
  let brushOp: BrushOp = $state("raise");
  let brushRadius = $state(25);
  let brushStrength = $state(0.5);
  let eroding = $state(false);
  let erosionProgress = $state(0);

  // AI state
  let aiMode: "idle" | "painting" | "running" | "preview" = $state("idle");
  let aiRunning = $state(false);
  let aiStatusText = $state("");
  let aiError = $state("");
  let capturedTerrain: Uint8Array | null = $state(null);
  let inpaintResult: Uint8Array | null = $state(null);
  let currentMask: Uint8Array | null = $state(null);

  onMount(async () => {
    const hm = await getHeightmap();
    viewer.buildTerrain(hm);
  });

  async function handleGenerate(params: NoiseParams) {
    const hm = await generateTerrain(params);
    viewer.rebuildFromFull(hm);
  }

  async function handleThermal(params: ThermalParams) {
    const hm = await runThermalErosion(params);
    viewer.rebuildFromFull(hm);
  }

  async function handleHydraulic(params: HydraulicParams) {
    eroding = true;
    erosionProgress = 0;
    try {
      await runHydraulicErosion(params, (progress) => {
        erosionProgress = progress;
      });
      const hm = await getHeightmap();
      viewer.rebuildFromFull(hm);
    } finally {
      eroding = false;
      erosionProgress = 0;
    }
  }

  async function handleAbort() {
    await abortErosion();
  }

  // --- AI workflow ---

  function handleOpenAIEditor() {
    aiError = "";
    const png = viewer.captureTopDown();
    if (!png) {
      aiError = "Failed to capture terrain view";
      return;
    }
    capturedTerrain = png;
    aiMode = "painting";
  }

  async function handleInpaint(mask: Uint8Array, prompt: string) {
    aiMode = "running";
    aiRunning = true;
    aiStatusText = "Running inpainting (~20-40s)...";
    aiError = "";
    currentMask = mask;

    try {
      const result = await runInpainting(capturedTerrain!, mask, prompt);
      inpaintResult = result;
      aiMode = "preview";
    } catch (e: any) {
      aiError = e?.message || String(e);
      aiMode = "painting"; // Go back to editor on error
    } finally {
      aiRunning = false;
      aiStatusText = "";
    }
  }

  async function handleApplyResult() {
    if (!inpaintResult || !currentMask) return;
    aiMode = "running";
    aiRunning = true;
    aiStatusText = "Applying depth estimation...";
    aiError = "";

    try {
      const hm = await runDepthEstimation(inpaintResult, currentMask);
      viewer.rebuildFromFull(hm);
      handleCloseAI();
    } catch (e: any) {
      aiError = e?.message || String(e);
      aiMode = "preview"; // Go back to preview on error
    } finally {
      aiRunning = false;
      aiStatusText = "";
    }
  }

  function handleCloseAI() {
    aiMode = "idle";
    capturedTerrain = null;
    inpaintResult = null;
    currentMask = null;
    aiError = "";
  }
</script>

<style>
  .ai-loading-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .ai-loading-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .ai-loading-text {
    color: var(--text-secondary);
    font-size: 14px;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
