# PowerShell script to start both Whisper server and Python backend with visible output
# This script uses PowerShell's Start-Process to run both servers and show their output

# Set the model name (default: small)
$modelName = "small"
if ($args.Count -gt 0) {
    $modelName = $args[0]
}

# Set the port for Python backend (default: 5167)
$port = 5167
if ($args.Count -gt 1) {
    $port = $args[1]
}

Write-Host "====================================="
Write-Host "Starting Meeting Minutes Backend"
Write-Host "====================================="
Write-Host "Model: $modelName"
Write-Host "Python Backend Port: $port"
Write-Host "====================================="
Write-Host ""

# Kill any existing whisper-server.exe processes
$whisperProcesses = Get-Process -Name "whisper-server" -ErrorAction SilentlyContinue
if ($whisperProcesses) {
    Write-Host "Stopping existing Whisper server processes..."
    $whisperProcesses | ForEach-Object { $_.Kill() }
    Start-Sleep -Seconds 1
}

# Kill any existing python.exe processes
$pythonProcesses = Get-Process -Name "python" -ErrorAction SilentlyContinue
if ($pythonProcesses) {
    Write-Host "Stopping existing Python processes..."
    $pythonProcesses | ForEach-Object { $_.Kill() }
    Start-Sleep -Seconds 1
}

# Check if whisper-server-package exists
if (-not (Test-Path "whisper-server-package")) {
    Write-Host "Error: whisper-server-package directory not found"
    Write-Host "Please run build_whisper.cmd first"
    exit 1
}

# Check if whisper-server.exe exists
if (-not (Test-Path "whisper-server-package\whisper-server.exe")) {
    Write-Host "Error: whisper-server.exe not found"
    Write-Host "Please run build_whisper.cmd first"
    exit 1
}

# Check if the model file exists
$modelFile = "whisper-server-package\models\ggml-$modelName.bin"
if (-not (Test-Path $modelFile)) {
    Write-Host "Error: Model file not found: $modelFile"
    Write-Host "Available models:"
    Get-ChildItem "whisper-server-package\models" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  - $($_.Name)" }
    Write-Host ""
    Write-Host "Please run download-ggml-model.cmd with the correct model name"
    exit 1
}

# Check if virtual environment exists
if (-not (Test-Path "venv")) {
    Write-Host "Error: Virtual environment not found"
    Write-Host "Please run build_whisper.cmd first"
    exit 1
}

# Check if Python app exists
if (-not (Test-Path "app\main.py")) {
    Write-Host "Error: app\main.py not found"
    Write-Host "Please run build_whisper.cmd first"
    exit 1
}

# Start Whisper server in a new window
Write-Host "Starting Whisper server..."
Start-Process -FilePath "cmd.exe" -ArgumentList "/k cd whisper-server-package && whisper-server.exe --model models\ggml-$modelName.bin --host 127.0.0.1 --port 8178 --diarize --print-progress" -WindowStyle Normal

# Wait for Whisper server to start
Write-Host "Waiting for Whisper server to start..."
Start-Sleep -Seconds 5

# Check if Whisper server is running
$whisperRunning = $false
try {
    $whisperProcesses = Get-Process -Name "whisper-server" -ErrorAction Stop
    $whisperRunning = $true
    Write-Host "Whisper server started with PID: $($whisperProcesses.Id)"
} catch {
    Write-Host "Error: Whisper server failed to start"
    exit 1
}

# Start Python backend in a new window
Write-Host "Starting Python backend..."
Start-Process -FilePath "cmd.exe" -ArgumentList "/k call venv\Scripts\activate.bat && set PORT=$port && python app\main.py" -WindowStyle Normal

# Wait for Python backend to start
Write-Host "Waiting for Python backend to start..."
Start-Sleep -Seconds 5

# Check if Python backend is running
$pythonRunning = $false
try {
    $pythonProcesses = Get-Process -Name "python" -ErrorAction Stop
    $pythonRunning = $true
    Write-Host "Python backend started with PID: $($pythonProcesses.Id)"
} catch {
    Write-Host "Error: Python backend failed to start"
    exit 1
}

# Check if services are listening on their ports
Write-Host "Checking if services are listening on their ports..."
$whisperListening = $false
$pythonListening = $false

# Wait a bit longer for services to start listening
Start-Sleep -Seconds 5

# Check Whisper server port
$netstatWhisper = netstat -ano | Select-String -Pattern ":8178.*LISTENING"
if ($netstatWhisper) {
    $whisperListening = $true
    Write-Host "Whisper server is listening on port 8178"
} else {
    Write-Host "Warning: Whisper server is not listening on port 8178"
}

# Check Python backend port
$netstatPython = netstat -ano | Select-String -Pattern ":$port.*LISTENING"
if ($netstatPython) {
    $pythonListening = $true
    Write-Host "Python backend is listening on port $port"
} else {
    Write-Host "Warning: Python backend is not listening on port $port"
}

# Final status
Write-Host ""
Write-Host "====================================="
Write-Host "Backend Status"
Write-Host "====================================="
Write-Host "Whisper Server: $(if ($whisperRunning) { "RUNNING" } else { "NOT RUNNING" })"
Write-Host "Whisper Server Port: $(if ($whisperListening) { "LISTENING on 8178" } else { "NOT LISTENING on 8178" })"
Write-Host "Python Backend: $(if ($pythonRunning) { "RUNNING" } else { "NOT RUNNING" })"
Write-Host "Python Backend Port: $(if ($pythonListening) { "LISTENING on $port" } else { "NOT LISTENING on $port" })"
Write-Host ""
Write-Host "The backend services are now running in separate windows."
Write-Host "You can close those windows to stop the services."
Write-Host "====================================="
