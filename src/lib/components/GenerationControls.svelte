<div class="section">
  <div class="section-title">Terrain Generation</div>
  <div class="control-row">
    <label for="noise-type">Type</label>
    <select id="noise-type" bind:value={noiseType}>
      <option value="perlin">Perlin</option>
      <option value="simplex">Simplex</option>
    </select>
  </div>
  <div class="control-row">
    <label for="seed">Seed</label>
    <input id="seed" type="range" min="0" max="9999" step="1" bind:value={seed} />
    <span class="value">{seed}</span>
  </div>
  <div class="control-row">
    <label for="octaves">Octaves</label>
    <input id="octaves" type="range" min="1" max="12" step="1" bind:value={octaves} />
    <span class="value">{octaves}</span>
  </div>
  <div class="control-row">
    <label for="frequency">Frequency</label>
    <input id="frequency" type="range" min="0.5" max="10.0" step="0.1" bind:value={frequency} />
    <span class="value">{frequency.toFixed(1)}</span>
  </div>
  <div class="control-row">
    <label for="lacunarity">Lacunarity</label>
    <input id="lacunarity" type="range" min="1.0" max="4.0" step="0.1" bind:value={lacunarity} />
    <span class="value">{lacunarity.toFixed(1)}</span>
  </div>
  <div class="control-row">
    <label for="persistence">Persistence</label>
    <input id="persistence" type="range" min="0.1" max="1.0" step="0.05" bind:value={persistence} />
    <span class="value">{persistence.toFixed(2)}</span>
  </div>
  <div class="control-row">
    <label for="amplitude">Amplitude</label>
    <input id="amplitude" type="range" min="0.1" max="2.0" step="0.1" bind:value={amplitude} />
    <span class="value">{amplitude.toFixed(1)}</span>
  </div>
  <button onclick={onGenerate} disabled={generating}>
    {generating ? "Generating..." : "Generate Terrain"}
  </button>
  <button onclick={onRandomize} disabled={generating}>
    Randomize
  </button>
</div>

<script lang="ts">
  import type { NoiseParams } from "../types";

  let { onGenerated }: { onGenerated: (params: NoiseParams) => void } = $props();

  let noiseType = $state<"perlin" | "simplex">("perlin");
  let seed = $state(42);
  let octaves = $state(6);
  let frequency = $state(3.0);
  let lacunarity = $state(2.0);
  let persistence = $state(0.5);
  let amplitude = $state(1.0);
  let generating = $state(false);

  function getParams(): NoiseParams {
    return {
      noiseType,
      seed,
      octaves,
      frequency,
      lacunarity,
      persistence,
      amplitude,
      offset: 0.5,
    };
  }

  function onGenerate() {
    generating = true;
    onGenerated(getParams());
    generating = false;
  }

  function onRandomize() {
    seed = Math.floor(Math.random() * 10000);
    frequency = 1.0 + Math.random() * 6.0;
    octaves = 3 + Math.floor(Math.random() * 7);
    persistence = 0.3 + Math.random() * 0.5;
    lacunarity = 1.5 + Math.random() * 1.5;
    amplitude = 0.5 + Math.random() * 1.0;
    onGenerate();
  }
</script>
