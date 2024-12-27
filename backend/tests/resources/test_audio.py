import numpy as np
import wave
import os

def create_test_audio(duration=1.0, sample_rate=44100):
    """Create a test audio file with a simple sine wave."""
    t = np.linspace(0, duration, int(sample_rate * duration))
    frequency = 440  # A4 note
    amplitude = 0.5
    samples = amplitude * np.sin(2 * np.pi * frequency * t)
    
    # Convert to 16-bit PCM
    samples = (samples * 32767).astype(np.int16)
    
    # Create directory if it doesn't exist
    os.makedirs("tests/resources", exist_ok=True)
    
    # Write to WAV file
    with wave.open("tests/resources/test_audio.wav", "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)  # 16-bit
        wf.setframerate(sample_rate)
        wf.writeframes(samples.tobytes())
    
    return "tests/resources/test_audio.wav"

if __name__ == "__main__":
    create_test_audio()
