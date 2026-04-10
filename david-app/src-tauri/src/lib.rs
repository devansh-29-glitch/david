/// David v1 — Tauri Application Core
/// =====================================
/// Wires together:
/// - Screen capture (macOS ScreenCaptureKit)
/// - System audio detection (Core Audio)
/// - Activity tracking (keyboard + mouse via rdev)
/// - Smart screenshot frequency (1.6 / 3.4 / 7.9 seconds)
/// - Fish Speech TTS server management (local subprocess)
/// - Wake word detection ("David")
/// - All communication with Railway backend

mod screen_capture;
mod audio_detector;
mod activity_tracker;
mod wake_word;
mod fish_speech;
mod backend_client;
pub mod commands;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Manager, Emitter};
use tokio::time::sleep;
use log::{info, error, warn};
use uuid::Uuid;

use screen_capture::ScreenCapture;
use audio_detector::AudioDetector;
use activity_tracker::{ActivityTracker, ActivityLevel};
use fish_speech::FishSpeechManager;
use backend_client::BackendClient;

pub const BACKEND_URL: &str = "https://david-api-production.up.railway.app";

/// Shared state passed around the app
pub struct DavidState {
    pub auth_token: Mutex<Option<String>>,
    pub user_name: Mutex<String>,
    pub user_tier: Mutex<String>,
    pub session_id: String,
    pub activity_level: Mutex<ActivityLevel>,
    pub audio_playing: Mutex<bool>,
    pub write_delete_count: Mutex<u32>,
    pub dwell_seconds: Mutex<f64>,
    pub current_app: Mutex<String>,
    pub last_unprompted_time: Mutex<Instant>,
    pub last_screenshot_b64: Mutex<Option<String>>,
}

impl DavidState {
    pub fn new() -> Self {
        Self {
            auth_token: Mutex::new(None),
            user_name: Mutex::new(String::new()),
            user_tier: Mutex::new("free".to_string()),
            session_id: Uuid::new_v4().to_string(),
            activity_level: Mutex::new(ActivityLevel::Slow),
            audio_playing: Mutex::new(false),
            write_delete_count: Mutex::new(0),
            dwell_seconds: Mutex::new(0.0),
            current_app: Mutex::new(String::new()),
            last_unprompted_time: Mutex::new(Instant::now() - Duration::from_secs(120)),
            last_screenshot_b64: Mutex::new(None),
        }
    }

    pub fn get_screenshot_interval(&self) -> Duration {
        let audio = *self.audio_playing.lock().unwrap();
        if audio {
            return Duration::from_millis(7900);
        }
        match *self.activity_level.lock().unwrap() {
            ActivityLevel::Rigorous => Duration::from_millis(1600),
            ActivityLevel::Slow => Duration::from_millis(3400),
            ActivityLevel::Idle => Duration::from_millis(7900),
        }
    }

    pub fn get_token(&self) -> Option<String> {
        self.auth_token.lock().unwrap().clone()
    }

    pub fn time_since_last_unprompted(&self) -> f64 {
        self.last_unprompted_time.lock().unwrap().elapsed().as_secs_f64()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--start-hidden"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let state = Arc::new(DavidState::new());
            app.manage(state.clone());

            let app_handle = app.handle().clone();

            // ── Start Fish Speech TTS server ───────────────────────────────────
            let resource_dir = app.path().resource_dir()
                .expect("Could not find resource dir");

            let fish_manager = FishSpeechManager::new(resource_dir.clone());
            let fish_clone = fish_manager.clone();
            std::thread::spawn(move || {
                match fish_clone.start() {
                    Ok(_) => info!("Fish Speech TTS server ready on port 8765"),
                    Err(e) => warn!("Fish Speech failed to start: {}. Voice output will use Gemini TTS.", e),
                }
            });
            app.manage(Arc::new(fish_manager));

            // ── Start audio detector ───────────────────────────────────────────
            let state_audio = state.clone();
            let app_handle_audio = app_handle.clone();
            tokio::spawn(async move {
                let detector = AudioDetector::new();
                loop {
                    let playing = detector.is_audio_playing();
                    let changed = {
                        let mut current = state_audio.audio_playing.lock().unwrap();
                        if *current != playing {
                            *current = playing;
                            true
                        } else {
                            false
                        }
                    };
                    if changed {
                        let _ = app_handle_audio.emit("audio-state-changed", playing);
                    }
                    sleep(Duration::from_millis(800)).await;
                }
            });

            // ── Start activity tracker ─────────────────────────────────────────
            let state_activity = state.clone();
            let app_handle_activity = app_handle.clone();
            tokio::spawn(async move {
                let tracker = ActivityTracker::new();
                tracker.start(state_activity, app_handle_activity);
            });

            // ── Smart screenshot + confusion detection loop ────────────────────
            let state_screen = state.clone();
            let app_handle_screen = app_handle.clone();
            tokio::spawn(async move {
                let capture = ScreenCapture::new();
                let client = BackendClient::new(BACKEND_URL);

                // Wait 2 seconds before starting
                sleep(Duration::from_secs(2)).await;

                loop {
                    let interval = state_screen.get_screenshot_interval();
                    sleep(interval).await;

                    let token = state_screen.get_token();
                    if token.is_none() {
                        continue; // Not logged in yet
                    }

                    match capture.capture_jpeg_base64(55) {
                        Ok(screenshot_b64) => {
                            // Store latest screenshot for chat context
                            *state_screen.last_screenshot_b64.lock().unwrap() = Some(screenshot_b64.clone());

                            let activity = format!("{:?}", *state_screen.activity_level.lock().unwrap()).to_lowercase();
                            let audio = *state_screen.audio_playing.lock().unwrap();
                            let dwell = *state_screen.dwell_seconds.lock().unwrap();
                            let wd = *state_screen.write_delete_count.lock().unwrap();
                            let app_name = state_screen.current_app.lock().unwrap().clone();
                            let time_since = state_screen.time_since_last_unprompted();
                            let session_id = state_screen.session_id.clone();

                            match client.send_screenshot(
                                token.as_deref().unwrap(),
                                &screenshot_b64,
                                &activity,
                                audio,
                                dwell,
                                wd,
                                &app_name,
                                &session_id,
                                time_since,
                            ).await {
                                Ok(response) if response.should_speak => {
                                    // Update last unprompted time
                                    *state_screen.last_unprompted_time.lock().unwrap() = Instant::now();

                                    let _ = app_handle_screen.emit("david-speaks", serde_json::json!({
                                        "message": response.message,
                                        "mode": response.mode,
                                        "audio_b64": response.audio_b64,
                                    }));
                                }
                                Err(e) => error!("Screenshot send error: {}", e),
                                _ => {}
                            }
                        }
                        Err(e) => error!("Screen capture error: {}", e),
                    }
                }
            });

            // ── Global hotkey: Cmd+Shift+D ─────────────────────────────────────
            let app_handle_hotkey = app_handle.clone();
            app.global_shortcut()
                .on_shortcut("CmdOrCtrl+Shift+D", move |_app, _shortcut, _event| {
                    let _ = app_handle_hotkey.emit("toggle-orb", ());
                })
                .expect("Failed to register hotkey Cmd+Shift+D");

            info!("David v1 started. Session: {}", state.session_id);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::david::login,
            commands::david::register,
            commands::david::get_me,
            commands::david::send_chat,
            commands::david::request_rewrite,
            commands::david::reset_session,
            commands::david::get_latest_screenshot,
        ])
        .run(tauri::generate_context!())
        .expect("Error running David");
}
