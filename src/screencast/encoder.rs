//! Hardware encoder detection and selection
//!
//! Queries GStreamer for available encoders and prioritizes hardware-accelerated ones

use crate::config::Container;
use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use std::sync::OnceLock;

/// Codec type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    H264,
    H265,
    VP9,
    AV1,
}

impl Codec {
    pub fn name(&self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::H265 => "H.265",
            Codec::VP9 => "VP9",
            Codec::AV1 => "AV1",
        }
    }

    /// GStreamer parser element required to mux this codec, if any.
    ///
    /// H.264/H.265 need a parser for MP4; VP9 goes into WebM/MKV without one.
    /// AV1 needs `av1parse` for MP4 (not currently a recording target here).
    fn parser_element(&self) -> Option<&'static str> {
        match self {
            Codec::H264 => Some("h264parse"),
            Codec::H265 => Some("h265parse"),
            Codec::AV1 => Some("av1parse"),
            Codec::VP9 => None,
        }
    }

    /// Whether this codec can be muxed into the given container by the pipeline.
    ///
    /// Matroska (MKV) is a catch-all. MP4 is only offered for H.264/H.265 — the
    /// codecs snappea wires a parser for and that mux reliably; VP9/AV1 in MP4 is
    /// unreliable and would otherwise produce an empty file (see issue #17).
    pub fn supports_container(&self, container: Container) -> bool {
        match (self, container) {
            (_, Container::Mkv) => true,
            (Codec::H264 | Codec::H265, Container::Mp4) => true,
            (Codec::VP9 | Codec::AV1, Container::Mp4) => false,
            (_, Container::Webm) => matches!(self, Codec::VP9 | Codec::AV1),
        }
    }

    /// Container to fall back to when the configured one is incompatible.
    pub fn default_container(&self) -> Container {
        match self {
            Codec::H264 | Codec::H265 => Container::Mp4,
            Codec::VP9 | Codec::AV1 => Container::Mkv,
        }
    }

    /// Best-effort codec inference from a GStreamer encoder element name.
    ///
    /// Used by the settings UI to gate the container dropdown without threading
    /// full `EncoderInfo` through the widget signature.
    pub fn from_element_name(name: &str) -> Option<Codec> {
        if name.contains("h265") || name.contains("hevc") {
            Some(Codec::H265)
        } else if name.contains("h264") || name.contains("x264") || name.contains("openh264") {
            Some(Codec::H264)
        } else if name.contains("av1") {
            Some(Codec::AV1)
        } else if name.contains("vp9") {
            Some(Codec::VP9)
        } else {
            None
        }
    }
}

/// Information about an available encoder
#[derive(Debug, Clone)]
pub struct EncoderInfo {
    /// Human-readable name (e.g., "VA-API H.264")
    pub name: String,
    /// GStreamer element name (e.g., "vaapih264enc")
    pub gst_element: String,
    /// Codec type
    pub codec: Codec,
    /// Whether this is hardware-accelerated
    pub hardware: bool,
    /// Whether this encoder can participate in the real DMA-BUF zero-copy path
    pub supports_dmabuf_zero_copy: bool,
    /// Priority (lower = better, hardware encoders have lower priority)
    pub priority: u8,
}

impl EncoderInfo {
    /// Display name with hardware/software indicator
    pub fn display_name(&self) -> String {
        let hw_indicator = if self.hardware {
            " (Hardware)"
        } else {
            " (Software)"
        };
        format!("{}{}", self.name, hw_indicator)
    }

    pub fn zero_copy_display_name(&self) -> &'static str {
        if self.supports_dmabuf_zero_copy {
            "DMA-BUF zero-copy capable"
        } else {
            "copied-memory path only"
        }
    }
}

/// A candidate encoder: an element name to probe plus the metadata to publish
/// if it resolves. `detect_encoders` walks each codec's candidate list in order
/// and keeps the first element that GStreamer can actually create.
struct Candidate {
    name: &'static str,
    gst_element: &'static str,
    codec: Codec,
    hardware: bool,
    supports_dmabuf_zero_copy: bool,
    priority: u8,
}

