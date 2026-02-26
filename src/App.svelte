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
    saveProject,
    loadProject,
    exportHeightmap,
  } from "./lib/tauri";
  import type { BrushOp, NoiseParams, ThermalParams, HydraulicParams, ProjectSettings } from "./lib/types";

  let viewer: ReturnType<typeof TerrainViewer>;
  let generationControls: ReturnType<typeof GenerationControls>;
  let erosionControls: ReturnType<typeof ErosionControls>;
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

  async function handleInpaint(mask: Uint8Array, prompt: string) {
    aiMode = "running";
    aiRunning = true;
    aiStatusText = "Generating with SDXL (~60s)...";
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
      // Composite the AI image onto the persistent terrain texture using the mask
      await viewer.compositeTexture(inpaintResult, currentMask);
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
