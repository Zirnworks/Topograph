use std::io::{Read, Write};
use std::path::Path;
use std::time::SystemTime;
use zip::write::SimpleFileOptions;
use zip::{ZipWriter, ZipArchive, CompressionMethod};
use serde::{Deserialize, Serialize};
use crate::heightmap::Heightmap;

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectManifest {
    format_version: u32,
    app_version: String,
    width: u32,
    height: u32,
    created_at: u64,
    has_texture: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadProjectResponse {
    pub texture_png: Option<Vec<u8>>,
    pub settings_json: String,
}

pub fn save_project(
    path: &Path,
    heightmap: &Heightmap,
    texture_png: Option<&[u8]>,
    settings_json: &str,
) -> Result<(), String> {
    let file = std::fs::File::create(path)
        .map_err(|e| format!("Failed to create file: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let deflate = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored);

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 1. manifest.json
    let manifest = ProjectManifest {
        format_version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        width: heightmap.width,
        height: heightmap.height,
        created_at: timestamp,
        has_texture: texture_png.is_some(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
    zip.start_file("manifest.json", deflate)
        .map_err(|e| format!("ZIP error: {e}"))?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|e| format!("Write error: {e}"))?;

    // 2. heightmap.bin (raw f32 LE)
    zip.start_file("heightmap.bin", deflate)
        .map_err(|e| format!("ZIP error: {e}"))?;
    for &val in &heightmap.data {
        zip.write_all(&val.to_le_bytes())
            .map_err(|e| format!("Write error: {e}"))?;
    }

    // 3. texture.png (optional, already compressed)
    if let Some(png_data) = texture_png {
        zip.start_file("texture.png", stored)
            .map_err(|e| format!("ZIP error: {e}"))?;
        zip.write_all(png_data)
            .map_err(|e| format!("Write error: {e}"))?;
    }

    // 4. settings.json
    zip.start_file("settings.json", deflate)
        .map_err(|e| format!("ZIP error: {e}"))?;
    zip.write_all(settings_json.as_bytes())
        .map_err(|e| format!("Write error: {e}"))?;

    zip.finish().map_err(|e| format!("ZIP finish error: {e}"))?;
    Ok(())
}

pub fn load_project(path: &Path) -> Result<(Heightmap, Option<Vec<u8>>, String), String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| format!("Invalid .topo file: {e}"))?;

    // 1. Read manifest
    let manifest: ProjectManifest = {
        let mut entry = zip.by_name("manifest.json")
            .map_err(|_| "Missing manifest.json in .topo file".to_string())?;
        let mut buf = String::new();
        entry.read_to_string(&mut buf)
            .map_err(|e| format!("Read error: {e}"))?;
        serde_json::from_str(&buf)
            .map_err(|e| format!("Invalid manifest: {e}"))?
    };

    if manifest.format_version > FORMAT_VERSION {
        return Err(format!(
            "Project version {} is newer than supported version {}",
            manifest.format_version, FORMAT_VERSION
        ));
    }

    // 2. Read heightmap.bin
    let heightmap = {
        let mut entry = zip.by_name("heightmap.bin")
            .map_err(|_| "Missing heightmap.bin in .topo file".to_string())?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)
            .map_err(|e| format!("Read error: {e}"))?;

        let expected = (manifest.width * manifest.height) as usize * 4;
        if bytes.len() != expected {
            return Err(format!(
                "Heightmap size mismatch: got {} bytes, expected {expected}",
                bytes.len()
            ));
        }

        let data: Vec<f32> = bytes.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        Heightmap { data, width: manifest.width, height: manifest.height }
    };

    // 3. Read texture.png (optional)
    let texture_png = if manifest.has_texture {
        match zip.by_name("texture.png") {
            Ok(mut entry) => {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)
                    .map_err(|e| format!("Read error: {e}"))?;
                Some(buf)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // 4. Read settings.json
    let settings_json = match zip.by_name("settings.json") {
        Ok(mut entry) => {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)
                .map_err(|e| format!("Read error: {e}"))?;
            buf
        }
        Err(_) => "{}".to_string(),
    };

    Ok((heightmap, texture_png, settings_json))
}

pub fn export_heightmap_png16(path: &Path, heightmap: &Heightmap) -> Result<(), String> {
    let w = heightmap.width;
    let h = heightmap.height;

    let pixels: Vec<u16> = heightmap.data.iter()
        .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
        .collect();

    let img = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::from_raw(w, h, pixels)
        .ok_or("Failed to create image buffer".to_string())?;

    img.save(path).map_err(|e| format!("Failed to save PNG: {e}"))?;
    Ok(())
}

pub fn export_heightmap_raw(path: &Path, heightmap: &Heightmap) -> Result<(), String> {
    let bytes: Vec<u8> = heightmap.data.iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();

    std::fs::write(path, &bytes)
        .map_err(|e| format!("Failed to write raw file: {e}"))?;
    Ok(())
}
