<div class="app-layout">
  <Sidebar>
    <BrushControls
      bind:brushOp
      bind:brushRadius
      bind:brushStrength
    />
    <GenerationControls bind:this={generationControls} onGenerated={handleGenerate} />
    <ErosionControls bind:this={erosionControls}
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
    <div style="margin-top: auto;">
      <FileControls
        onSave={handleSave}
        onLoad={handleLoad}
        onExport={handleExport}
      />
    </div>
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
    {#if aiMode === "adjusting"}
      <div class="adjust-overlay">
        <div class="adjust-panel">
          <div class="adjust-title">Adjust Displacement Strength</div>
          <div class="adjust-slider-row">
            <span class="adjust-label">0%</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              bind:value={adjustStrength}
              oninput={onStrengthChange}
              class="adjust-slider"
            />
            <span class="adjust-label">{Math.round(adjustStrength * 100)}%</span>
          </div>
          <div class="adjust-actions">
            <button class="adjust-btn cancel" onclick={cancelAdjustment}>Cancel</button>
            <button class="adjust-btn confirm" onclick={confirmAdjustment}>Confirm</button>
          </div>
        </div>
      </div>
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
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import FileControls from "./lib/components/FileControls.svelte";
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
    applyHeightmapImage,
    setHeightmap,
    saveProject,
    loadProject,
    exportHeightmap,
  } from "./lib/tauri";
  import type { AISculptMode, BrushOp, NoiseParams, ThermalParams, HydraulicParams, ProjectSettings } from "./lib/types";

  let viewer: ReturnType<typeof TerrainViewer>;
  let generationControls: ReturnType<typeof GenerationControls>;
  let erosionControls: ReturnType<typeof ErosionControls>;
  let brushOp: BrushOp = $state("raise");
  let brushRadius = $state(25);
  let brushStrength = $state(0.5);
  let eroding = $state(false);
  let erosionProgress = $state(0);

  // AI state
  let aiMode: "idle" | "painting" | "running" | "preview" | "adjusting" = $state("idle");
  let aiRunning = $state(false);
  let aiStatusText = $state("");
  let aiError = $state("");
  let capturedTerrain: Uint8Array | null = $state(null);
  let inpaintResult: Uint8Array | null = $state(null);
  let currentMask: Uint8Array | null = $state(null);
  let currentAIMode: AISculptMode = $state("heightmap");

  // Adjustment mode state
  let adjustStrength = $state(0.5);
  let originalHeightmap: Float32Array | null = null;
  let modifiedHeightmap: Float32Array | null = null;
  let hmWidth = 0;
  let hmHeight = 0;

  let unlisten: (() => void) | null = null;

  function onKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      handleSave();
    } else if ((e.metaKey || e.ctrlKey) && e.key === "o") {
      e.preventDefault();
      handleLoad();
    }
  }

  onMount(async () => {
    const hm = await getHeightmap();
    viewer.buildTerrain(hm);

    window.addEventListener("keydown", onKeyDown);

    unlisten = await listen<string>("menu-action", (event) => {
      const action = event.payload;
      switch (action) {
        case "save": handleSave(); break;
        case "open": handleLoad(); break;
        case "export_png16": handleExport("png16"); break;
        case "export_raw": handleExport("raw_f32"); break;
      }
    });
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKeyDown);
    unlisten?.();
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

  // --- File operations ---

  async function handleSave() {
    try {
      const path = await save({
        filters: [{ name: "Topograph Project", extensions: ["topo"] }],
        defaultPath: "terrain.topo",
      });
      if (!path) return;

      const texturePng = await viewer.getTexturePNG();
      const settings: ProjectSettings = {
        version: 1,
        brush: { op: brushOp, radius: brushRadius, strength: brushStrength },
        generation: generationControls.getSettings(),
        erosion: erosionControls.getSettings(),
      };

      await saveProject(texturePng, JSON.stringify(settings), path);
    } catch (e: any) {
      console.error("Save failed:", e);
    }
  }

  async function handleLoad() {
    try {
      const path = await open({
        filters: [{ name: "Topograph Project", extensions: ["topo"] }],
        multiple: false,
      });
      if (!path) return;

      const response = await loadProject(path as string);

      const hm = await getHeightmap();
      viewer.rebuildFromFull(hm);

      if (response.texturePng) {
        await viewer.restoreTexture(new Uint8Array(response.texturePng));
      } else {
        viewer.clearTexture();
      }

      if (response.settingsJson && response.settingsJson !== "{}") {
        const settings: ProjectSettings = JSON.parse(response.settingsJson);
        brushOp = settings.brush.op;
        brushRadius = settings.brush.radius;
        brushStrength = settings.brush.strength;
        generationControls.setSettings(settings.generation);
        erosionControls.setSettings(settings.erosion);
      }
    } catch (e: any) {
      console.error("Load failed:", e);
    }
  }

  async function handleExport(format: string) {
    try {
      const ext = format === "png16" ? "png" : "bin";
      const name = format === "png16" ? "PNG Image (16-bit)" : "Raw f32 Binary";
      const path = await save({
        filters: [{ name, extensions: [ext] }],
        defaultPath: `terrain.${ext}`,
      });
      if (!path) return;

      await exportHeightmap(path, format);
    } catch (e: any) {
      console.error("Export failed:", e);
    }
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

  async function handleInpaint(mask: Uint8Array, prompt: string, mode: AISculptMode) {
    aiMode = "running";
    aiRunning = true;
    aiStatusText = mode === "heightmap"
      ? "Generating heightmap with SDXL (~60s)..."
      : "Generating with SDXL (~60s)...";
    aiError = "";
    currentMask = mask;
    currentAIMode = mode;

    try {
      const result = await runInpainting(capturedTerrain!, mask, prompt, mode);
      inpaintResult = result;
      aiMode = "preview";
    } catch (e: any) {
      aiError = e?.message || String(e);
      aiMode = "painting";
    } finally {
      aiRunning = false;
      aiStatusText = "";
    }
  }

  async function handleApplyResult() {
    if (!inpaintResult || !currentMask) return;
    aiMode = "running";
    aiRunning = true;
    aiError = "";

    try {
      // Snapshot original heightmap before applying
      const origHm = await getHeightmap();
      originalHeightmap = new Float32Array(origHm.data);
      hmWidth = origHm.width;
      hmHeight = origHm.height;

      if (currentAIMode === "heightmap") {
        aiStatusText = "Applying heightmap...";
        const hm = await applyHeightmapImage(inpaintResult, currentMask);
        modifiedHeightmap = new Float32Array(hm.data);
        // Enter adjustment mode with live slider
        adjustStrength = 0.5;
        applyBlend(0.5);
        aiMode = "adjusting";
      } else {
        aiStatusText = "Applying depth estimation...";
        const hm = await runDepthEstimation(inpaintResult, currentMask);
        modifiedHeightmap = new Float32Array(hm.data);
        // Enter adjustment mode
        adjustStrength = 0.5;
        applyBlend(0.5);
        aiMode = "adjusting";
      }
    } catch (e: any) {
      aiError = e?.message || String(e);
      aiMode = "preview";
    } finally {
      aiRunning = false;
      aiStatusText = "";
    }
  }

  function applyBlend(strength: number) {
    if (!originalHeightmap || !modifiedHeightmap) return;
    const blended = new Float32Array(originalHeightmap.length);
    for (let i = 0; i < blended.length; i++) {
      blended[i] = originalHeightmap[i] * (1 - strength) + modifiedHeightmap[i] * strength;
    }
    viewer.rebuildFromFull({ width: hmWidth, height: hmHeight, data: blended });
  }

  function onStrengthChange() {
    applyBlend(adjustStrength);
  }

  async function confirmAdjustment() {
    if (!originalHeightmap || !modifiedHeightmap) return;
    // Compute final blend and commit to Rust
    const final_ = new Float32Array(originalHeightmap.length);
    for (let i = 0; i < final_.length; i++) {
      final_[i] = originalHeightmap[i] * (1 - adjustStrength) + modifiedHeightmap[i] * adjustStrength;
    }
    await setHeightmap(final_);

    // If texture mode, also composite the texture
    if (currentAIMode === "texture" && inpaintResult && currentMask) {
      await viewer.compositeTexture(inpaintResult, currentMask);
    }

    handleCloseAI();
  }

  async function cancelAdjustment() {
    if (originalHeightmap) {
      // Restore original heightmap
      await setHeightmap(originalHeightmap);
      viewer.rebuildFromFull({ width: hmWidth, height: hmHeight, data: originalHeightmap });
    }
    handleCloseAI();
  }

  function handleCloseAI() {
    aiMode = "idle";
    capturedTerrain = null;
    inpaintResult = null;
    currentMask = null;
    currentAIMode = "heightmap";
    aiError = "";
    originalHeightmap = null;
    modifiedHeightmap = null;
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

  .adjust-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    justify-content: center;
    padding: 16px;
    z-index: 100;
    pointer-events: none;
  }

  .adjust-panel {
    background: rgba(22, 33, 62, 0.95);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px 24px;
    min-width: 400px;
    pointer-events: auto;
  }

  .adjust-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 12px;
    text-align: center;
  }

  .adjust-slider-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .adjust-slider {
    flex: 1;
    -webkit-appearance: none;
    appearance: none;
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    outline: none;
  }

  .adjust-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
  }

  .adjust-label {
    font-size: 12px;
    color: var(--text-secondary);
    min-width: 32px;
    font-variant-numeric: tabular-nums;
  }

  .adjust-actions {
    display: flex;
    gap: 8px;
  }

  .adjust-btn {
    flex: 1;
    padding: 8px;
    font-size: 13px;
    margin-top: 0;
    border-radius: 4px;
  }

  .adjust-btn.cancel {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
  }

  .adjust-btn.cancel:hover {
    background: var(--border);
  }

  .adjust-btn.confirm {
    background: var(--accent);
  }
</style>
