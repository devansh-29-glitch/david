pub mod david {
    use tauri::State;
    use std::sync::Arc;
    use crate::DavidState;
    use crate::backend_client::BackendClient;
    use crate::BACKEND_URL;

    #[tauri::command]
    pub async fn register(
        name: String,
        email: String,
        password: String,
        state: State<'_, Arc<DavidState>>,
    ) -> Result<serde_json::Value, String> {
        let client = BackendClient::new(BACKEND_URL);
        match client.register(&name, &email, &password).await {
            Ok(resp) => {
                *state.auth_token.lock().unwrap() = Some(resp.token.clone());
                *state.user_name.lock().unwrap() = name;
                if let Some(tier) = resp.user["tier"].as_str() {
                    *state.user_tier.lock().unwrap() = tier.to_string();
                }
                Ok(serde_json::json!({
                    "token": resp.token,
                    "user": resp.user,
                }))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tauri::command]
    pub async fn login(
        email: String,
        password: String,
        state: State<'_, Arc<DavidState>>,
    ) -> Result<serde_json::Value, String> {
        let client = BackendClient::new(BACKEND_URL);
        match client.login(&email, &password).await {
            Ok(resp) => {
                *state.auth_token.lock().unwrap() = Some(resp.token.clone());
                if let Some(name) = resp.user["name"].as_str() {
                    *state.user_name.lock().unwrap() = name.to_string();
                }
                if let Some(tier) = resp.user["tier"].as_str() {
                    *state.user_tier.lock().unwrap() = tier.to_string();
                }
                Ok(serde_json::json!({
                    "token": resp.token,
                    "user": resp.user,
                }))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tauri::command]
    pub async fn get_me(
        state: State<'_, Arc<DavidState>>,
    ) -> Result<serde_json::Value, String> {
        let token = state.get_token().ok_or("Not logged in")?;
        let client = BackendClient::new(BACKEND_URL);
        client.get_me(&token).await.map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn send_chat(
        message: String,
        state: State<'_, Arc<DavidState>>,
    ) -> Result<serde_json::Value, String> {
        let token = state.get_token().ok_or("Not logged in")?;
        let screenshot = state.last_screenshot_b64.lock().unwrap().clone();
        let audio = *state.audio_playing.lock().unwrap();
        let session_id = state.session_id.clone();

        let client = BackendClient::new(BACKEND_URL);
        match client.send_chat(
            &token,
            &message,
            screenshot.as_deref(),
            &session_id,
            audio,
        ).await {
            Ok(resp) => Ok(serde_json::json!({
                "message": resp.message,
                "audio_b64": resp.audio_b64,
                "mode": resp.mode,
            })),
            Err(e) => Err(e.to_string()),
        }
    }

    #[tauri::command]
    pub async fn request_rewrite(
        text: String,
        instruction: String,
        state: State<'_, Arc<DavidState>>,
    ) -> Result<String, String> {
        let token = state.get_token().ok_or("Not logged in")?;
        let screenshot = state.last_screenshot_b64.lock().unwrap().clone();

        let client = BackendClient::new(BACKEND_URL);
        client.rewrite(&token, &text, &instruction, screenshot.as_deref())
            .await
            .map(|r| r.rewritten)
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn reset_session(
        state: State<'_, Arc<DavidState>>,
    ) -> Result<(), String> {
        let token = state.get_token().ok_or("Not logged in")?;
        let session_id = state.session_id.clone();
        let client = reqwest::Client::new();
        client.post(format!("{}/david/reset?session_id={}", BACKEND_URL, session_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[tauri::command]
    pub async fn get_latest_screenshot(
        state: State<'_, Arc<DavidState>>,
    ) -> Result<Option<String>, String> {
        Ok(state.last_screenshot_b64.lock().unwrap().clone())
    }
}
