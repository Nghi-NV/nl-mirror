//! Screenshot saving utility

use crate::core::FrameData;
use chrono::Local;
use image::{ImageBuffer, Rgba};
use std::sync::Arc;

/// Convert YUV I420 frame to RGBA for saving as PNG
fn yuv_to_rgba(frame: &FrameData) -> Vec<u8> {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let mut rgba = vec![0u8; w * h * 4];

    for row in 0..h {
        let uv_row = row / 2;
        for col in 0..w {
            let y_idx = row * frame.y_stride + col;
            let uv_col = col / 2;
            let u_idx = uv_row * frame.uv_stride + uv_col;
            let v_idx = uv_row * frame.uv_stride + uv_col;

            let y = frame.y_plane[y_idx] as f32 / 255.0;
            let u = frame.u_plane[u_idx] as f32 / 255.0 - 0.5;
            let v = frame.v_plane[v_idx] as f32 / 255.0 - 0.5;

            // BT.601 YUV to RGB (full range)
            let r = (y + 1.402 * v).clamp(0.0, 1.0);
            let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 1.0);
            let b = (y + 1.772 * u).clamp(0.0, 1.0);

            let rgba_idx = (row * w + col) * 4;
            rgba[rgba_idx] = (r * 255.0) as u8;
            rgba[rgba_idx + 1] = (g * 255.0) as u8;
            rgba[rgba_idx + 2] = (b * 255.0) as u8;
            rgba[rgba_idx + 3] = 255;
        }
    }

    rgba
}

/// Save YUV frame data to a PNG file on Desktop
pub fn save_screenshot_yuv(frame: FrameData) {
    std::thread::spawn(move || {
        let width = frame.width;
        let height = frame.height;

        eprintln!("[SNAPSHOT] Converting YUV to RGBA...");
        let rgba = yuv_to_rgba(&frame);

        if rgba.is_empty() {
            eprintln!("[SNAPSHOT] Frame buffer empty, cannot save.");
            return;
        }

        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("screenshot_{}.png", timestamp);

        let save_path = match dirs::desktop_dir() {
            Some(mut path) => {
                path.push(&filename);
                path
            }
            None => {
                eprintln!("[SNAPSHOT] Could not find Desktop dir, falling back to current dir.");
                std::path::PathBuf::from(&filename)
            }
        };

        eprintln!(
            "[SNAPSHOT] Saving {}x{} to {:?}...",
            width, height, save_path
        );

        match ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba) {
            Some(buffer) => match buffer.save(&save_path) {
                Ok(_) => eprintln!("[SNAPSHOT] Saved to {:?}", save_path),
                Err(e) => eprintln!("[SNAPSHOT] Failed to save: {}", e),
            },
            None => eprintln!("[SNAPSHOT] Failed to create image buffer."),
        }
    });
}

/// Old RGBA-based screenshot (kept for compatibility)
#[allow(dead_code)]
pub fn save_screenshot(width: u32, height: u32, rgba: Arc<Vec<u8>>) {
    std::thread::spawn(move || {
        if rgba.is_empty() {
            eprintln!("[SNAPSHOT] Frame buffer empty, cannot save.");
            return;
        }

        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("screenshot_{}.png", timestamp);

        let save_path = match dirs::desktop_dir() {
            Some(mut path) => {
                path.push(&filename);
                path
            }
            None => {
                eprintln!("[SNAPSHOT] Could not find Desktop dir, falling back to current dir.");
                std::path::PathBuf::from(&filename)
            }
        };

        eprintln!(
            "[SNAPSHOT] Saving {}x{} to {:?}...",
            width, height, save_path
        );

        let rgba_vec = rgba.as_ref().clone();
        match ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba_vec) {
            Some(buffer) => match buffer.save(&save_path) {
                Ok(_) => eprintln!("[SNAPSHOT] Saved to {:?}", save_path),
                Err(e) => eprintln!("[SNAPSHOT] Failed to save: {}", e),
            },
            None => eprintln!("[SNAPSHOT] Failed to create image buffer."),
        }
    });
}
