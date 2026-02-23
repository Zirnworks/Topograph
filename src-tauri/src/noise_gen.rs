use noise::{NoiseFn, Perlin, OpenSimplex};
use serde::Deserialize;
use crate::heightmap::Heightmap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoiseType {
    Perlin,
    Simplex,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseParams {
    pub noise_type: NoiseType,
    pub seed: u32,
    pub octaves: u32,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
    pub amplitude: f64,
    pub offset: f64,
}

pub fn generate_terrain(hm: &mut Heightmap, params: &NoiseParams) {
    match params.noise_type {
        NoiseType::Perlin => {
            let source = Perlin::new(params.seed);
            fill_heightmap(hm, &source, params);
        }
        NoiseType::Simplex => {
            let source = OpenSimplex::new(params.seed);
            fill_heightmap(hm, &source, params);
        }
    }
}

fn fill_heightmap(hm: &mut Heightmap, source: &impl NoiseFn<f64, 2>, params: &NoiseParams) {
    for y in 0..hm.height {
        for x in 0..hm.width {
            let nx = x as f64 / hm.width as f64;
            let ny = y as f64 / hm.height as f64;

            let val = fbm(source, nx, ny, params);
            let normalized = (val * params.amplitude + params.offset).clamp(0.0, 1.0);
            hm.set(x, y, normalized as f32);
        }
    }
}

fn fbm(source: &impl NoiseFn<f64, 2>, x: f64, y: f64, params: &NoiseParams) -> f64 {
    let mut freq = params.frequency;
    let mut amp = 1.0;
    let mut max_amp = 0.0;
    let mut value = 0.0;

    for _ in 0..params.octaves {
        value += source.get([x * freq, y * freq]) * amp;
        max_amp += amp;
        freq *= params.lacunarity;
        amp *= params.persistence;
    }

    if max_amp > 0.0 {
        value / max_amp
    } else {
        0.0
    }
}
