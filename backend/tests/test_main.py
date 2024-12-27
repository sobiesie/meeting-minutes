import pytest
from fastapi.testclient import TestClient
import os
import json
import numpy as np
from datetime import datetime, timedelta
import sounddevice as sd
from pathlib import Path
import tempfile
import shutil

from app.main import app
from app.services.transcription_service import TranscriptionService
from app.services.summarization_service import SummarizationService
from app.services.audio_service import audio_service as global_audio_service
from app.database import get_db, Base, engine

# Create test client
client = TestClient(app)

# Test data directory
TEST_DIR = Path(__file__).parent / "resources"
TEST_AUDIO_FILE = TEST_DIR / "test_audio.wav"

@pytest.fixture(scope="session")
def test_db():
    # Create test database
    Base.metadata.create_all(bind=engine)
    yield
    # Clean up
    Base.metadata.drop_all(bind=engine)

@pytest.fixture
def test_audio_file():
    """Create a test audio file"""
    if not TEST_AUDIO_FILE.exists():
        # Create a simple sine wave
        duration = 2.0  # seconds
        sample_rate = 44100
        t = np.linspace(0, duration, int(sample_rate * duration))
        audio_data = np.sin(2 * np.pi * 440 * t)  # 440 Hz sine wave
        
        # Save as WAV file
        import soundfile as sf
        sf.write(TEST_AUDIO_FILE, audio_data, sample_rate)
    
    return TEST_AUDIO_FILE

@pytest.fixture
def test_recording_dir():
    """Create a temporary directory for test recordings."""
    with tempfile.TemporaryDirectory() as temp_dir:
        os.environ["TEST_RECORDING_DIR"] = temp_dir
        yield temp_dir
        if "TEST_RECORDING_DIR" in os.environ:
            del os.environ["TEST_RECORDING_DIR"]

# API Endpoint Tests
class TestAPIEndpoints:
    def test_health_check(self):
        """Test the health check endpoint"""
        response = client.get("/health")
        assert response.status_code == 200
        assert response.json() == {"status": "healthy"}

    def test_start_recording(self, test_recording_dir):
        """Test starting a recording session"""
        response = client.post("/recording/start")
        assert response.status_code == 200
        data = response.json()
        assert "session_id" in data
        assert "message" in data
        assert data["message"] == "Recording started"

    def test_stop_recording(self):
        """Test stopping a recording session"""
        # First start a recording
        start_response = client.post("/recording/start")
        session_id = start_response.json()["session_id"]

        # Then stop it
        response = client.post(f"/recording/stop/{session_id}")
        assert response.status_code == 200
        assert response.json()["status"] == "stopped"

    def test_get_transcription(self, test_audio_file):
        """Test getting transcription for an audio file"""
        with open(test_audio_file, "rb") as f:
            files = {"file": ("test_audio.wav", f, "audio/wav")}
            response = client.post("/transcribe", files=files)
        
        assert response.status_code == 200
        data = response.json()
        assert "transcription" in data
        assert isinstance(data["transcription"], str)

    def test_get_transcription(self):
        """Test getting transcription for a recording"""
        # First start a recording
        start_response = client.post("/recording/start")
        session_id = start_response.json()["session_id"]

        # Stop the recording
        stop_response = client.post(f"/recording/stop/{session_id}")
        assert stop_response.status_code == 200

        # Then get transcription
        response = client.get(f"/transcribe/{session_id}")
        assert response.status_code == 200
        assert "transcription" in response.json()

    def test_get_summary(self):
        """Test getting summary of a transcription"""
        test_text = "This is a test transcription. It should be summarized."
        response = client.post("/summarize", json={"text": test_text})
        
        assert response.status_code == 200
        data = response.json()
        assert "summary" in data
        assert isinstance(data["summary"], str)

    def test_list_meetings(self, test_db):
        """Test listing all meetings"""
        response = client.get("/meetings")
        assert response.status_code == 200
        data = response.json()
        assert isinstance(data, list)

    def test_get_meeting(self, test_db):
        """Test getting a specific meeting"""
        # First create a meeting
        meeting_data = {
            "title": "Test Meeting",
            "transcription": "Test transcription",
            "summary": "Test summary"
        }
        create_response = client.post("/meetings", json=meeting_data)
        meeting_id = create_response.json()["id"]

        # Then get it
        response = client.get(f"/meetings/{meeting_id}")
        assert response.status_code == 200
        data = response.json()
        assert data["title"] == meeting_data["title"]
        assert data["transcription"] == meeting_data["transcription"]
        assert data["summary"] == meeting_data["summary"]

# Service Tests
class TestTranscriptionService:
    def test_transcribe_audio_file(self, test_audio_file):
        """Test transcribing an audio file"""
        service = TranscriptionService()
        transcription = service.transcribe_file(str(test_audio_file))
        assert isinstance(transcription, str)
        assert len(transcription) > 0

    def test_transcribe_audio_chunk(self):
        """Test transcribing an audio chunk"""
        os.environ["TESTING"] = "true"  # Ensure we're in test mode
        service = TranscriptionService()
        
        # Create a simple audio chunk
        duration = 1.0
        sample_rate = 16000
        t = np.linspace(0, duration, int(sample_rate * duration))
        audio_data = np.sin(2 * np.pi * 440 * t)

        transcription = service.transcribe_audio(audio_data)
        assert isinstance(transcription, str)
        assert len(transcription) > 0

