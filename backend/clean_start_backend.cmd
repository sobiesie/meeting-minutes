@echo off
setlocal enabledelayedexpansion

REM Color codes for Windows console
set "GREEN=[92m"
set "BLUE=[94m"
set "YELLOW=[93m"
set "RED=[91m"
set "PURPLE=[95m"
set "NC=[0m"

REM Configuration
set "PACKAGE_NAME=whisper-server-package"
set "MODEL_DIR=%PACKAGE_NAME%\models"
set "PORT=5167"

REM Helper functions for logging
:log_info
echo %BLUE%[INFO]%NC% %~1
exit /b 0

:log_success
echo %GREEN%[SUCCESS]%NC% %~1
exit /b 0

:log_warning
echo %YELLOW%[WARNING]%NC% %~1
exit /b 0

:log_error
echo %RED%[ERROR]%NC% %~1
exit /b 1

:log_section
echo.
echo %PURPLE%=== %~1 ===%NC%
echo.
exit /b 0

REM Error handling function
:handle_error
call :log_error "%~1"
call :cleanup
exit /b 1

REM Cleanup function
:cleanup
call :log_section "Cleanup"

if defined WHISPER_PID (
    call :log_info "Stopping Whisper server..."
    taskkill /F /PID !WHISPER_PID! 2>nul
    if !ERRORLEVEL! equ 0 (
        call :log_success "Whisper server stopped"
    ) else (
        call :log_warning "Failed to kill Whisper server process"
    )
    
    taskkill /F /FI "IMAGENAME eq whisper-server.exe" 2>nul
    if !ERRORLEVEL! equ 0 (
        call :log_success "All whisper-server processes stopped"
    )
)

if defined PYTHON_PID (
    call :log_info "Stopping Python backend..."
    taskkill /F /PID !PYTHON_PID! 2>nul
    if !ERRORLEVEL! equ 0 (
        call :log_success "Python backend stopped"
    ) else (
        call :log_warning "Failed to kill Python backend process"
    )
)
exit /b 0

REM Check if required directories and files exist
call :log_section "Environment Check"

if not exist "%PACKAGE_NAME%" (
    call :handle_error "Whisper server directory not found. Please run build_whisper.cmd first"
    exit /b 1
)

if not exist "app" (
    call :handle_error "Python backend directory not found. Please check your installation"
    exit /b 1
)

if not exist "app\main.py" (
    call :handle_error "Python backend main.py not found. Please check your installation"
    exit /b 1
)

if not exist "venv" (
    call :handle_error "Virtual environment not found. Please run build_whisper.cmd first"
    exit /b 1
)

REM Kill any existing whisper-server processes
call :log_section "Initial Cleanup"

call :log_info "Checking for existing whisper servers..."
taskkill /F /FI "IMAGENAME eq whisper-server.exe" 2>nul
if %ERRORLEVEL% equ 0 (
    call :log_success "Existing whisper servers terminated"
) else (
    call :log_warning "No existing whisper servers found"
)
timeout /t 1 >nul

REM Check and kill if backend app in port 5167 is running
call :log_section "Backend App Check"

call :log_info "Checking for processes on port 5167..."
set "PORT_IN_USE="
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":5167.*LISTENING"') do (
    set "PORT_IN_USE=%%a"
)

if defined PORT_IN_USE (
    call :log_warning "Backend app is running on port %PORT%"
    set /p REPLY="Kill it? (y/N) "
    if /i not "!REPLY!"=="y" (
        call :handle_error "User chose not to terminate existing backend app"
        exit /b 1
    )
    
    call :log_info "Terminating backend app..."
    taskkill /F /PID !PORT_IN_USE! 2>nul
    if !ERRORLEVEL! equ 0 (
        call :log_success "Backend app terminated"
    ) else (
        call :handle_error "Failed to terminate backend app"
        exit /b 1
    )
    timeout /t 1 >nul
)

REM Check for existing model
call :log_section "Model Check"

if not exist "%MODEL_DIR%" (
    call :handle_error "Models directory not found. Please run build_whisper.cmd first"
    exit /b 1
)

call :log_info "Checking for Whisper models..."
set "EXISTING_MODELS="
for /f "delims=" %%a in ('dir /b /s "%MODEL_DIR%\ggml-*.bin" 2^>nul') do (
    set "EXISTING_MODELS=!EXISTING_MODELS!%%a
"
)

if defined EXISTING_MODELS (
    call :log_success "Found existing models:"
    echo %BLUE%%EXISTING_MODELS%%NC%
) else (
    call :log_warning "No existing models found"
)