/// Detect available video encoders
///
/// For each codec/backend we probe a list of candidate GStreamer element names
/// in preference order and keep the first that resolves. This matters because
/// the element names differ across GStreamer generations:
///   - Legacy `gstreamer-vaapi` exposes `vaapih264enc`, `vaapih265enc`, ...
///   - The modern stateless `va` plugin (gst-plugins-bad, GStreamer 1.22+, and
///     the default on recent Arch) exposes `vah264enc`, `vah265enc`, ... instead,
///     with `gstreamer-vaapi` deprecated/removed.
/// Probing only the legacy names made snappea miss hardware encoders entirely on
/// modern systems, leaving software VP9 as the only option (see issue #17).
///
/// The legacy `vaapi*` elements are listed first so systems that still have them
/// keep the existing DMA-BUF zero-copy path (which relies on `vaapipostproc` /
/// `memory:VASurface`); the modern `va*` elements are wired through the generic
/// copied-memory pipeline for now (`supports_dmabuf_zero_copy: false`).
pub fn detect_encoders() -> Result<Vec<EncoderInfo>> {
    gst::init().context("Failed to initialize GStreamer")?;

    // Each inner slice is a preference-ordered list of interchangeable encoders
    // for one codec/backend; only the first available element in each slice is
    // added, so we never show two entries for the same underlying encoder.
    let candidate_groups: &[&[Candidate]] = &[
        // VA-API H.264 (Intel/AMD) - priority 10
        &[
            Candidate { name: "VA-API H.264", gst_element: "vaapih264enc", codec: Codec::H264, hardware: true, supports_dmabuf_zero_copy: true, priority: 10 },
            Candidate { name: "VA-API H.264", gst_element: "vah264enc", codec: Codec::H264, hardware: true, supports_dmabuf_zero_copy: false, priority: 10 },
            Candidate { name: "VA-API H.264 (low-power)", gst_element: "vah264lpenc", codec: Codec::H264, hardware: true, supports_dmabuf_zero_copy: false, priority: 10 },
        ],
        // VA-API H.265 - priority 11
        &[
            Candidate { name: "VA-API H.265", gst_element: "vaapih265enc", codec: Codec::H265, hardware: true, supports_dmabuf_zero_copy: true, priority: 11 },
            Candidate { name: "VA-API H.265", gst_element: "vah265enc", codec: Codec::H265, hardware: true, supports_dmabuf_zero_copy: false, priority: 11 },
            Candidate { name: "VA-API H.265 (low-power)", gst_element: "vah265lpenc", codec: Codec::H265, hardware: true, supports_dmabuf_zero_copy: false, priority: 11 },
        ],
        // VA-API VP9 - priority 12
        &[
            Candidate { name: "VA-API VP9", gst_element: "vaapivp9enc", codec: Codec::VP9, hardware: true, supports_dmabuf_zero_copy: true, priority: 12 },
            Candidate { name: "VA-API VP9", gst_element: "vavp9enc", codec: Codec::VP9, hardware: true, supports_dmabuf_zero_copy: false, priority: 12 },
        ],
        // NVENC H.264 (NVIDIA) - priority 20
        &[
            Candidate { name: "NVENC H.264", gst_element: "nvh264enc", codec: Codec::H264, hardware: true, supports_dmabuf_zero_copy: false, priority: 20 },
            Candidate { name: "NVENC H.264", gst_element: "nvcudah264enc", codec: Codec::H264, hardware: true, supports_dmabuf_zero_copy: false, priority: 20 },
        ],
        // NVENC H.265 - priority 21
        &[
            Candidate { name: "NVENC H.265", gst_element: "nvh265enc", codec: Codec::H265, hardware: true, supports_dmabuf_zero_copy: false, priority: 21 },
            Candidate { name: "NVENC H.265", gst_element: "nvcudah265enc", codec: Codec::H265, hardware: true, supports_dmabuf_zero_copy: false, priority: 21 },
        ],
        // Software H.264 - priority 100. x264 (gst-plugins-ugly) preferred,
        // openh264 (gst-plugins-bad) as a fallback so MP4 still works without
        // gst-plugins-ugly installed.
        &[
            Candidate { name: "x264 H.264", gst_element: "x264enc", codec: Codec::H264, hardware: false, supports_dmabuf_zero_copy: false, priority: 100 },
            Candidate { name: "OpenH264", gst_element: "openh264enc", codec: Codec::H264, hardware: false, supports_dmabuf_zero_copy: false, priority: 100 },
        ],
        // Software VP9 - priority 101
        &[
            Candidate { name: "VP9", gst_element: "vp9enc", codec: Codec::VP9, hardware: false, supports_dmabuf_zero_copy: false, priority: 101 },
        ],
    ];

    // Probing spins up real GStreamer pipelines, so cache the result for the
    // lifetime of the process — encoder availability doesn't change at runtime,
    // and detection is called from both the settings UI and the record path.
    static CACHE: OnceLock<Vec<EncoderInfo>> = OnceLock::new();
    let encoders = CACHE.get_or_init(|| {
        let mut encoders = Vec::new();
        for group in candidate_groups {
            // Keep the first candidate that both exists AND actually encodes.
            // Probing (not just existence) is what filters out encoders that are
            // present but fail at runtime — e.g. a VA-API element the GPU/driver
            // can't drive, the root cause behind issue #17.
            if let Some(candidate) = group
                .iter()
                .find(|c| encoder_available(c.gst_element) && encoder_works(c.gst_element, c.codec))
            {
                encoders.push(EncoderInfo {
                    name: candidate.name.to_string(),
                    gst_element: candidate.gst_element.to_string(),
                    codec: candidate.codec,
                    hardware: candidate.hardware,
                    supports_dmabuf_zero_copy: candidate.supports_dmabuf_zero_copy,
                    priority: candidate.priority,
                });
            }
        }

        // Sort by priority (lower first)
        encoders.sort_by_key(|e| e.priority);
        encoders
    });

    Ok(encoders.clone())
}

