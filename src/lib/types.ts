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
