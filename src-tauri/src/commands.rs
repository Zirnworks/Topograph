use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::ipc::Response;
use tauri::{AppHandle, State};
use crate::ai;
use crate::erosion::{hydraulic, thermal};
use crate::erosion::hydraulic::HydraulicParams;
use crate::erosion::thermal::ThermalParams;
use crate::ipc;
use crate::noise_gen::{self, NoiseParams};
use crate::sculpt::{self, BrushStroke};
use crate::state::AppState;

#[tauri::command]
pub fn get_heightmap(state: State<'_, AppState>) -> Response {
    let hm = state.heightmap.lock().unwrap();
    Response::new(ipc::pack_full(&hm))
}

#[tauri::command]
pub fn apply_brush_stroke(stroke: BrushStroke, state: State<'_, AppState>) -> Response {
    let mut hm = state.heightmap.lock().unwrap();
    let (rx, ry, rw, rh) = sculpt::apply_brush(&mut hm, &stroke);
    if rw == 0 || rh == 0 {
        return Response::new(ipc::pack_full(&hm));
    }
    Response::new(ipc::pack_region(&hm, rx, ry, rw, rh))
}

#[tauri::command]
pub fn generate_terrain(params: NoiseParams, state: State<'_, AppState>) -> Response {
    let mut hm = state.heightmap.lock().unwrap();
    noise_gen::generate_terrain(&mut hm, &params);
    Response::new(ipc::pack_full(&hm))
}

#[tauri::command]
pub fn run_thermal_erosion(params: ThermalParams, state: State<'_, AppState>) -> Response {
    let mut hm = state.heightmap.lock().unwrap();
    thermal::erode(&mut hm, &params);
    Response::new(ipc::pack_full(&hm))
}

#[tauri::command]
pub fn run_hydraulic_erosion(
    params: HydraulicParams,
    state: State<'_, AppState>,
    channel: tauri::ipc::Channel<f32>,
) -> Result<(), String> {
    if state
        .erosion_running
        .swap(true, Ordering::SeqCst)
    {
        return Err("Erosion already running".to_string());
    }
    state.erosion_abort.store(false, Ordering::SeqCst);

    let hm = Arc::clone(&state.heightmap);
    let abort = Arc::clone(&state.erosion_abort);
    let running = Arc::clone(&state.erosion_running);

    std::thread::spawn(move || {
        {
            let mut hm_guard = hm.lock().unwrap();
            hydraulic::erode(&mut hm_guard, &params, &abort, &|progress| {
                let _ = channel.send(progress);
            });
        }
        running.store(false, Ordering::SeqCst);
    });

    Ok(())
}

#[tauri::command]
pub fn abort_erosion(state: State<'_, AppState>) {
    state.erosion_abort.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn run_depth_estimation(
    image_data: Vec<u8>,
    mask_data: Option<Vec<u8>>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Response, String> {
    let hm_lock = state.heightmap.lock().unwrap();
    let width = hm_lock.width;
    let height = hm_lock.height;
    drop(hm_lock);

    let depth_values = ai::run_depth_estimation(&app_handle, &image_data, width, height)?;

    let mut hm = state.heightmap.lock().unwrap();
    if depth_values.len() != hm.data.len() {
        return Err(format!(
            "Depth data length mismatch: {} vs {}",
            depth_values.len(),
            hm.data.len()
        ));
    }

    match mask_data {
        Some(mask_png) => {
            // Decode the mask PNG to get per-pixel weights
            let mask = ai::decode_mask_png(&mask_png, width, height)?;

            // Find the height range of the original terrain in the masked region
            // so we can scale the depth values to match
            let mut masked_min = f32::MAX;
            let mut masked_max = f32::MIN;
            for i in 0..hm.data.len() {
                if mask[i] > 0.1 {
                    masked_min = masked_min.min(hm.data[i]);
                    masked_max = masked_max.max(hm.data[i]);
                }
            }
            // Also sample a border ring around the mask to get surrounding height context
            if masked_min > masked_max {
                masked_min = 0.0;
                masked_max = 1.0;
            }
            // Add some headroom so AI can create features above/below existing terrain
            let range = (masked_max - masked_min).max(0.05);
            let target_min = (masked_min - range * 0.3).max(0.0);
            let target_max = (masked_max + range * 0.3).min(1.0);

            // Find depth range in masked area
            let mut depth_min = f32::MAX;
            let mut depth_max = f32::MIN;
            for i in 0..depth_values.len() {
                if mask[i] > 0.1 {
                    depth_min = depth_min.min(depth_values[i]);
                    depth_max = depth_max.max(depth_values[i]);
                }
            }
            let depth_range = (depth_max - depth_min).max(1e-6);

            // Blend: remap depth to target range, mix with original using mask weight
            // Apply Gaussian feathering at mask edges
            let feathered_mask = ai::feather_mask(&mask, width, height, 8);
            for i in 0..hm.data.len() {
                let w = feathered_mask[i];
                if w > 0.001 {
                    // Remap depth to match surrounding terrain height range
                    let normalized = (depth_values[i] - depth_min) / depth_range;
                    let remapped = target_min + normalized * (target_max - target_min);
                    hm.data[i] = hm.data[i] * (1.0 - w) + remapped * w;
                }
            }
        }
        None => {
            // No mask — replace entire heightmap (legacy behavior)
            hm.data.copy_from_slice(&depth_values);
        }
    }

    Ok(Response::new(ipc::pack_full(&hm)))
}

#[tauri::command]
pub fn run_inpainting(
    image_data: Vec<u8>,
    mask_data: Vec<u8>,
    prompt: String,
    app_handle: AppHandle,
) -> Result<Vec<u8>, String> {
    ai::run_inpainting(&app_handle, &image_data, &mask_data, &prompt)
}
