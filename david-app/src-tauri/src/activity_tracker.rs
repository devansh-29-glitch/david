/// Activity Tracker
/// =================
/// Monitors system-wide keyboard and mouse activity.
/// Determines: screenshot frequency, write-delete loops, dwell time.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rdev::{listen, Event, EventType, Key};
use tauri::{AppHandle, Emitter};
use log::info;
use crate::DavidState;

#[derive(Debug, Clone, PartialEq)]
pub enum ActivityLevel {
    Rigorous,  // Fast typing, rapid clicks → 1.6s
    Slow,      // Reading, slow writing → 3.4s
    Idle,      // No activity → 7.9s
}

pub struct ActivityTracker;

impl ActivityTracker {
    pub fn new() -> Self { Self }

    pub fn start(&self, state: Arc<DavidState>, app_handle: AppHandle) {
        let state_listener = state.clone();
        let recent_events_2s: Arc<Mutex<Vec<Instant>>> = Arc::new(Mutex::new(Vec::new()));
        let recent_events_10s: Arc<Mutex<Vec<Instant>>> = Arc::new(Mutex::new(Vec::new()));
        let recent_keys: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let dwell_start: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));

        let events_2s_l = recent_events_2s.clone();
        let events_10s_l = recent_events_10s.clone();
        let keys_l = recent_keys.clone();
        let dwell_l = dwell_start.clone();

        // ── Keystroke + mouse listener ─────────────────────────────────────
        std::thread::spawn(move || {
            listen(move |event: Event| {
                let now = Instant::now();

                match event.event_type {
                    EventType::KeyPress(key) => {
                        events_2s_l.lock().unwrap().push(now);
                        events_10s_l.lock().unwrap().push(now);

                        let key_name = format!("{:?}", key);
                        let mut keys = keys_l.lock().unwrap();
                        keys.push(key_name.clone());
                        if keys.len() > 40 {
                            keys.remove(0);
                        }

                        // Write-delete detection
                        let backspace_count = keys.iter()
                            .filter(|k| k.contains("Backspace"))
                            .count();
                        let other_count = keys.len().saturating_sub(backspace_count);

                        if backspace_count >= 4 && other_count >= 4 {
                            let current = *state_listener.write_delete_count.lock().unwrap();
                            *state_listener.write_delete_count.lock().unwrap() = current + 1;
                        }

                        // Reset on Enter/Escape
                        if matches!(key, Key::Return | Key::KpReturn | Key::Escape) {
                            *state_listener.write_delete_count.lock().unwrap() = 0;
                            keys.clear();
                            *dwell_l.lock().unwrap() = Instant::now();
                        }
                    }
                    EventType::ButtonPress(_) => {
                        events_2s_l.lock().unwrap().push(now);
                        events_10s_l.lock().unwrap().push(now);
                        // Reset dwell on click (user navigated)
                        *dwell_l.lock().unwrap() = Instant::now();
                        *state_listener.write_delete_count.lock().unwrap() = 0;
                    }
                    EventType::MouseMove { .. } => {
                        events_10s_l.lock().unwrap().push(now);
                    }
                    EventType::Wheel { .. } => {
                        events_10s_l.lock().unwrap().push(now);
                    }
                    _ => {}
                }
            }).expect("Failed to start activity listener");
        });

        // ── Activity level updater (runs every 500ms) ──────────────────────
        let state_updater = state.clone();
        let dwell_updater = dwell_start.clone();
        let events_2s_u = recent_events_2s.clone();
        let events_10s_u = recent_events_10s.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let now = Instant::now();
                let cutoff_2s = now - Duration::from_secs(2);
                let cutoff_10s = now - Duration::from_secs(10);

                // Clean old events
                let count_2s = {
                    let mut events = events_2s_u.lock().unwrap();
                    events.retain(|t| *t > cutoff_2s);
                    events.len()
                };

                let count_10s = {
                    let mut events = events_10s_u.lock().unwrap();
                    events.retain(|t| *t > cutoff_10s);
                    events.len()
                };

                // Determine activity level
                let level = if count_2s >= 5 {
                    ActivityLevel::Rigorous
                } else if count_10s >= 2 {
                    ActivityLevel::Slow
                } else {
                    ActivityLevel::Idle
                };

                *state_updater.activity_level.lock().unwrap() = level.clone();

                // Update dwell time
                let dwell = dwell_updater.lock().unwrap().elapsed().as_secs_f64();
                *state_updater.dwell_seconds.lock().unwrap() = dwell;

                let level_str = match level {
                    ActivityLevel::Rigorous => "rigorous",
                    ActivityLevel::Slow => "slow",
                    ActivityLevel::Idle => "idle",
                };
                let _ = app_handle.emit("activity-level", level_str);
            }
        });

        info!("Activity tracker started");
    }
}
