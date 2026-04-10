use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct ScreenCapture;

impl ScreenCapture {
    pub fn new() -> Self { Self }

    pub fn capture_jpeg_base64(&self, _quality: u8) -> Result<String> {
        let tmp = std::env::temp_dir().join("david_screen.jpg");
        std::process::Command::new("screencapture")
            .args(["-x", "-t", "jpg", tmp.to_str().unwrap()])
            .output()?;
        let bytes = std::fs::read(&tmp)?;
        let _ = std::fs::remove_file(&tmp);
        Ok(BASE64.encode(&bytes))
    }
}