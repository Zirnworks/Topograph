<div class="section">
  <div class="section-title">Erosion</div>

  <div class="subsection-title">Thermal</div>
  <div class="control-row">
    <label for="thermal-iter">Iterations</label>
    <input id="thermal-iter" type="range" min="1" max="50" step="1" bind:value={thermalIterations} />
    <span class="value">{thermalIterations}</span>
  </div>
  <div class="control-row">
    <label for="thermal-talus">Talus</label>
    <input id="thermal-talus" type="range" min="0.1" max="1.5" step="0.05" bind:value={thermalTalus} />
    <span class="value">{thermalTalus.toFixed(2)}</span>
  </div>
  <div class="control-row">
    <label for="thermal-rate">Transfer</label>
    <input id="thermal-rate" type="range" min="0.05" max="0.5" step="0.05" bind:value={thermalTransfer} />
    <span class="value">{thermalTransfer.toFixed(2)}</span>
  </div>
  <button onclick={onThermal} disabled={eroding}>
    Apply Thermal
  </button>

  <div class="subsection-title" style="margin-top: 12px;">Hydraulic</div>
  <div class="control-row">
    <label for="hydro-drops">Droplets</label>
    <input id="hydro-drops" type="range" min="10000" max="500000" step="10000" bind:value={numDroplets} />
    <span class="value">{(numDroplets / 1000).toFixed(0)}K</span>
  </div>
  <div class="control-row">
    <label for="hydro-erosion">Erosion</label>
    <input id="hydro-erosion" type="range" min="0.05" max="1.0" step="0.05" bind:value={erosionRate} />
    <span class="value">{erosionRate.toFixed(2)}</span>
  </div>
  <div class="control-row">
    <label for="hydro-deposit">Deposit</label>
    <input id="hydro-deposit" type="range" min="0.05" max="1.0" step="0.05" bind:value={depositionRate} />
    <span class="value">{depositionRate.toFixed(2)}</span>
  </div>
  <div class="control-row">
    <label for="hydro-inertia">Inertia</label>
    <input id="hydro-inertia" type="range" min="0.0" max="1.0" step="0.05" bind:value={inertia} />
    <span class="value">{inertia.toFixed(2)}</span>
  </div>

  {#if eroding}
    <div class="progress-bar">
      <div class="progress-fill" style="width: {erosionProgress * 100}%"></div>
    </div>
    <button onclick={onAbort}>Cancel</button>
  {:else}
    <button onclick={onHydraulic}>Apply Hydraulic</button>
  {/if}
</div>

<script lang="ts">
  import type { ThermalParams, HydraulicParams } from "../types";

  let {
    eroding = false,
    erosionProgress = 0,
    onThermalErode,
    onHydraulicErode,
    onAbortErosion,
  }: {
    eroding: boolean;
    erosionProgress: number;
    onThermalErode: (params: ThermalParams) => void;
    onHydraulicErode: (params: HydraulicParams) => void;
    onAbortErosion: () => void;
  } = $props();

  let thermalIterations = $state(10);
  let thermalTalus = $state(0.6);
  let thermalTransfer = $state(0.3);

  let numDroplets = $state(100000);
  let erosionRate = $state(0.3);
  let depositionRate = $state(0.3);
  let inertia = $state(0.3);

  export function getSettings() {
    return { thermalIterations, thermalTalus, thermalTransfer, numDroplets, erosionRate, depositionRate, inertia };
  }

  export function setSettings(s: { thermalIterations: number; thermalTalus: number; thermalTransfer: number; numDroplets: number; erosionRate: number; depositionRate: number; inertia: number }) {
    thermalIterations = s.thermalIterations;
    thermalTalus = s.thermalTalus;
    thermalTransfer = s.thermalTransfer;
    numDroplets = s.numDroplets;
    erosionRate = s.erosionRate;
    depositionRate = s.depositionRate;
    inertia = s.inertia;
  }

  function onThermal() {
    onThermalErode({
      iterations: thermalIterations,
      talus: thermalTalus,
      transferRate: thermalTransfer,
    });
  }

  function onHydraulic() {
    onHydraulicErode({
      numDroplets,
      maxLifetime: 64,
      erosionRate,
      depositionRate,
      evaporationRate: 0.01,
      inertia,
      minSlope: 0.01,
      capacityFactor: 8.0,
      erosionRadius: 3,
      gravity: 4.0,
    });
  }

  function onAbort() {
    onAbortErosion();
  }
</script>
