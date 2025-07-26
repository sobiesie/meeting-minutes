// New Groq integration module for speech-to-text via Whisper Large v3 Turbo
use anyhow::{anyhow, Result};
use hound;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use std::io::Cursor;
use screenpipe_core::Language;

/// Transcribe the provided audio buffer with Groq Whisper Large v3 Turbo.
///
/// * `api_key` – Groq API key.
/// * `audio`   – Mono PCM samples in the range [-1.0, 1.0].
/// * `sample_rate` – Sample rate of `audio` (Hz). Whisper expects 16 kHz but the function will accept whatever the caller passes (the caller is expected to resample to 16 kHz beforehand).
pub async fn transcribe_with_groq(
    api_key: &str,
    audio: &[f32],
    sample_rate: u32,
    languages: Vec<Language>,
) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow!("Missing GROQ_API_KEY"));
    }

    // Encode the float samples to a 16-bit little-endian WAV file in-memory.
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::new(Cursor::new(Vec::<u8>::new()), spec)?;
    for &sample in audio {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_sample = (clamped * 32767.0) as i16;
        writer.write_sample(int_sample)?;
    }
    // Extract the inner buffer.
    let cursor = writer.into_inner()?; // finalize is called internally by into_inner
    let wav_bytes = cursor.into_inner();

    // Prepare multipart form
    let file_part = Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    // Whisper supports optional language hint – use first language code if supplied.
    let language_code = languages
        .first()
        .map(|l| format!("{:?}", l).to_lowercase())
        .unwrap_or_else(|| "en".to_string());

    let form = Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "text")
        .text("temperature", "0")
        .text("language", language_code);

    let client = Client::new();
    let resp = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Groq API error {status}: {err_body}"));
    }

    let transcript = resp.text().await?.trim().to_string();
    Ok(transcript)
}