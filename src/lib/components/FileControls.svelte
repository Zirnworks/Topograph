<div class="section">
  <div class="section-title">File</div>
  <button onclick={onSave}>Save Project</button>
  <button onclick={onLoad}>Open Project</button>
  <div class="subsection-title" style="margin-top: 8px;">Export Heightmap</div>
  <button onclick={() => onExport("png16")}>Export PNG (16-bit)</button>
  <button onclick={() => onExport("raw_f32")}>Export Raw f32</button>
</div>

<script lang="ts">
  let {
    onSave,
    onLoad,
    onExport,
  }: {
    onSave: () => void;
    onLoad: () => void;
    onExport: (format: string) => void;
  } = $props();
</script>