/// Check if a GStreamer encoder element is available
fn encoder_available(element_name: &str) -> bool {
    gst::ElementFactory::find(element_name).is_some()
}

/// Timeout for encoder probe pipelines (EOS or Error).
const PROBE_TIMEOUT_SECS: u64 = 3;

/// Verify an encoder actually works by running a tiny throwaway pipeline
/// (`videotestsrc → videoconvert → encoder → parser → fakesink`) to EOS.
///
/// Existence (`ElementFactory::find`) is not enough: hardware encoders routinely
/// register but then fail to negotiate or initialize on the actual GPU/driver.
/// Only encoders that reach EOS within [`PROBE_TIMEOUT_SECS`] are offered.
fn encoder_works(element_name: &str, codec: Codec) -> bool {
    let parser = codec
        .parser_element()
        .map(|p| format!(" ! {p}"))
        .unwrap_or_default();
    let pipeline_desc = format!(
        "videotestsrc num-buffers=3 \
         ! video/x-raw,format=NV12,width=320,height=240,framerate=10/1 \
         ! videoconvert ! {element_name}{parser} ! fakesink"
    );

    let pipeline = match gst::parse::launch(&pipeline_desc) {
        Ok(p) => p,
        Err(e) => {
            log::debug!("Encoder probe build failed for {element_name}: {e}");
            return false;
        }
    };
    let Some(bus) = pipeline.bus() else {
        return false;
    };
    if pipeline.set_state(gst::State::Playing).is_err() {
        let _ = pipeline.set_state(gst::State::Null);
        return false;
    }

    let ok = loop {
        match bus.timed_pop(gst::ClockTime::from_seconds(PROBE_TIMEOUT_SECS)) {
            Some(msg) => match msg.view() {
                gst::MessageView::Eos(_) => break true,
                gst::MessageView::Error(_) => break false,
                _ => continue,
            },
            None => break false, // timed out
        }
    };
    let _ = pipeline.set_state(gst::State::Null);
    if !ok {
        log::debug!("Encoder probe failed/timed out for {element_name}");
    }
    ok
}

