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
  </Sidebar>
  <div class="viewer-container">
    <TerrainViewer
      bind:this={viewer}
      {brushOp}
      {brushRadius}
      {brushStrength}
    />
  </div>
</div>

<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import TerrainViewer from "./lib/components/TerrainViewer.svelte";
  import BrushControls from "./lib/components/BrushControls.svelte";
  import GenerationControls from "./lib/components/GenerationControls.svelte";
  import ErosionControls from "./lib/components/ErosionControls.svelte";
  import {
    getHeightmap,
    generateTerrain,
    runThermalErosion,
    runHydraulicErosion,
    abortErosion,
  } from "./lib/tauri";
  import type { BrushOp, NoiseParams, ThermalParams, HydraulicParams } from "./lib/types";

  let viewer: ReturnType<typeof TerrainViewer>;
  let brushOp: BrushOp = $state("raise");
  let brushRadius = $state(25);
  let brushStrength = $state(0.5);
  let eroding = $state(false);
  let erosionProgress = $state(0);

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
</script>
