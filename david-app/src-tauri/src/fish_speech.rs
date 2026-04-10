/// Fish Speech Manager
/// ====================
/// Manages the Fish Speech TTS server as a local subprocess.
/// Fish Speech runs on port 8765.
/// Started automatically when David launches.
/// The Fish Speech executable + model are bundled in the DMG resources.
///
/// When Fish Speech is available, voice output uses it (better quality).
/// When it fails or is unavailable, voice output falls back to Gemini TTS
/// which is handled server-side.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result;
use log::{info, error, warn};

#[derive(Clone)]
pub struct FishSpeechManager {
    process: Arc<Mutex<Option<Child>>>,
    resource_dir: PathBuf,
    pub port: u16,
}

impl FishSpeechManager {
    pub fn new(resource_dir: PathBuf) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            resource_dir,
            port: 8765,
        }
    }

    /// Start Fish Speech server.
    /// The fish-speech directory is bundled in resources/.
    /// Returns Ok when server is ready, Err if it fails to start.
    pub fn start(&self) -> Result<()> {
        let fish_dir = self.resource_dir.join("resources").join("fish-speech");
        let fish_exe = fish_dir.join("fish-speech-server");

        if !fish_exe.exists() {
            // Try alternate location
            let fish_exe_alt = fish_dir.join("server.py");
            if fish_exe_alt.exists() {
                return self.start_python_server(&fish_dir, &fish_exe_alt);
            }
            anyhow::bail!("Fish Speech not found at {:?}", fish_exe);
        }

        let child = Command::new(&fish_exe)
            .args(["--port", &self.port.to_string()])
            .current_dir(&fish_dir)
            .spawn()?;

        info!("Fish Speech process started");
        self.wait_for_ready()?;

        *self.process.lock().unwrap() = Some(child);
        Ok(())
    }

    fn start_python_server(&self, fish_dir: &PathBuf, server_py: &PathBuf) -> Result<()> {
        // Find bundled Python or system Python
        let python = if cfg!(target_os = "macos") {
            self.resource_dir.join("resources").join("python").join("bin").join("python3")
        } else {
            PathBuf::from("python3")
        };

        let child = Command::new(&python)
            .arg(server_py)
            .args(["--port", &self.port.to_string()])
            .current_dir(fish_dir)
            .spawn()?;

        info!("Fish Speech Python server started");
        self.wait_for_ready()?;

        *self.process.lock().unwrap() = Some(child);
        Ok(())
    }

    fn wait_for_ready(&self) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(30) {
                anyhow::bail!("Fish Speech did not start within 30 seconds");
            }

            match reqwest::blocking::get(
                format!("http://localhost:{}/health", self.port)
            ) {
                Ok(resp) if resp.status().is_success() => {
                    info!("Fish Speech ready on port {}", self.port);
                    return Ok(());
                }
                _ => std::thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    /// Generate speech audio from text.
    /// Returns MP3 bytes or None if Fish Speech is unavailable.
    pub async fn synthesize(&self, text: &str) -> Option<Vec<u8>> {
        let url = format!("http://localhost:{}/v1/tts", self.port);
        let body = serde_json::json!({
            "text": text,
            "format": "mp3",
            "streaming": false,
        });

        match reqwest::Client::new()
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(15))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                resp.bytes().await.ok().map(|b| b.to_vec())
            }
            _ => None,
        }
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.process.lock().unwrap().take() {
            let _ = child.kill();
            info!("Fish Speech stopped");
        }
    }
}

impl Drop for FishSpeechManager {
    fn drop(&mut self) {
        self.stop();
    }
}
