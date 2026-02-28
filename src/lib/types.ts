export interface HeightmapData {
  width: number;
  height: number;
  data: Float32Array;
}

export interface HeightmapRegion {
  x: number;
  y: number;
  w: number;
  h: number;
  data: Float32Array;
}

export type BrushOp = "raise" | "lower" | "smooth" | "flatten";

export interface BrushStroke {
  x: number;
  y: number;
  radius: number;
  strength: number;
  op: BrushOp;
}

export interface NoiseParams {
  noiseType: "perlin" | "simplex";
  seed: number;
  octaves: number;
  frequency: number;
  lacunarity: number;
  persistence: number;
  amplitude: number;
  offset: number;
}

export interface ThermalParams {
  iterations: number;
  talus: number;
  transferRate: number;
}

export interface HydraulicParams {
  numDroplets: number;
  maxLifetime: number;
  erosionRate: number;
  depositionRate: number;
  evaporationRate: number;
  inertia: number;
  minSlope: number;
  capacityFactor: number;
  erosionRadius: number;
  gravity: number;
}

export type AISculptMode = "texture" | "heightmap" | "texture_gen";
export type AIStatus = "idle" | "running" | "error";

export interface ProjectSettings {
  version: 1;
  brush: {
    op: BrushOp;
    radius: number;
    strength: number;
  };
  generation: {
    noiseType: "perlin" | "simplex";
    seed: number;
    octaves: number;
    frequency: number;
    lacunarity: number;
    persistence: number;
    amplitude: number;
  };
  erosion: {
    thermalIterations: number;
    thermalTalus: number;
    thermalTransfer: number;
    numDroplets: number;
    erosionRate: number;
    depositionRate: number;
    inertia: number;
  };
}

export interface LoadProjectResponse {
  texturePng: number[] | null;
  settingsJson: string;
}