REM Whisper models
set "models=tiny
tiny.en
tiny-q5_1
tiny.en-q5_1
tiny-q8_0
base
base.en
base-q5_1
base.en-q5_1
base-q8_0
small
small.en
small.en-tdrz
small-q5_1
small.en-q5_1
small-q8_0
medium
medium.en
medium-q5_0
medium.en-q5_0
medium-q8_0
large-v1
large-v2
large-v2-q5_0
large-v2-q8_0
large-v3
large-v3-q5_0
large-v3-turbo
large-v3-turbo-q5_0
large-v3-turbo-q8_0"

REM Ask user which model to use if the argument is not provided
set "MODEL_SHORT_NAME="
if "%~1"=="" (
    call :log_section "Model Selection"
    call :log_info "Available models:"
    echo %BLUE%%models%%NC%
    set /p MODEL_SHORT_NAME="Enter a model name (e.g. small): "
) else (
    set "MODEL_SHORT_NAME=%~1"
)

REM Check if the model is valid
set "MODEL_VALID=0"
for %%m in (%models%) do (
    if "%%m"=="%MODEL_SHORT_NAME%" set "MODEL_VALID=1"
)

if "%MODEL_VALID%"=="0" (
    call :handle_error "Invalid model: %MODEL_SHORT_NAME%"
    exit /b 1
)

set "MODEL_NAME=ggml-%MODEL_SHORT_NAME%.bin"
call :log_success "Selected model: %MODEL_NAME%"

REM Check if the modelname exists in directory
if exist "%MODEL_DIR%\%MODEL_NAME%" (
    call :log_success "Model file exists: %MODEL_DIR%\%MODEL_NAME%"
) else (
    call :log_warning "Model file does not exist: %MODEL_DIR%\%MODEL_NAME%"
    call :log_info "Downloading model..."
    
    call download-ggml-model.cmd %MODEL_SHORT_NAME%
    if %ERRORLEVEL% neq 0 (
        call :handle_error "Failed to download model"
        exit /b 1
    )
    
    REM Move model to models directory
    move "whisper.cpp\models\%MODEL_NAME%" "%MODEL_DIR%\"
    if %ERRORLEVEL% neq 0 (
        call :handle_error "Failed to move model to models directory"
        exit /b 1
    )
)

call :log_section "Starting Services"

REM Start the whisper server in background
call :log_info "Starting Whisper server..."
cd "%PACKAGE_NAME%" || (
    call :handle_error "Failed to change to whisper-server directory"
    exit /b 1
)

REM Start the server and capture its PID
start /b cmd /c "run-server.cmd --model models\%MODEL_NAME% > ..\whisper-server.log 2>&1"
for /f "tokens=2" %%a in ('tasklist /fi "imagename eq whisper-server.exe" /fo list ^| findstr "PID:"') do (
    set "WHISPER_PID=%%a"
)

cd ..

REM Wait for server to start and check if it's running
timeout /t 2 >nul
tasklist /fi "PID eq %WHISPER_PID%" >nul 2>&1
if %ERRORLEVEL% neq 0 (
    call :handle_error "Whisper server failed to start"
    exit /b 1
)

REM Start the Python backend in background
call :log_info "Starting Python backend..."

REM Activate virtual environment
call :log_info "Activating virtual environment..."
call venv\Scripts\activate.bat
if %ERRORLEVEL% neq 0 (
    call :handle_error "Failed to activate virtual environment"
    exit /b 1
)

REM Check if required Python packages are installed
pip show fastapi >nul 2>&1
if %ERRORLEVEL% neq 0 (
    call :handle_error "FastAPI not found. Please run build_whisper.cmd to install dependencies"
    exit /b 1
)

REM Start the Python backend and capture its PID
start /b cmd /c "python app\main.py > python-backend.log 2>&1"
timeout /t 2 >nul
for /f "tokens=2" %%a in ('tasklist /fi "imagename eq python.exe" /fo list ^| findstr "PID:"') do (
    set "PYTHON_PID=%%a"
)

REM Wait for backend to start
timeout /t 10 >nul

REM Check if the port is actually listening
netstat -ano | findstr ":5167.*LISTENING" >nul
if %ERRORLEVEL% neq 0 (
    call :handle_error "Python backend is not listening on port %PORT%"
    exit /b 1
)

call :log_success "All services started successfully!"
echo %GREEN%Whisper Server (PID: %WHISPER_PID%)%NC%
echo %GREEN%Python Backend (PID: %PYTHON_PID%)%NC%
echo %BLUE%Press Ctrl+C to stop all services%NC%

REM Show whisper server port and python backend port
echo %BLUE%Whisper Server Port: 8178%NC%
echo %BLUE%Python Backend Port: %PORT%%NC%

REM Keep the script running
echo.
echo Servers are running. Press Ctrl+C to stop...
pause >nul

REM Cleanup on exit
call :cleanup
exit /b 0
