import os
import uuid
import pyaudio
import wave
import threading
import queue
import numpy as np
from datetime import datetime
from typing import Dict, Optional, List, Tuple

class AudioService:
    def __init__(self):
        self.recordings = {}  # Dictionary to store active recording sessions
        self.completed_recordings = set()  # Set to track completed recording sessions
        self.recording_threads = {}  # Dictionary to store recording threads
        self.recording_queues = {}  # Dictionary to store recording queues
        self.stop_flags = {}  # Dictionary to store stop flags
        
        # Use absolute path for recordings directory
        base_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        self.recording_dir = os.path.join(base_dir, os.getenv("RECORDING_DIR", "recordings"))
        
        if os.getenv("TESTING") == "true":
            self.recording_dir = os.getenv("TEST_RECORDING_DIR", self.recording_dir)
        
        os.makedirs(self.recording_dir, exist_ok=True)
        print(f"Recording directory: {self.recording_dir}")
        
        # Audio settings - match BlackHole's default settings
        self.format = pyaudio.paFloat32
        self.channels = 2
        self.rate = 48000  # BlackHole's default rate
        self.chunk = 1024
        self.audio = pyaudio.PyAudio()

    def _find_audio_devices(self):
        """Find both microphone and BlackHole devices."""
        blackhole_index = None
        mic_index = None
        
        print("\nAvailable Audio Devices:")
        for i in range(self.audio.get_device_count()):
            dev_info = self.audio.get_device_info_by_index(i)
            name = dev_info['name']
            print(f"Device {i}: {name}")
            print(f"  Max Input Channels: {dev_info['maxInputChannels']}")
            print(f"  Max Output Channels: {dev_info['maxOutputChannels']}")
            print(f"  Default Sample Rate: {dev_info['defaultSampleRate']}")
            
            # Look for BlackHole device
            if 'blackhole' in name.lower():
                blackhole_index = i
                self.rate = int(dev_info['defaultSampleRate'])
            
            # Look for built-in microphone
            elif ('built-in' in name.lower() or 'microphone' in name.lower()) and dev_info['maxInputChannels'] > 0:
                mic_index = i
        
        if blackhole_index is None:
            raise ValueError(
                "BlackHole audio device not found. Please run the install script again:\n"
                "./install.sh\n"
                "Then make sure to select 'Meeting Audio' as the output device in your meeting application"
            )
            
        if mic_index is None:
            print("Warning: No microphone found, will only record system audio")
            
        return blackhole_index, mic_index

    def start_recording(self) -> str:
        """Start recording both microphone and application audio."""
        session_id = str(uuid.uuid4())
        
        try:
            # Find audio devices
            blackhole_index, mic_index = self._find_audio_devices()
            
            print(f"\nUsing BlackHole for system audio capture")
            if mic_index is not None:
                print(f"Using built-in microphone for voice capture")
            
            # Create queues for audio chunks
            self.recording_queues[session_id] = {
                'system': queue.Queue(),
                'mic': queue.Queue() if mic_index is not None else None
            }
            self.stop_flags[session_id] = threading.Event()
            
            # Start recording threads
            threads = []
            
            # System audio thread
            system_thread = threading.Thread(
                target=self._record_audio,
                args=(session_id, blackhole_index, 'system')
            )
            system_thread.daemon = True
            system_thread.start()
            threads.append(system_thread)
            
            # Microphone thread (if available)
            if mic_index is not None:
                mic_thread = threading.Thread(
                    target=self._record_audio,
                    args=(session_id, mic_index, 'mic')
                )
                mic_thread.daemon = True
                mic_thread.start()
                threads.append(mic_thread)
            
            self.recording_threads[session_id] = threads
            return session_id
            
        except Exception as e:
            print(f"Error starting recording: {str(e)}")
            self.cleanup_session(session_id)
            raise ValueError(f"Failed to start recording: {str(e)}")

    def _record_audio(self, session_id: str, device_index: int, source: str) -> None:
        """Record audio from a specific device in a separate thread."""
        stream = None
        try:
            # Open an audio stream
            stream = self.audio.open(
                format=self.format,
                channels=self.channels,
                rate=self.rate,
                input=True,
                input_device_index=device_index,
                frames_per_buffer=self.chunk,
                stream_callback=None
            )
            
            print(f"Started recording from {source} with settings:")
            print(f"  Format: {self.format}")
            print(f"  Channels: {self.channels}")
            print(f"  Rate: {self.rate}")
            print(f"  Device Index: {device_index}")
            
            while not self.stop_flags[session_id].is_set():
                try:
                    data = stream.read(self.chunk, exception_on_overflow=False)
                    self.recording_queues[session_id][source].put(data)
                except OSError as e:
                    print(f"Warning in {source} recording: {str(e)}")
                    continue
                    
        except Exception as e:
            print(f"Error in {source} recording thread: {str(e)}")
            raise
        finally:
            if stream is not None:
                try:
                    stream.stop_stream()
                    stream.close()
                except:
                    pass

    def stop_recording(self, session_id: str) -> None:
        """Stop recording and mix the audio streams."""
        if session_id in self.completed_recordings:
            return
            
        if session_id not in self.recording_threads:
            raise ValueError(f"Recording session {session_id} not found")
        
        try:
            # Signal the recording threads to stop
            self.stop_flags[session_id].set()
            for thread in self.recording_threads[session_id]:
                thread.join(timeout=5)
            
            # Get all audio chunks and mix them
            system_frames = []
            mic_frames = []
            
            # Get system audio
            while not self.recording_queues[session_id]['system'].empty():
                system_frames.append(self.recording_queues[session_id]['system'].get())
            
            # Get mic audio if available
            if self.recording_queues[session_id]['mic'] is not None:
                while not self.recording_queues[session_id]['mic'].empty():
                    mic_frames.append(self.recording_queues[session_id]['mic'].get())
            
            if not system_frames and not mic_frames:
                raise ValueError("No audio data recorded")
            
            # Mix the audio streams
            mixed_frames = self._mix_audio(system_frames, mic_frames)
            
            # Save the mixed audio
            file_path = os.path.join(self.recording_dir, f"{session_id}.wav")
            with wave.open(file_path, 'wb') as wf:
                wf.setnchannels(self.channels)
                wf.setsampwidth(self.audio.get_sample_size(self.format))
                wf.setframerate(self.rate)
                wf.writeframes(mixed_frames)
            
            print(f"Saved recording to: {file_path}")
            print(f"File size: {os.path.getsize(file_path)} bytes")
            
            # Mark as completed
            self.completed_recordings.add(session_id)
            self.cleanup_session(session_id)
            
        except Exception as e:
            print(f"Error in stop_recording: {str(e)}")
            self.cleanup_session(session_id)
            raise

    def _mix_audio(self, system_frames: List[bytes], mic_frames: List[bytes]) -> bytes:
        """Mix system audio and microphone audio with proper levels."""
        if not system_frames and not mic_frames:
            return b''
            
        # Convert bytes to numpy arrays
        if system_frames:
            system_data = np.frombuffer(b''.join(system_frames), dtype=np.float32)
        else:
            system_data = np.zeros(len(mic_frames) * self.chunk, dtype=np.float32)
            
        if mic_frames:
            mic_data = np.frombuffer(b''.join(mic_frames), dtype=np.float32)
            # Adjust microphone volume (can be tweaked)
            mic_data *= 1.2  # Slightly boost mic volume
        else:
            mic_data = np.zeros(len(system_frames) * self.chunk, dtype=np.float32)
        
        # Ensure both arrays are the same length
        min_length = min(len(system_data), len(mic_data))
        system_data = system_data[:min_length]
        mic_data = mic_data[:min_length]
        
        # Mix the audio streams
        mixed_data = system_data + mic_data
        
        # Normalize to prevent clipping
        max_val = np.max(np.abs(mixed_data))
        if max_val > 1.0:
            mixed_data /= max_val
        
        return mixed_data.tobytes()

    def cleanup_session(self, session_id: str) -> None:
        """Clean up recording resources for a session."""
        if session_id in self.recording_threads:
            del self.recording_threads[session_id]
        if session_id in self.recording_queues:
            del self.recording_queues[session_id]
        if session_id in self.stop_flags:
            del self.stop_flags[session_id]

    def get_recording_path(self, session_id: str) -> str:
        """Get the path to a recording file."""
        file_path = os.path.join(self.recording_dir, f"{session_id}.wav")
        if not os.path.exists(file_path):
            if session_id in self.recording_threads:
                raise ValueError(f"Recording {session_id} is still in progress")
            elif session_id not in self.completed_recordings:
                raise ValueError(f"Recording {session_id} not found or was never started")
            else:
                raise ValueError(f"Recording file for {session_id} is missing")
        return file_path

    def __del__(self):
        """Cleanup when the service is destroyed."""
        self.audio.terminate()

