use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct ScreenCapture;

impl ScreenCapture {
    pub fn new() -> Self { Self }

    pub fn capture_jpeg_base64(&self, _quality: u8) -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            let tmp = std::env::temp_dir().join("david_screen.jpg");
            let output = std::process::Command::new("screencapture")
                .args(["-x", "-t", "jpg", tmp.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                anyhow::bail!("screencapture failed");
            }

            let bytes = std::fs::read(&tmp)?;
            let _ = std::fs::remove_file(&tmp);
            return Ok(BASE64.encode(&bytes));
        }

        #[cfg(target_os = "windows")]
        {
            anyhow::bail!("Screen capture not implemented for Windows dev mode");
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("Unsupported platform");
        }
    }
}