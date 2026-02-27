use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

/// Locate the Python binary inside the ml/venv.
/// Falls back to system `python3` if venv doesn't exist.
fn python_bin(app_dir: &std::path::Path) -> PathBuf {
    let venv_python = app_dir.join("ml/venv/bin/python");
    if venv_python.exists() {
        venv_python
    } else {
        PathBuf::from("python3")
    }
}

/// Resolve the project root (where ml/ lives).
/// In dev mode this is the Topograph source root.
pub fn project_root(_app_handle: &tauri::AppHandle) -> PathBuf {
    // In dev mode, the resource dir points to src-tauri, so go up one level
    // But more reliably, use CARGO_MANIFEST_DIR at compile time or env
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // CARGO_MANIFEST_DIR = <project>/src-tauri, go up to <project>
    manifest_dir.parent().unwrap_or(&manifest_dir).to_path_buf()
}

/// Run depth estimation: takes a PNG image, returns raw f32 heightmap data.
pub fn run_depth_estimation(
    app_handle: &tauri::AppHandle,
    image_data: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<f32>, String> {
    let root = project_root(app_handle);
    let python = python_bin(&root);
    let script = root.join("ml/depth_estimate.py");

    if !script.exists() {
        return Err(format!("Depth estimation script not found: {}", script.display()));
    }

    // Write input PNG to temp file
    let tmp_dir = std::env::temp_dir().join("topograph");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let input_path = tmp_dir.join("depth_input.png");
    let output_path = tmp_dir.join("depth_output.bin");

    std::fs::write(&input_path, image_data)
        .map_err(|e| format!("Failed to write input PNG: {e}"))?;

    // Spawn Python subprocess
    let output = Command::new(&python)
        .arg(&script)
        .arg("--input")
        .arg(&input_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--width")
        .arg(width.to_string())
        .arg("--height")
        .arg(height.to_string())
        .output()
        .map_err(|e| format!("Failed to spawn Python: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Depth estimation failed (exit code {:?}):\nstdout: {stdout}\nstderr: {stderr}",
            output.status.code()
        ));
    }

    // Parse JSON status from stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let status: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse Python output: {e}\nRaw: {stdout}"))?;

    if status["success"] != true {
        let error = status["error"].as_str().unwrap_or("Unknown error");
        return Err(format!("Depth estimation error: {error}"));
    }

    // Read output binary (f32 array, row-major, little-endian)
    let mut file = std::fs::File::open(&output_path)
        .map_err(|e| format!("Failed to open depth output: {e}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read depth output: {e}"))?;

    let expected_len = (width * height) as usize * 4;
    if bytes.len() != expected_len {
        return Err(format!(
            "Depth output size mismatch: got {} bytes, expected {expected_len}",
            bytes.len()
        ));
    }

    // Convert bytes to f32 array
    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    // Cleanup temp files (best effort)
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    Ok(floats)
}

/// Run inpainting: takes terrain PNG + mask PNG + prompt, returns inpainted PNG bytes.
pub fn run_inpainting(
    app_handle: &tauri::AppHandle,
    image_data: &[u8],
    mask_data: &[u8],
    prompt: &str,
    mode: &str,
) -> Result<Vec<u8>, String> {
    let root = project_root(app_handle);
    let python = python_bin(&root);
    let script = root.join("ml/inpaint.py");

    if !script.exists() {
        return Err(format!("Inpainting script not found: {}", script.display()));
    }

    let tmp_dir = std::env::temp_dir().join("topograph");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let image_path = tmp_dir.join("inpaint_image.png");
    let mask_path = tmp_dir.join("inpaint_mask.png");
    let output_path = tmp_dir.join("inpaint_output.png");

    std::fs::write(&image_path, image_data)
        .map_err(|e| format!("Failed to write image: {e}"))?;
    std::fs::write(&mask_path, mask_data)
        .map_err(|e| format!("Failed to write mask: {e}"))?;

    let output = Command::new(&python)
        .arg(&script)
        .arg("--image")
        .arg(&image_path)
        .arg("--mask")
        .arg(&mask_path)
        .arg("--prompt")
        .arg(prompt)
        .arg("--output")
        .arg(&output_path)
        .arg("--mode")
        .arg(mode)
        .output()
        .map_err(|e| format!("Failed to spawn Python: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Inpainting failed:\nstdout: {stdout}\nstderr: {stderr}"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let status: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse Python output: {e}\nRaw: {stdout}"))?;

    if status["success"] != true {
        let error = status["error"].as_str().unwrap_or("Unknown error");
        return Err(format!("Inpainting error: {error}"));
    }

    let result_bytes = std::fs::read(&output_path)
        .map_err(|e| format!("Failed to read inpainting output: {e}"))?;

    // Cleanup
    let _ = std::fs::remove_file(&image_path);
    let _ = std::fs::remove_file(&mask_path);
    let _ = std::fs::remove_file(&output_path);

    Ok(result_bytes)
}

/// Decode a PNG mask image (grayscale) into per-pixel f32 weights [0.0, 1.0].
/// White (255) = 1.0, Black (0) = 0.0.
pub fn decode_mask_png(png_data: &[u8], width: u32, height: u32) -> Result<Vec<f32>, String> {
    // Minimal PNG decode: write to temp, use Python to convert, or decode manually.
    // Use the simplest approach: save PNG, run a tiny Python script to output raw f32.
    let tmp_dir = std::env::temp_dir().join("topograph");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let mask_path = tmp_dir.join("mask_decode.png");
    let output_path = tmp_dir.join("mask_decode.bin");

    std::fs::write(&mask_path, png_data)
        .map_err(|e| format!("Failed to write mask: {e}"))?;

    // Find python
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent().unwrap_or(&manifest_dir);
    let python = python_bin(root);

    let output = Command::new(&python)
        .arg("-c")
        .arg(format!(
            "import numpy as np; from PIL import Image; \
             m = np.array(Image.open('{}').convert('L').resize(({}, {})), dtype=np.float32) / 255.0; \
             m.tofile('{}')",
            mask_path.display(), width, height, output_path.display()
        ))
        .output()
        .map_err(|e| format!("Failed to decode mask: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Mask decode failed: {stderr}"));
    }

    let bytes = std::fs::read(&output_path)
        .map_err(|e| format!("Failed to read decoded mask: {e}"))?;

    let _ = std::fs::remove_file(&mask_path);
    let _ = std::fs::remove_file(&output_path);

    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    Ok(floats)
}

/// Apply Gaussian feathering to a mask to smooth edges.
/// `radius` controls the feathering distance in pixels.
pub fn feather_mask(mask: &[f32], width: u32, height: u32, radius: u32) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let r = radius as i32;

    // Two-pass separable box blur (approximates Gaussian, fast)
    // Pass 1: horizontal
    let mut temp = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dx in -r..=r {
                let nx = x as i32 + dx;
                if nx >= 0 && nx < w as i32 {
                    sum += mask[y * w + nx as usize];
                    count += 1.0;
                }
            }
            temp[y * w + x] = sum / count;
        }
    }

    // Pass 2: vertical
    let mut result = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dy in -r..=r {
                let ny = y as i32 + dy;
                if ny >= 0 && ny < h as i32 {
                    sum += temp[ny as usize * w + x];
                    count += 1.0;
                }
            }
            result[y * w + x] = sum / count;
        }
    }

    result
}