class TestSummarizationService:
    def test_summarize_text(self):
        """Test summarizing text"""
        os.environ["TESTING"] = "true"  # Ensure we're in test mode
        service = SummarizationService()
        
        # Test with empty text
        assert service.summarize("") == ""
        
        # Test with short text
        short_text = "This is a short text."
        assert service.summarize(short_text) == short_text
        
        # Test with longer text
        test_text = """
        This is a test transcription. It contains multiple sentences.
        We want to make sure the summarization service can handle
        different types of input text. The summary should be shorter
        than the original text but maintain the key points.
        """
        summary = service.summarize(test_text)
        assert isinstance(summary, str)
        assert len(summary) < len(test_text)  # Summary should be shorter than original
        assert len(summary.split()) < len(test_text.split())  # Summary should have fewer words

    def test_empty_text(self):
        """Test summarizing empty text"""
        service = SummarizationService()
        summary = service.summarize("")
        assert summary == "" or summary is None

class TestAudioService:
    def test_start_stop_recording(self, test_recording_dir):
        """Test starting and stopping audio recording"""
        service = global_audio_service

        # Start recording
        session_id = service.start_recording()
        assert isinstance(session_id, str)
        assert service.is_recording(session_id)

        # Add some test audio data
        duration = 1.0
        sample_rate = 16000
        t = np.linspace(0, duration, int(sample_rate * duration))
        audio_data = np.sin(2 * np.pi * 440 * t)
        service.add_audio_chunk(session_id, audio_data)

        # Stop recording and check the result
        service.stop_recording(session_id)
        assert not service.is_recording(session_id)
        
        # Get the audio data and check it
        audio_data = service.get_audio_data(session_id)
        assert isinstance(audio_data, np.ndarray)

    def test_get_audio_data(self, test_recording_dir):
        """Test getting audio data from the service"""
        os.environ["TESTING"] = "true"  # Ensure we're in test mode
        service = global_audio_service
        
        # Start recording
        session_id = service.start_recording()
        
        # Get the audio data immediately (test data should be present)
        audio_data = service.get_audio_data(session_id)
        assert isinstance(audio_data, np.ndarray)
        assert len(audio_data) > 0  # Test data should be present
        assert audio_data.dtype == np.float64

# Error Handling Tests
class TestErrorHandling:
    def test_invalid_audio_file(self):
        """Test handling invalid audio file"""
        # Create an invalid audio file
        with tempfile.NamedTemporaryFile(suffix=".wav") as temp_file:
            temp_file.write(b"invalid audio data")
            temp_file.seek(0)

            files = {"file": ("invalid.wav", temp_file, "audio/wav")}
            response = client.post("/transcribe", files=files)

        assert response.status_code == 400
        assert "detail" in response.json()
        assert "Failed to read audio file" in response.json()["detail"]

    def test_missing_file(self):
        """Test handling missing file in request"""
        response = client.post("/transcribe")
        assert response.status_code == 422

    def test_invalid_meeting_id(self, test_db):
        """Test handling invalid meeting ID"""
        response = client.get("/meetings/999999")
        assert response.status_code == 404

    def test_invalid_session_id(self):
        """Test handling invalid recording session ID"""
        response = client.post("/recording/stop/invalid_session_id")
        assert response.status_code == 404

# Integration Tests
class TestIntegration:
    def test_full_recording_workflow(self, test_recording_dir):
        """Test the complete recording workflow"""
        os.environ["TESTING"] = "true"  # Ensure we're in test mode
        os.environ["TEST_RECORDING_DIR"] = test_recording_dir
        
        # 1. Start recording
        start_response = client.post("/recording/start")
        assert start_response.status_code == 200
        session_id = start_response.json()["session_id"]

        # 2. Add some test audio data
        duration = 1.0
        sample_rate = 16000
        t = np.linspace(0, duration, int(sample_rate * duration))
        audio_data = np.sin(2 * np.pi * 440 * t)
        global_audio_service.add_audio_chunk(session_id, audio_data)

        # 3. Stop recording
        stop_response = client.post(f"/recording/stop/{session_id}")
        assert stop_response.status_code == 200
        assert stop_response.json()["status"] == "stopped"

        # 4. Get transcription
        transcribe_response = client.get(f"/transcribe/{session_id}")
        assert transcribe_response.status_code == 200
        assert "transcription" in transcribe_response.json()
        assert isinstance(transcribe_response.json()["transcription"], str)

        # 5. Get summary
        transcription = transcribe_response.json()["transcription"]
        summary_response = client.post("/summarize", json={"text": transcription})
        assert summary_response.status_code == 200
        summary = summary_response.json()["summary"]
        
        # 6. Save meeting
        meeting_data = {
            "title": f"Test Meeting {datetime.now()}",
            "transcription": transcription,
            "summary": summary
        }
        save_response = client.post("/meetings", json=meeting_data)
        assert save_response.status_code == 200
        meeting_id = save_response.json()["id"]
        
        # 7. Verify saved meeting
        get_response = client.get(f"/meetings/{meeting_id}")
        assert get_response.status_code == 200
        saved_meeting = get_response.json()
        assert saved_meeting["transcription"] == transcription
        assert saved_meeting["summary"] == summary
