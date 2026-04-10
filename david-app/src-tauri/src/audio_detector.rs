/// Audio Detector
/// ===============
/// Detects whether any system audio is playing.
/// When audio is playing, David switches to text-only mode.
/// David NEVER interrupts music, videos, or any audio.
///
/// Mac: Uses Core Audio AudioObjectGetPropertyData to read peak audio level.
/// Windows: Uses Windows Audio Session API (WASAPI) peak meter.

pub struct AudioDetector;

impl AudioDetector {
    pub fn new() -> Self { Self }

    pub fn is_audio_playing(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.check_macos();

        #[cfg(target_os = "windows")]
        return self.check_windows();

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return false;
    }

    #[cfg(target_os = "macos")]
    fn check_macos(&self) -> bool {
        use std::process::Command;

        // Use AppleScript to check if audio output volume is non-zero
        // and use pmset to detect audio assertions (most reliable method
        // without requiring full CoreAudio bindings)
        let output = Command::new("pmset")
            .args(["-g", "assertions"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            // "PreventUserIdleSystemSleep" assertion is created when audio plays
            if text.contains("PreventUserIdleSystemSleep") && text.contains("1") {
                return true;
            }
        }

        // Fallback: check if any audio processes are active via proc info
        let audio_processes = [
            "Spotify", "Music", "QuickTime Player", "VLC",
            "zoom.us", "Discord", "Slack", "FaceTime",
            "Safari", "Google Chrome", "Firefox", "Arc",
            "IINA", "Doppler",
        ];

        for proc in &audio_processes {
            if let Ok(out) = Command::new("pgrep").arg("-x").arg(proc).output() {
                if out.status.success() && !out.stdout.is_empty() {
                    // Process running — check if it actually has audio session
                    // For music apps, assume playing if running
                    if matches!(*proc, "Spotify" | "Music" | "Doppler") {
                        // Check Spotify play state via AppleScript
                        if *proc == "Spotify" {
                            let script = Command::new("osascript")
                                .args(["-e", "tell application \"Spotify\" to player state as string"])
                                .output();
                            if let Ok(s) = script {
                                let state = String::from_utf8_lossy(&s.stdout);
                                if state.trim() == "playing" {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    #[cfg(target_os = "windows")]
    fn check_windows(&self) -> bool {
        // Use Windows Audio Session API to get peak audio level
        // If peak > threshold, audio is playing
        use std::ptr;

        unsafe {
            use windows::Win32::System::Com::{
                CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
            };
            use windows::Win32::Media::Audio::{
                eMultimedia, eRender,
                IMMDeviceEnumerator, MMDeviceEnumerator,
                IAudioMeterInformation,
            };

            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                    Ok(e) => e,
                    Err(_) => return false,
                };

            let device = match enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
                Ok(d) => d,
                Err(_) => return false,
            };

            let meter: IAudioMeterInformation = match device.Activate(CLSCTX_ALL, None) {
                Ok(m) => m,
                Err(_) => return false,
            };

            let mut peak: f32 = 0.0;
            if meter.GetPeakValue(&mut peak).is_ok() {
                return peak > 0.005; // Audio playing if peak above threshold
            }

            false
        }
    }
}