# Create a mock audio service for testing
class MockAudioService:
    def __init__(self):
        self.recordings = {}
        self.completed_recordings = set()
        self.recording_dir = "test_recordings"
        os.makedirs(self.recording_dir, exist_ok=True)

    def start_recording(self) -> str:
        """Start a new recording session."""
        session_id = str(uuid.uuid4())
        self.recordings[session_id] = []
        
        # Add test audio data
        duration = 1.0
        sample_rate = 16000
        t = np.linspace(0, duration, int(sample_rate * duration))
        test_audio = np.sin(2 * np.pi * 440 * t)
        self.recordings[session_id].append(test_audio)
        
        return session_id

    def add_audio_chunk(self, session_id: str, audio_data: np.ndarray) -> None:
        """Add an audio chunk to the recording."""
        if session_id not in self.recordings:
            raise ValueError(f"Recording session {session_id} not found")
        self.recordings[session_id].append(audio_data)

    def stop_recording(self, session_id: str) -> None:
        """Stop the recording session."""
        if session_id in self.completed_recordings:
            # Session already stopped, return without error
            return
            
        if session_id not in self.recordings:
            raise ValueError(f"Recording session {session_id} not found")
        
        # Save the audio data to a file
        file_path = os.path.join(self.recording_dir, f"{session_id}.wav")
        audio_data = np.concatenate(self.recordings[session_id])
        self.save_audio_file(audio_data, file_path)
        
        # Mark session as completed and clean up
        self.completed_recordings.add(session_id)
        del self.recordings[session_id]

    def get_audio_data(self, session_id: str) -> np.ndarray:
        """Get the audio data for a recording session."""
        # First check if we have in-memory data
        if session_id in self.recordings:
            return np.concatenate(self.recordings[session_id])
            
        # If not, try to read from file
        file_path = os.path.join(self.recording_dir, f"{session_id}.wav")
        if not os.path.exists(file_path):
            raise ValueError(f"Recording {session_id} not found")
        
        sample_rate, audio_data = wave.open(file_path, 'rb').readframes(-1)
        return audio_data

    def is_recording(self, session_id: str) -> bool:
        """Check if a recording session exists."""
        return session_id in self.recordings

    def save_audio_file(self, audio_data: np.ndarray, file_path: str) -> None:
        """Save audio data to a WAV file."""
        os.makedirs(os.path.dirname(os.path.abspath(file_path)), exist_ok=True)
        sample_rate = 16000
        wave.open(file_path, 'wb').writeframes(audio_data.astype(np.float32))

    def get_recording_path(self, session_id: str) -> str:
        """Get the path to a recording file."""
        file_path = os.path.join(self.recording_dir, f"{session_id}.wav")
        if not os.path.exists(file_path):
            if session_id in self.recordings:
                raise ValueError(f"Recording {session_id} is still in progress")
            elif session_id not in self.completed_recordings:
                raise ValueError(f"Recording {session_id} not found or was never started")
            else:
                raise ValueError(f"Recording file for {session_id} is missing")
        return file_path

# Global instance of the audio service
# Use mock service if in test environment
if os.getenv("TESTING") == "true":
    audio_service = MockAudioService()
else:
    audio_service = AudioService()