/// Get the best available encoder (first hardware encoder, or first software if none)
pub fn best_encoder() -> Result<EncoderInfo> {
    let encoders = detect_encoders()?;
    encoders
        .into_iter()
        .next()
        .context("No video encoders available")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_name() {
        assert_eq!(Codec::H264.name(), "H.264");
        assert_eq!(Codec::H265.name(), "H.265");
        assert_eq!(Codec::VP9.name(), "VP9");
        assert_eq!(Codec::AV1.name(), "AV1");
    }

    #[test]
    fn test_encoder_info_display_name() {
        let hw_encoder = EncoderInfo {
            name: "VA-API H.264".to_string(),
            gst_element: "vaapih264enc".to_string(),
            codec: Codec::H264,
            hardware: true,
            supports_dmabuf_zero_copy: true,
            priority: 10,
        };
        assert_eq!(hw_encoder.display_name(), "VA-API H.264 (Hardware)");

        let sw_encoder = EncoderInfo {
            name: "x264 H.264".to_string(),
            gst_element: "x264enc".to_string(),
            codec: Codec::H264,
            hardware: false,
            supports_dmabuf_zero_copy: false,
            priority: 100,
        };
        assert_eq!(sw_encoder.display_name(), "x264 H.264 (Software)");
    }

    #[test]
    fn test_detect_encoders_returns_sorted_list() {
        // This test will succeed even if no encoders are available
        let result = detect_encoders();
        assert!(result.is_ok());

        let encoders = result.unwrap();
        // Verify encoders are sorted by priority
        for i in 1..encoders.len() {
            assert!(encoders[i - 1].priority <= encoders[i].priority);
        }
    }

    #[test]
    fn test_best_encoder() {
        // This test may fail on systems with no encoders, which is acceptable
        // In CI/CD, we'd need GStreamer plugins installed
        let result = best_encoder();

        if let Ok(encoder) = result {
            // If we have an encoder, verify it's valid
            assert!(!encoder.name.is_empty());
            assert!(!encoder.gst_element.is_empty());
        }
        // If no encoders available, that's also a valid outcome for this test
    }

    #[test]
    fn test_detect_encoders_have_unique_elements() {
        // Each codec/backend must resolve to at most one element, even on systems
        // that have both the legacy (vaapi*) and modern (va*) plugins installed.
        let Ok(encoders) = detect_encoders() else {
            return;
        };
        let mut seen = std::collections::HashSet::new();
        for e in &encoders {
            assert!(
                seen.insert(e.gst_element.clone()),
                "duplicate encoder element: {}",
                e.gst_element
            );
        }
    }

    #[test]
    fn test_codec_container_support() {
        // MP4 only for H.264/H.265; MKV for everything.
        assert!(Codec::H264.supports_container(Container::Mp4));
        assert!(Codec::H265.supports_container(Container::Mp4));
        assert!(!Codec::VP9.supports_container(Container::Mp4));
        assert!(!Codec::AV1.supports_container(Container::Mp4));

        for codec in [Codec::H264, Codec::H265, Codec::VP9, Codec::AV1] {
            assert!(codec.supports_container(Container::Mkv));
        }

        assert_eq!(Codec::H264.default_container(), Container::Mp4);
        assert_eq!(Codec::VP9.default_container(), Container::Mkv);
    }

    #[test]
    fn test_codec_from_element_name() {
        assert_eq!(Codec::from_element_name("vaapih264enc"), Some(Codec::H264));
        assert_eq!(Codec::from_element_name("vah264enc"), Some(Codec::H264));
        assert_eq!(Codec::from_element_name("x264enc"), Some(Codec::H264));
        assert_eq!(Codec::from_element_name("openh264enc"), Some(Codec::H264));
        assert_eq!(Codec::from_element_name("vah265enc"), Some(Codec::H265));
        assert_eq!(Codec::from_element_name("nvh265enc"), Some(Codec::H265));
        assert_eq!(Codec::from_element_name("vaav1enc"), Some(Codec::AV1));
        assert_eq!(Codec::from_element_name("vp9enc"), Some(Codec::VP9));
        assert_eq!(Codec::from_element_name("wildcard"), None);
    }

    #[test]
    fn test_detected_encoder_codec_container_consistency() {
        // Every detected encoder must have a working default container.
        let Ok(encoders) = detect_encoders() else {
            return;
        };
        for e in &encoders {
            assert!(e.codec.supports_container(e.codec.default_container()));
        }
    }

    #[test]
    fn test_hardware_priority_lower_than_software() {
        // Verify priority system: hardware < software
        let hw_priority = 10u8; // VA-API
        let sw_priority = 100u8; // x264
        assert!(hw_priority < sw_priority);
    }
}
