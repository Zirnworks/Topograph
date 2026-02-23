<div
  class="viewer"
  bind:this={container}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointerleave={onPointerUp}
></div>

<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as THREE from "three";
  import { SceneManager } from "../rendering/scene";
  import { TerrainRenderer } from "../rendering/terrain-mesh";
  import { applyBrushStroke, isRegion } from "../tauri";
  import type { HeightmapData, HeightmapRegion, BrushOp } from "../types";

  let {
    brushOp = "raise" as BrushOp,
    brushRadius = 25,
    brushStrength = 0.5,
  } = $props();

  let container: HTMLElement;
  let sceneManager: SceneManager | null = null;
  let terrainRenderer = new TerrainRenderer();
  let resizeObserver: ResizeObserver | null = null;

  // Sculpting state
  let painting = false;
  let rafId = 0;
  let pendingPos: { x: number; y: number } | null = null;
  let ipcInFlight = false;

  // Brush cursor
  let brushCursor: THREE.Mesh | null = null;
  const raycaster = new THREE.Raycaster();
  const mouseNDC = new THREE.Vector2();

  export function buildTerrain(data: HeightmapData) {
    if (!sceneManager) return;
    terrainRenderer.buildFull(data, sceneManager.scene);
    setupBrushCursor();
  }

  export function updateRegion(region: HeightmapRegion) {
    terrainRenderer.updateRegion(region);
  }

  export function rebuildFromFull(data: HeightmapData) {
    if (!sceneManager) return;
    terrainRenderer.buildFull(data, sceneManager.scene);
    setupBrushCursor();
  }

  function setupBrushCursor() {
    if (!sceneManager) return;
    if (brushCursor) {
      sceneManager.scene.remove(brushCursor);
      brushCursor.geometry.dispose();
      (brushCursor.material as THREE.Material).dispose();
    }
    const geo = new THREE.RingGeometry(0.8, 1, 64);
    geo.rotateX(-Math.PI / 2);
    const mat = new THREE.MeshBasicMaterial({
      color: 0xffffff,
      opacity: 0.4,
      transparent: true,
      side: THREE.DoubleSide,
      depthWrite: false,
    });
    brushCursor = new THREE.Mesh(geo, mat);
    brushCursor.visible = false;
    brushCursor.renderOrder = 999;
    sceneManager.scene.add(brushCursor);
  }

  function screenToTerrainHit(
    event: PointerEvent
  ): THREE.Intersection | null {
    if (!sceneManager || !terrainRenderer.getMesh()) return null;
    const rect = container.getBoundingClientRect();
    mouseNDC.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    mouseNDC.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    raycaster.setFromCamera(mouseNDC, sceneManager.camera);
    const hits = raycaster.intersectObject(terrainRenderer.getMesh()!);
    return hits.length > 0 ? hits[0] : null;
  }

  function worldToHeightmap(point: THREE.Vector3): { x: number; y: number } {
    const dims = terrainRenderer.getDimensions();
    return {
      x: (point.x + 0.5) * (dims.width - 1),
      y: (point.z + 0.5) * (dims.height - 1),
    };
  }

  function updateBrushCursor(hit: THREE.Intersection | null) {
    if (!brushCursor) return;
    if (!hit) {
      brushCursor.visible = false;
      return;
    }
    brushCursor.visible = true;
    brushCursor.position.copy(hit.point);
    brushCursor.position.y += 0.001; // slight offset to avoid z-fighting
    const dims = terrainRenderer.getDimensions();
    const worldRadius = brushRadius / (dims.width - 1);
    brushCursor.scale.setScalar(worldRadius);
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return; // Left click only

    const hit = screenToTerrainHit(e);
    if (!hit) return;

    painting = true;
    container.setPointerCapture(e.pointerId);
    sendStroke(hit);
  }

  function onPointerMove(e: PointerEvent) {
    const hit = screenToTerrainHit(e);
    updateBrushCursor(hit);

    if (!painting) return;
    if (!hit) return;

    const hmPos = worldToHeightmap(hit.point);
    pendingPos = hmPos;

    if (!rafId) {
      rafId = requestAnimationFrame(flushStroke);
    }
  }

  function onPointerUp(_e: PointerEvent) {
    if (!painting) return;
    painting = false;
    pendingPos = null;
  }

  async function sendStroke(hit: THREE.Intersection) {
    const hmPos = worldToHeightmap(hit.point);
    await doStroke(hmPos);
  }

  async function flushStroke() {
    rafId = 0;
    if (!pendingPos || ipcInFlight) return;
    const pos = pendingPos;
    pendingPos = null;
    await doStroke(pos);
  }

  async function doStroke(pos: { x: number; y: number }) {
    ipcInFlight = true;
    try {
      const result = await applyBrushStroke({
        x: pos.x,
        y: pos.y,
        radius: brushRadius,
        strength: brushStrength,
        op: brushOp,
      });
      if (isRegion(result)) {
        terrainRenderer.updateRegion(result);
      } else {
        if (sceneManager) {
          terrainRenderer.buildFull(result, sceneManager.scene);
          setupBrushCursor();
        }
      }
    } finally {
      ipcInFlight = false;
      // If there's a pending stroke queued while we were in-flight, flush it
      if (pendingPos && painting) {
        rafId = requestAnimationFrame(flushStroke);
      }
    }
  }

  onMount(() => {
    sceneManager = new SceneManager(container);
    sceneManager.start();

    // OrbitControls configured in scene.ts: right-click = orbit, middle = pan
    // Left-click is exclusively for sculpting

    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          sceneManager?.resize(width, height);
        }
      }
    });
    resizeObserver.observe(container);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    if (sceneManager) {
      if (brushCursor) {
        sceneManager.scene.remove(brushCursor);
        brushCursor.geometry.dispose();
        (brushCursor.material as THREE.Material).dispose();
      }
      terrainRenderer.dispose(sceneManager.scene);
      sceneManager.dispose();
    }
  });
</script>

<style>
  .viewer {
    width: 100%;
    height: 100%;
    cursor: crosshair;
  }
</style>
