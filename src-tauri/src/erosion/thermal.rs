use serde::Deserialize;
use crate::heightmap::Heightmap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThermalParams {
    pub iterations: u32,
    pub talus: f32,
    pub transfer_rate: f32,
}

pub fn erode(hm: &mut Heightmap, params: &ThermalParams) {
    let w = hm.width as i32;
    let h = hm.height as i32;
    let cell_size = 1.0 / w as f32;

    let neighbors: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    for _ in 0..params.iterations {
        let snapshot = hm.data.clone();

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let center = snapshot[idx];

                let mut total_diff = 0.0f32;
                let mut max_diff = 0.0f32;
                let mut diffs: [(f32, i32, i32); 4] = [(0.0, 0, 0); 4];
                let mut n_lower = 0usize;

                for &(dx, dy) in &neighbors {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || nx >= w || ny < 0 || ny >= h {
                        continue;
                    }
                    let nidx = (ny * w + nx) as usize;
                    let diff = center - snapshot[nidx];
                    let slope = diff / cell_size;
                    if slope > params.talus {
                        diffs[n_lower] = (diff, dx, dy);
                        total_diff += diff;
                        if diff > max_diff {
                            max_diff = diff;
                        }
                        n_lower += 1;
                    }
                }

                if n_lower == 0 {
                    continue;
                }

                let excess = (max_diff - params.talus * cell_size) * params.transfer_rate;
                for i in 0..n_lower {
                    let (diff, dx, dy) = diffs[i];
                    let proportion = diff / total_diff;
                    let transfer = excess * proportion;
                    let nidx = ((y + dy) * w + (x + dx)) as usize;
                    hm.data[idx] -= transfer;
                    hm.data[nidx] += transfer;
                }
            }
        }
    }
}
