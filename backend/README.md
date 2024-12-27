# Meeting Minutes Assistant Backend

A privacy-focused AI-powered meeting assistant that handles audio recording, real-time transcription, and meeting summarization.

## Features

- Real-time audio recording (both system and microphone)
- Live transcription using Whisper
- Meeting summarization using Ollama (Qwen 2.5:3b model)
- WebSocket streaming for real-time updates
- Secure data storage with SQLite
- Privacy-focused design with local processing

## Prerequisites

- Python 3.8+
- PortAudio (for audio recording)
- FFmpeg (for Whisper)
- Ollama (for summarization)

### System Dependencies Installation

On macOS:
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install PortAudio and FFmpeg
brew install portaudio ffmpeg

# Install Ollama
brew install ollama

# Pull the Qwen model
ollama pull qwen:3b
```

## Quick Install

The easiest way to set up the backend is to use the installer script:

```bash
# Make the installer executable
chmod +x install.sh

# Run the installer
./install.sh
```

## Manual Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd MeetingMinutes/backend
```

2. Create and activate a virtual environment:
```bash
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
```

3. Install dependencies:
```bash
pip install -r requirements.txt
```

4. Copy the example environment file and configure your settings:
```bash
cp .env.example .env
```

5. Edit `.env` and configure your settings:
- Set the Ollama URL (default: http://localhost:11434)
- Configure audio settings if needed
- Set database path if needed

## Running the Application

1. Start the Ollama service:
```bash
ollama serve
```

2. Start the FastAPI server:
```bash
uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
```

## API Endpoints

### REST Endpoints

- `POST /meetings/start` - Start a new meeting recording
- `POST /meetings/{meeting_id}/stop` - Stop recording and generate summary
- `GET /meetings/{meeting_id}` - Get meeting details, transcript, and summary

### WebSocket Endpoints

- `ws://localhost:8000/ws/meetings/{meeting_id}/stream` - Stream real-time transcription

## Usage Example

1. Start a new meeting:
```bash
curl -X POST "http://localhost:8000/meetings/start?name=Team%20Meeting"
```

2. Connect to WebSocket for real-time transcription:
```javascript
const ws = new WebSocket('ws://localhost:8000/ws/meetings/1/stream');
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('Transcript:', data.transcript);
};
```

3. Stop the meeting:
```bash
curl -X POST "http://localhost:8000/meetings/1/stop"
```

4. Get meeting summary:
```bash
curl "http://localhost:8000/meetings/1"
```

## Development

Run tests:
```bash
pytest -v
```

For test coverage:
```bash
pytest --cov=app tests/
```

## Security and Privacy

- All audio processing is done locally using Whisper
- Data is stored in an encrypted SQLite database
- No audio data is sent to external services
- Summarization is done locally using Ollama

## Troubleshooting

### Audio Recording Issues
- Make sure PortAudio is installed: `brew install portaudio`
- Check if your microphone is properly connected and has permissions
- Try running `python3 -m sounddevice` to test audio device detection

### Transcription Issues
- Ensure FFmpeg is installed: `brew install ffmpeg`
- Check if Whisper model is properly downloaded
- Verify audio file format and quality

### Summarization Issues
- Verify Ollama is running: `ollama serve`
- Check if Qwen model is pulled: `ollama pull qwen:3b`
- Verify Ollama API endpoint in .env file

## License

[MIT License](LICENSE)
