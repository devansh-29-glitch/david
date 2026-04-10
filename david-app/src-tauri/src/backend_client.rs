/// Backend Client
/// ===============
/// HTTP client for Railway backend communication.
/// All AI calls go through here.

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct BackendClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct ScreenshotResponse {
    pub should_speak: bool,
    pub message: String,
    pub mode: String,
    pub audio_b64: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: String,
    pub audio_b64: Option<String>,
    pub mode: String,
}

#[derive(Debug, Deserialize)]
pub struct RewriteResponse {
    pub rewritten: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: serde_json::Value,
}

impl BackendClient {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        Self { base_url: base_url.to_string(), client }
    }

    pub async fn register(&self, name: &str, email: &str, password: &str) -> Result<AuthResponse> {
        let body = serde_json::json!({"name": name, "email": email, "password": password});
        let resp = self.client
            .post(format!("{}/auth/register", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await?;
            anyhow::bail!("{}", err["detail"].as_str().unwrap_or("Registration failed"));
        }
        Ok(resp.json().await?)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthResponse> {
        let body = serde_json::json!({"email": email, "password": password});
        let resp = self.client
            .post(format!("{}/auth/login", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await?;
            anyhow::bail!("{}", err["detail"].as_str().unwrap_or("Login failed"));
        }
        Ok(resp.json().await?)
    }

    pub async fn get_me(&self, token: &str) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/auth/me", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn send_screenshot(
        &self,
        token: &str,
        screenshot_b64: &str,
        activity_level: &str,
        audio_playing: bool,
        dwell_seconds: f64,
        write_delete_count: u32,
        current_app: &str,
        session_id: &str,
        time_since_last_unprompted: f64,
    ) -> Result<ScreenshotResponse> {
        let body = serde_json::json!({
            "screenshot_b64": screenshot_b64,
            "activity_level": activity_level,
            "audio_playing": audio_playing,
            "dwell_seconds": dwell_seconds,
            "write_delete_count": write_delete_count,
            "current_app": current_app,
            "session_id": session_id,
            "time_since_last_unprompted": time_since_last_unprompted,
        });

        let resp = self.client
            .post(format!("{}/david/screenshot", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await?;

        Ok(resp.json().await?)
    }

    pub async fn send_chat(
        &self,
        token: &str,
        message: &str,
        screenshot_b64: Option<&str>,
        session_id: &str,
        audio_playing: bool,
    ) -> Result<ChatResponse> {
        let body = serde_json::json!({
            "message": message,
            "screenshot_b64": screenshot_b64,
            "session_id": session_id,
            "audio_playing": audio_playing,
        });

        let resp = self.client
            .post(format!("{}/david/chat", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if resp.status() == 429 {
            anyhow::bail!("Daily limit reached. Upgrade your plan for more.");
        }

        Ok(resp.json().await?)
    }

    pub async fn rewrite(
        &self,
        token: &str,
        text: &str,
        instruction: &str,
        screenshot_b64: Option<&str>,
    ) -> Result<RewriteResponse> {
        let body = serde_json::json!({
            "text": text,
            "instruction": instruction,
            "screenshot_b64": screenshot_b64,
        });

        let resp = self.client
            .post(format!("{}/david/rewrite", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if resp.status() == 429 {
            anyhow::bail!("Daily rewrite limit reached. Upgrade your plan.");
        }

        Ok(resp.json().await?)
    }
}
