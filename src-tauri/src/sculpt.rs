use serde::Deserialize;
use crate::heightmap::Heightmap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrushOp {
    Raise,
    Lower,
    Smooth,
    Flatten,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrushStroke {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub strength: f32,
    pub op: BrushOp,
}

/// Apply a brush stroke. Returns bounding box of affected region: (x, y, w, h).
pub fn apply_brush(hm: &mut Heightmap, stroke: &BrushStroke) -> (u32, u32, u32, u32) {
    let cx = stroke.x;
    let cy = stroke.y;
    let r = stroke.radius;

    let x0 = (cx - r).floor().max(0.0) as u32;
    let y0 = (cy - r).floor().max(0.0) as u32;
    let x1 = ((cx + r).ceil() as u32).min(hm.width - 1);
    let y1 = ((cy + r).ceil() as u32).min(hm.height - 1);

    if x0 > x1 || y0 > y1 {
        return (0, 0, 0, 0);
    }

    // For flatten: sample target height at brush center
    let flatten_target = if matches!(stroke.op, BrushOp::Flatten) {
        let ix = (cx.round() as u32).clamp(0, hm.width - 1);
        let iy = (cy.round() as u32).clamp(0, hm.height - 1);
        Some(hm.get(ix, iy))
    } else {
        None
    };

    // For smooth: snapshot heights so we read original values
    let smooth_snapshot = if matches!(stroke.op, BrushOp::Smooth) {
        Some(hm.data.clone())
    } else {
        None
    };

    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist_sq = dx * dx + dy * dy;
            let r_sq = r * r;
            if dist_sq > r_sq {
                continue;
            }

            let t = dist_sq / r_sq;
            let falloff = (-t * 3.0).exp(); // Gaussian falloff
            let influence = stroke.strength * falloff;

            let current = hm.get(px, py);
            let new_val = match stroke.op {
                BrushOp::Raise => current + influence * 0.02,
                BrushOp::Lower => current - influence * 0.02,
                BrushOp::Flatten => {
                    let target = flatten_target.unwrap();
                    current + (target - current) * influence
                }
                BrushOp::Smooth => {
                    let snap = smooth_snapshot.as_ref().unwrap();
                    let avg = sample_avg(snap, hm.width, hm.height, px, py);
                    current + (avg - current) * influence
                }
            };

            hm.set(px, py, new_val.clamp(0.0, 1.0));
        }
    }

    let rw = x1 - x0 + 1;
    let rh = y1 - y0 + 1;
    (x0, y0, rw, rh)
}

fn sample_avg(data: &[f32], w: u32, h: u32, x: u32, y: u32) -> f32 {
    let idx = |x: u32, y: u32| data[(y * w + x) as usize];
    let mut sum = idx(x, y);
    let mut count = 1.0f32;
    if x > 0 {
        sum += idx(x - 1, y);
        count += 1.0;
    }
    if x < w - 1 {
        sum += idx(x + 1, y);
        count += 1.0;
    }
    if y > 0 {
        sum += idx(x, y - 1);
        count += 1.0;
    }
    if y < h - 1 {
        sum += idx(x, y + 1);
        count += 1.0;
    }
    sum / count
}
