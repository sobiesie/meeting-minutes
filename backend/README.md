# Meeting Minutes Backend

FastAPI backend for meeting transcription and analysis

## Features
- Audio file upload and storage
- Real-time Whisper-based transcription with streaming support
- Meeting analysis with LLMs (supports Claude, Groq, and Ollama)
- REST API endpoints

## Requirements
- Python 3.9+
- FFmpeg
- C++ compiler (for Whisper.cpp)
- CMake
- Git (for submodules)
- Ollama running
- API Keys (for Claude or Groq) if planning to use APIS
- ChromaDB

## Installation

### 1. Environment Setup
Create `.env` file in the backend directory:
```bash
ANTHROPIC_API_KEY=your_key_here  # Optional, for Claude
GROQ_API_KEY=your_key_here      # Optional, for Groq
```

### 2. Build Whisper Server

#### For Windows:
Run the build script which will:
- Initialize and update git submodules
- Build Whisper.cpp with custom server modifications
- Set up the server package with required files
- Download the selected Whisper model

```cmd
build_whisper.cmd
```

If no model is specified, the script will prompt you to choose one interactively.

#### For macOS/Linux:
```bash
./build_whisper.sh
```

### 3. Running the Server

#### For Windows:
The PowerShell script provides an interactive way to start the backend services:

```cmd
start_with_output.cmd
```

Or directly with PowerShell:
```powershell
powershell -ExecutionPolicy Bypass -File start_with_output.ps1
```

The script will:
1. Check and clean up any existing processes
2. Display available models and prompt for selection
3. Download the selected model if not present
4. Start the Whisper server in a new window
5. Start the FastAPI backend in a new window

To stop all services, close the command windows or press Ctrl+C in each window.

#### For macOS/Linux:
```bash
./clean_start_backend.sh
```

## API Documentation
Access Swagger UI at `http://localhost:5167/docs`

## Services
The backend runs two services:
1. Whisper.cpp Server: Handles real-time audio transcription
2. FastAPI Backend: Manages API endpoints, LLM integration, and data storage

## Windows-Specific Information
- The Windows scripts create separate command windows for each service, allowing you to see the output in real-time
- You can check the status of services using `check_status.cmd`
- If you prefer to start services individually:
  - `start_whisper_server.cmd [model]` - Starts just the Whisper server
  - `start_python_backend.cmd [port]` - Starts just the Python backend

## Troubleshooting

### Common Issues on Windows
- If you see "whisper-server.exe not found", run `build_whisper.cmd` first
- If a model fails to download, try running `download-ggml-model.cmd [model]` directly
- If services don't start, check if ports 8178 (Whisper) and 5167 (Backend) are available
- Ensure you have administrator privileges when running the scripts
- If PowerShell script execution is blocked, run PowerShell as administrator and use:
  ```powershell
  Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process
  ```

### General Troubleshooting
- If services fail to start, the script will automatically clean up processes
- Check logs for detailed error messages
- Ensure all ports (5167 for backend, 8178 for Whisper) are available
- Verify API keys if using Claude or Groq
- For Ollama, ensure the Ollama service is running and models are pulled
- If build fails:
  - Ensure all dependencies (CMake, C++ compiler) are installed
  - Check if git submodules are properly initialized
  - Verify you have write permissions in the directory
