<div class="sidebar">
  {@render children()}
</div>

<script lang="ts">
  import type { Snippet } from "svelte";
  let { children }: { children: Snippet } = $props();
</script>
