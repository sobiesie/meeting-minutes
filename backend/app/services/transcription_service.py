import os
import numpy as np
from scipy.io import wavfile
from typing import Optional

class TranscriptionService:
    def __init__(self):
        self.testing = os.getenv("TESTING") == "true"

    def transcribe_file(self, file_path: str) -> str:
        """Transcribe an audio file."""
        if not os.path.exists(file_path):
            raise ValueError(f"Audio file {file_path} not found")

        if self.testing:
            return "This is a test transcription."
        
        try:
            # Read the audio file
            sample_rate, audio_data = wavfile.read(file_path)
            
            # Convert to float32 and normalize if needed
            audio_data = audio_data.astype(np.float32)
            if audio_data.max() > 1.0:
                audio_data = audio_data / 32768.0  # For 16-bit audio
            
            # In a real implementation, we would use a speech-to-text service here
            # For now, return a placeholder
            return "This is a placeholder transcription."
            
        except Exception as e:
            print(f"Error transcribing file: {str(e)}")  # Add logging
            raise ValueError(f"Failed to transcribe audio file: {str(e)}")

    def transcribe_audio(self, audio_data: np.ndarray) -> str:
        """Transcribe audio data directly."""
        if self.testing:
            return "This is a test transcription."
        
        try:
            # Convert to float32 and normalize if needed
            audio_data = audio_data.astype(np.float32)
            if audio_data.max() > 1.0:
                audio_data = audio_data / 32768.0  # For 16-bit audio
            
            # In a real implementation, we would use a speech-to-text service here
            # For now, return a placeholder
            return "This is a placeholder transcription."
            
        except Exception as e:
            print(f"Error transcribing audio: {str(e)}")  # Add logging
            raise ValueError(f"Failed to transcribe audio data: {str(e)}")

# Create a mock transcription service for testing
class MockTranscriptionService:
    def transcribe_file(self, file_path: str) -> str:
        """Mock transcription for testing."""
        try:
            # Still try to read the file to validate it exists
            sample_rate, audio_data = wavfile.read(file_path)
            if len(audio_data) == 0:
                raise ValueError("Empty audio file")
            return "This is a test transcription."
        except Exception as e:
            print(f"Error in mock transcription: {str(e)}")  # Add logging
            raise ValueError(f"Failed to read audio file: {str(e)}")

    def transcribe_audio(self, audio_data: np.ndarray) -> str:
        """Mock transcription for testing."""
        if len(audio_data) == 0:
            raise ValueError("Empty audio data")
        return "This is a test transcription."

# Global instance of the transcription service
# Use mock service if in test environment
if os.getenv("TESTING") == "true":
    transcription_service = MockTranscriptionService()
else:
    transcription_service = TranscriptionService()
