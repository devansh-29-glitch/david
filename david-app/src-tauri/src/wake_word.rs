/// Wake Word Detection
/// ====================
/// Detects "David" wake commands from voice input.
/// Runs on every audio transcript from the Gemini audio pipeline.

#[derive(Debug, Clone)]
pub enum WakeCommand {
    Activate,
    ActivateWithMessage(String),
}

pub fn detect_wake_command(transcript: &str) -> Option<WakeCommand> {
    let lower = transcript.trim().to_lowercase();
    let clean = lower.trim_matches(|c: char| !c.is_alphabetic() && c != ' ');

    if clean.starts_with("david,") || clean.starts_with("david ") {
        let offset = if clean.starts_with("david,") { 6 } else { 5 };
        let after = transcript[offset..].trim().to_string();
        if !after.is_empty() && after.split_whitespace().count() > 1 {
            return Some(WakeCommand::ActivateWithMessage(after));
        }
    }

    if clean == "david" || clean == "hey david" || clean == "ok david" {
        return Some(WakeCommand::Activate);
    }

    None
}
