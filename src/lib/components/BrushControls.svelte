<div class="section">
  <div class="section-title">Brush</div>
  <div class="control-row">
    <label for="brush-tool">Tool</label>
    <select id="brush-tool" bind:value={brushOp}>
      <option value="raise">Raise</option>
      <option value="lower">Lower</option>
      <option value="smooth">Smooth</option>
      <option value="flatten">Flatten</option>
    </select>
  </div>
  <div class="control-row">
    <label for="brush-radius">Radius</label>
    <input id="brush-radius" type="range" min="5" max="100" step="1" bind:value={brushRadius} />
    <span class="value">{brushRadius}</span>
  </div>
  <div class="control-row">
    <label for="brush-strength">Strength</label>
    <input id="brush-strength" type="range" min="0.05" max="1.0" step="0.05" bind:value={brushStrength} />
    <span class="value">{brushStrength.toFixed(2)}</span>
  </div>
</div>

<script lang="ts">
  import type { BrushOp } from "../types";

  let {
    brushOp = $bindable("raise" as BrushOp),
    brushRadius = $bindable(25),
    brushStrength = $bindable(0.5),
  } = $props();
</script>
