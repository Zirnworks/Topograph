import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  HeightmapData,
  HeightmapRegion,
  BrushStroke,
  NoiseParams,
  ThermalParams,
  HydraulicParams,
} from "./types";

const IPC_VERSION = 1;
const MSG_FULL = 0;
const MSG_REGION = 1;

function parseResponse(buffer: ArrayBuffer): HeightmapData | HeightmapRegion {
  const view = new DataView(buffer);
  const version = view.getUint32(0, true);
  if (version !== IPC_VERSION)
    throw new Error(`IPC version mismatch: ${version}`);

  const type = view.getUint8(4);
  if (type === MSG_FULL) {
    const width = view.getUint32(8, true);
    const height = view.getUint32(12, true);
    const data = new Float32Array(buffer, 16, width * height);
    return { width, height, data };
  } else {
    const x = view.getUint32(8, true);
    const y = view.getUint32(12, true);
    const w = view.getUint32(16, true);
    const h = view.getUint32(20, true);
    const data = new Float32Array(buffer, 24, w * h);
    return { x, y, w, h, data };
  }
}

export function isRegion(
  r: HeightmapData | HeightmapRegion
): r is HeightmapRegion {
  return "x" in r;
}

export async function getHeightmap(): Promise<HeightmapData> {
  const buffer: ArrayBuffer = await invoke("get_heightmap");
  return parseResponse(buffer) as HeightmapData;
}

export async function applyBrushStroke(
  stroke: BrushStroke
): Promise<HeightmapData | HeightmapRegion> {
  const buffer: ArrayBuffer = await invoke("apply_brush_stroke", { stroke });
  return parseResponse(buffer);
}

export async function generateTerrain(
  params: NoiseParams
): Promise<HeightmapData> {
  const buffer: ArrayBuffer = await invoke("generate_terrain", { params });
  return parseResponse(buffer) as HeightmapData;
}

export async function runThermalErosion(
  params: ThermalParams
): Promise<HeightmapData> {
  const buffer: ArrayBuffer = await invoke("run_thermal_erosion", { params });
  return parseResponse(buffer) as HeightmapData;
}

export async function runHydraulicErosion(
  params: HydraulicParams,
  onProgress: (progress: number) => void
): Promise<void> {
  const channel = new Channel<number>();
  channel.onmessage = (progress) => {
    onProgress(progress);
  };
  await invoke("run_hydraulic_erosion", { params, channel });
}

export async function abortErosion(): Promise<void> {
  await invoke("abort_erosion");
}
