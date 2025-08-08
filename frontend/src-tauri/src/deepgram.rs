use anyhow::{anyhow, Result};
use screenpipe_core::Language;

/// Placeholder Deepgram transcription implementation.
/// Always returns an error because Deepgram support is disabled in this build.
#[allow(unused_variables)]
pub async fn transcribe_with_deepgram(
    api_key: &str,
    audio: &[f32],
    _device: &str,
    _sample_rate: u32,
    _languages: Vec<Language>,
) -> Result<String> {
    Err(anyhow!("Deepgram transcription is not enabled in this build"))
}