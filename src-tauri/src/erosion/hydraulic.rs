use rand::Rng;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::heightmap::Heightmap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HydraulicParams {
    pub num_droplets: u32,
    pub max_lifetime: u32,
    pub erosion_rate: f32,
    pub deposition_rate: f32,
    pub evaporation_rate: f32,
    pub inertia: f32,
    pub min_slope: f32,
    pub capacity_factor: f32,
    pub erosion_radius: u32,
    pub gravity: f32,
}

pub fn erode(
    hm: &mut Heightmap,
    params: &HydraulicParams,
    abort: &AtomicBool,
    progress: &dyn Fn(f32),
) {
    let mut rng = rand::thread_rng();
    let w = hm.width as f32;
    let h = hm.height as f32;
    let brush = compute_erosion_brush(params.erosion_radius as i32);

    for i in 0..params.num_droplets {
        if i % 1000 == 0 {
            if abort.load(Ordering::Relaxed) {
                return;
            }
            progress(i as f32 / params.num_droplets as f32);
        }

        let mut px = rng.gen::<f32>() * (w - 2.0) + 0.5;
        let mut py = rng.gen::<f32>() * (h - 2.0) + 0.5;
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        let mut speed = 1.0f32;
        let mut water = 1.0f32;
        let mut sediment = 0.0f32;

        for _ in 0..params.max_lifetime {
            let (gx, gy, h_here) = gradient_at(hm, px, py);

            dx = dx * params.inertia - gx * (1.0 - params.inertia);
            dy = dy * params.inertia - gy * (1.0 - params.inertia);

            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-6 {
                let angle = rng.gen::<f32>() * std::f32::consts::TAU;
                dx = angle.cos();
                dy = angle.sin();
            } else {
                dx /= len;
                dy /= len;
            }

            let new_px = px + dx;
            let new_py = py + dy;

            if new_px < 0.5 || new_px >= w - 1.5 || new_py < 0.5 || new_py >= h - 1.5 {
                break;
            }

            let h_new = interpolate_height(hm, new_px, new_py);
            let h_diff = h_new - h_here;

            let capacity = (-h_diff).max(params.min_slope) * speed * water * params.capacity_factor;

            if sediment > capacity || h_diff > 0.0 {
                let deposit = if h_diff > 0.0 {
                    sediment.min(h_diff)
                } else {
                    (sediment - capacity) * params.deposition_rate
                };
                sediment -= deposit;
                deposit_at(hm, px, py, deposit);
            } else {
                let erode_amount =
                    ((capacity - sediment) * params.erosion_rate).min(-h_diff);
                erode_at(hm, px, py, erode_amount, &brush);
                sediment += erode_amount;
            }

            speed = (speed * speed + h_diff * params.gravity).max(0.0).sqrt();
            water *= 1.0 - params.evaporation_rate;
            px = new_px;
            py = new_py;
        }
    }

    progress(1.0);
}

fn interpolate_height(hm: &Heightmap, x: f32, y: f32) -> f32 {
    let ix = x as u32;
    let iy = y as u32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;

    let tl = hm.get(ix, iy);
    let tr = hm.get((ix + 1).min(hm.width - 1), iy);
    let bl = hm.get(ix, (iy + 1).min(hm.height - 1));
    let br = hm.get(
        (ix + 1).min(hm.width - 1),
        (iy + 1).min(hm.height - 1),
    );

    let top = tl + (tr - tl) * fx;
    let bot = bl + (br - bl) * fx;
    top + (bot - top) * fy
}

fn gradient_at(hm: &Heightmap, x: f32, y: f32) -> (f32, f32, f32) {
    let ix = x as u32;
    let iy = y as u32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;

    let tl = hm.get(ix, iy);
    let tr = hm.get((ix + 1).min(hm.width - 1), iy);
    let bl = hm.get(ix, (iy + 1).min(hm.height - 1));
    let br = hm.get(
        (ix + 1).min(hm.width - 1),
        (iy + 1).min(hm.height - 1),
    );

    let gx = (tr - tl) * (1.0 - fy) + (br - bl) * fy;
    let gy = (bl - tl) * (1.0 - fx) + (br - tr) * fx;
    let height = tl + (tr - tl) * fx + (bl - tl) * fy + (tl - tr - bl + br) * fx * fy;

    (gx, gy, height)
}

fn deposit_at(hm: &mut Heightmap, x: f32, y: f32, amount: f32) {
    let ix = x as u32;
    let iy = y as u32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;

    let w = hm.width;
    let h = hm.height;

    // Bilinear distribution
    let weights = [
        ((1.0 - fx) * (1.0 - fy), ix, iy),
        (fx * (1.0 - fy), (ix + 1).min(w - 1), iy),
        ((1.0 - fx) * fy, ix, (iy + 1).min(h - 1)),
        (fx * fy, (ix + 1).min(w - 1), (iy + 1).min(h - 1)),
    ];

    for &(weight, cx, cy) in &weights {
        let idx = (cy * w + cx) as usize;
        hm.data[idx] += amount * weight;
    }
}

fn erode_at(hm: &mut Heightmap, x: f32, y: f32, amount: f32, brush: &[(i32, i32, f32)]) {
    let ix = x.round() as i32;
    let iy = y.round() as i32;
    let w = hm.width as i32;
    let h = hm.height as i32;

    for &(bx, by, weight) in brush {
        let cx = ix + bx;
        let cy = iy + by;
        if cx >= 0 && cx < w && cy >= 0 && cy < h {
            let idx = (cy * w + cx) as usize;
            hm.data[idx] -= amount * weight;
        }
    }
}

fn compute_erosion_brush(radius: i32) -> Vec<(i32, i32, f32)> {
    let mut offsets = Vec::new();
    let mut total_weight = 0.0f32;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            if dist <= radius as f32 {
                let weight = 1.0 - dist / (radius as f32 + 1.0);
                offsets.push((dx, dy, weight));
                total_weight += weight;
            }
        }
    }

    // Normalize weights
    if total_weight > 0.0 {
        for entry in &mut offsets {
            entry.2 /= total_weight;
        }
    }

    offsets
}
