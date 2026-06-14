//! ffmpeg subprocess wrappers for video processing.
//!
//! Detects ffmpeg at startup with graceful degradation.
//! All operations use temp directories for output files.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// ffmpeg runner with availability detection.
pub struct FfmpegRunner {
    pub available: bool,
    ffmpeg_path: String,
    temp_dir: PathBuf,
}

impl FfmpegRunner {
    /// Detect ffmpeg on PATH. Returns a runner with `available` set accordingly.
    pub fn detect() -> Self {
        let ffmpeg_path = "ffmpeg".to_string();
        let available = std::process::Command::new(&ffmpeg_path)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let temp_dir = std::env::temp_dir().join("hkask-media");

        if available {
            tracing::info!(target: "hkask.mcp.media.ffmpeg", "ffmpeg detected");
        } else {
            tracing::warn!(target: "hkask.mcp.media.ffmpeg", "ffmpeg not found — video tools will be unavailable");
        }

        Self {
            available,
            ffmpeg_path,
            temp_dir,
        }
    }

    /// Ensure the temp directory exists.
    fn ensure_temp_dir(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))
    }

    /// Generate a unique output path in the temp directory.
    fn output_path(&self, extension: &str) -> PathBuf {
        let name = uuid::Uuid::new_v4().to_string();
        self.temp_dir.join(format!("{}.{}", name, extension))
    }

    /// Trim a video to specified start/end times.
    /// Uses stream copy (-c copy) for fast, lossless trimming.
    pub async fn clip(&self, input: &str, start_sec: f32, end_sec: f32) -> Result<PathBuf, String> {
        if !self.available {
            return Err("ffmpeg not available".to_string());
        }
        self.ensure_temp_dir()?;

        let output = self.output_path("mp4");
        let duration = end_sec - start_sec;

        let status = Command::new(&self.ffmpeg_path)
            .arg("-ss")
            .arg(format!("{:.3}", start_sec))
            .arg("-to")
            .arg(format!("{:.3}", end_sec))
            .arg("-i")
            .arg(input)
            .arg("-c")
            .arg("copy")
            .arg("-avoid_negative_ts")
            .arg("make_zero")
            .arg(&output)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| format!("ffmpeg clip spawn failed: {}", e))?;

        if !status.success() {
            return Err(format!(
                "ffmpeg clip failed with exit code: {:?}",
                status.code()
            ));
        }

        tracing::info!(target: "hkask.mcp.media.ffmpeg", input = %input, duration = %duration, output = %output.display(), "Video clipped");
        Ok(output)
    }

    /// Convert a video segment to GIF.
    /// Uses the two-pass palettegen + paletteuse pipeline for quality.
    pub async fn to_gif(
        &self,
        input: &str,
        start_sec: f32,
        duration_sec: f32,
        width: u32,
        fps: u32,
    ) -> Result<PathBuf, String> {
        if !self.available {
            return Err("ffmpeg not available".to_string());
        }
        self.ensure_temp_dir()?;

        let output = self.output_path("gif");
        let palette = self.output_path("png");

        // Build filter complex for palette generation + GIF conversion
        let filter = format!(
            "fps={},scale={}:-1:flags=lanczos,split[v1][v2];[v1]palettegen[p];[v2][p]paletteuse",
            fps, width
        );

        let status = Command::new(&self.ffmpeg_path)
            .arg("-ss")
            .arg(format!("{:.3}", start_sec))
            .arg("-t")
            .arg(format!("{:.3}", duration_sec))
            .arg("-i")
            .arg(input)
            .arg("-filter_complex")
            .arg(&filter)
            .arg(&output)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| format!("ffmpeg GIF spawn failed: {}", e))?;

        // Clean up palette temp file
        let _ = std::fs::remove_file(&palette);

        if !status.success() {
            return Err(format!(
                "ffmpeg GIF conversion failed with exit code: {:?}",
                status.code()
            ));
        }

        tracing::info!(target: "hkask.mcp.media.ffmpeg", input = %input, duration = %duration_sec, width = %width, fps = %fps, output = %output.display(), "GIF created");
        Ok(output)
    }

    /// Add text caption overlay to a video.
    /// Uses the drawtext filter with configurable position and font size.
    pub async fn add_caption(
        &self,
        input: &str,
        text: &str,
        position: &str,
        font_size: u32,
    ) -> Result<PathBuf, String> {
        if !self.available {
            return Err("ffmpeg not available".to_string());
        }
        self.ensure_temp_dir()?;

        let output = self.output_path("mp4");

        // Map position to drawtext y-coordinate
        let y_pos = match position {
            "top" => "(h-text_h-10)",
            "center" => "(h-text_h)/2",
            _ => "10", // bottom
        };

        // Escape special characters in text for ffmpeg filter
        let escaped_text = text
            .replace('\\', "\\\\")
            .replace(':', "\\:")
            .replace('\'', "\\'");

        let drawtext = format!(
            "drawtext=text='{}':fontsize={}:fontcolor=white:box=1:boxcolor=black@0.5:boxborderw=5:x=(w-text_w)/2:y={}",
            escaped_text, font_size, y_pos
        );

        let status = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(input)
            .arg("-vf")
            .arg(&drawtext)
            .arg("-c:a")
            .arg("copy")
            .arg(&output)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| format!("ffmpeg caption spawn failed: {}", e))?;

        if !status.success() {
            return Err(format!(
                "ffmpeg caption failed with exit code: {:?}",
                status.code()
            ));
        }

        tracing::info!(target: "hkask.mcp.media.ffmpeg", input = %input, text = %text, output = %output.display(), "Caption added");
        Ok(output)
    }
}
