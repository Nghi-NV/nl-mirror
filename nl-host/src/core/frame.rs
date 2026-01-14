use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Represents a single decoded video frame with YUV I420 data
/// YUV planes are uploaded directly to GPU for shader-based RGB conversion
#[derive(Clone)]
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub y_plane: Arc<Vec<u8>>,
    pub u_plane: Arc<Vec<u8>>,
    pub v_plane: Arc<Vec<u8>>,
    pub y_stride: usize,
    pub uv_stride: usize,
}

/// Frame buffer with mutex synchronization
///
/// Holds at most 1 pending frame. If a new frame arrives before
/// the previous one is consumed, the old frame is dropped (to minimize latency).
pub struct FrameBuffer {
    pending_frame: Mutex<Option<FrameData>>,
    frame_count: AtomicU64,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self {
            pending_frame: Mutex::new(None),
            frame_count: AtomicU64::new(0),
        }
    }

    /// Push a new frame, replacing any pending frame
    /// Returns true if previous frame was skipped, None if lock failed
    pub fn push(&self, frame: FrameData) -> bool {
        if let Ok(mut pending) = self.pending_frame.try_lock() {
            let skipped = pending.is_some();
            *pending = Some(frame);
            self.frame_count.fetch_add(1, Ordering::Relaxed);
            skipped
        } else {
            // Lock contention - drop this frame to avoid blocking
            true
        }
    }

    /// Consume the pending frame, if any
    pub fn consume(&self) -> Option<FrameData> {
        self.pending_frame
            .try_lock()
            .ok()
            .and_then(|mut p| p.take())
    }

    /// Get total frame count received
    pub fn get_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}
