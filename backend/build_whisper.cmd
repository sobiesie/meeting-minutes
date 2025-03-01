@echo off
setlocal enabledelayedexpansion

REM Color codes for Windows console
set "GREEN=[92m"
set "BLUE=[94m"
set "YELLOW=[93m"
set "RED=[91m"
set "NC=[0m"

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
echo %BLUE%=== %~1 ===%NC%
echo.
exit /b 0

REM Main script
call :log_section "Starting Whisper.cpp Build Process"

call :log_info "Updating git submodules..."
git submodule update --init --recursive
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to update git submodules"
    exit /b 1
)

call :log_info "Checking for whisper.cpp directory..."
if not exist "whisper.cpp" (
    call :log_error "Directory 'whisper.cpp' not found. Please make sure you're in the correct directory and the submodule is initialized"
    exit /b 1
)

call :log_info "Changing to whisper.cpp directory..."
cd whisper.cpp
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to change to whisper.cpp directory"
    exit /b 1
)

call :log_info "Checking for custom server directory..."
if not exist "..\whisper-custom\server" (
    call :log_error "Directory '../whisper-custom/server' not found. Please make sure the custom server files exist"
    exit /b 1
)

call :log_info "Copying custom server files..."
xcopy /E /Y /I "..\whisper-custom\server\*" "examples\server\"
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to copy custom server files"
    exit /b 1
)
call :log_success "Custom server files copied successfully"

call :log_info "Verifying server files..."
dir "examples\server\"
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to list server files"
    exit /b 1
)

call :log_section "Building Whisper Server"
call :log_info "Running cmake build..."

REM Create build directory if it doesn't exist
if not exist "build" mkdir build
cd build

REM Run CMake to configure the project
call :log_info "Running CMake configuration..."
cmake .. -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_EXAMPLES=ON -DWHISPER_BUILD_TESTS=OFF
if %ERRORLEVEL% neq 0 (
    call :log_error "CMake configuration failed"
    exit /b 1
)

REM Build the project
call :log_info "Building with CMake..."
cmake --build . --config Release
if %ERRORLEVEL% neq 0 (
    call :log_error "Build failed"
    exit /b 1
)

cd ..
call :log_success "Build completed successfully"

REM Configuration
set "PACKAGE_NAME=whisper-server-package"
set "MODEL_NAME=ggml-small.bin"
set "MODEL_DIR=%PACKAGE_NAME%\models"

call :log_section "Package Configuration"
call :log_info "Package name: %PACKAGE_NAME%"
call :log_info "Model name: %MODEL_NAME%"
call :log_info "Model directory: %MODEL_DIR%"

REM Create necessary directories
call :log_info "Creating package directories..."
if not exist "%PACKAGE_NAME%" mkdir "%PACKAGE_NAME%"
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to create package directory"
    exit /b 1
)

if not exist "%MODEL_DIR%" mkdir "%MODEL_DIR%"
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to create models directory"
    exit /b 1
)
call :log_success "Package directories created successfully"

REM Copy server binary
call :log_info "Copying server binary..."
copy "build\bin\Release\whisper-server.exe" "%PACKAGE_NAME%\"
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to copy server binary"
    exit /b 1
)
call :log_success "Server binary copied successfully"

REM Check for existing models
call :log_section "Model Management"
call :log_info "Checking for existing Whisper models..."

set "EXISTING_MODELS="
for /f "delims=" %%a in ('dir /b /s "%MODEL_DIR%\ggml-*.bin" 2^>nul') do (
    set "EXISTING_MODELS=!EXISTING_MODELS!%%a
"
)

if defined EXISTING_MODELS (
    call :log_info "Found existing models:"
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
    REM Let user interactively select a model name
    call :log_info "Available models: %models%"
    set /p MODEL_SHORT_NAME="Enter a model name (e.g. small): "
) else (
    set "MODEL_SHORT_NAME=%~1"
)

REM Check if the model is valid
set "MODEL_VALID=0"
for /f "tokens=*" %%a in ('echo %models%') do (
    if "%%a"=="%MODEL_SHORT_NAME%" set "MODEL_VALID=1"
)

if "%MODEL_VALID%"=="0" (
    call :log_error "Invalid model: %MODEL_SHORT_NAME%"
    exit /b 1
)

set "MODEL_NAME=ggml-%MODEL_SHORT_NAME%.bin"

REM Check if the modelname exists in directory
if exist "%MODEL_DIR%\%MODEL_NAME%" (
    call :log_info "Model file exists: %MODEL_DIR%\%MODEL_NAME%"
) else (
    call :log_warning "Model file does not exist: %MODEL_DIR%\%MODEL_NAME%"
    call :log_info "Trying to download model..."
    
    REM Run the download script
    cd ..
    call download-ggml-model.cmd %MODEL_SHORT_NAME%
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to download model"
        exit /b 1
    )
    
    REM Move model to models directory
    move "whisper.cpp\models\%MODEL_NAME%" "whisper.cpp\%MODEL_DIR%\"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to move model to models directory"
        exit /b 1
    )
    cd whisper.cpp
)

REM Create run script
call :log_info "Creating run script..."
(
    echo @echo off
    echo REM Default configuration
    echo set "HOST=127.0.0.1"
    echo set "PORT=8178"
    echo set "MODEL=models\ggml-large-v3.bin"
    echo.
    echo REM Parse command line arguments
    echo :parse_args
    echo if "%%~1"=="" goto run
    echo if "%%~1"=="--host" (
    echo     set "HOST=%%~2"
    echo     shift /2
    echo     goto parse_args
    echo )
    echo if "%%~1"=="--port" (
    echo     set "PORT=%%~2"
    echo     shift /2
    echo     goto parse_args
    echo )
    echo if "%%~1"=="--model" (
    echo     set "MODEL=%%~2"
    echo     shift /2
    echo     goto parse_args
    echo )
    echo echo Unknown option: %%~1
    echo exit /b 1
    echo.
    echo :run
    echo REM Run the server
    echo whisper-server.exe ^
    echo     --model "%%MODEL%%" ^
    echo     --host "%%HOST%%" ^
    echo     --port "%%PORT%%" ^
    echo     --diarize ^
    echo     --print-progress
) > "%PACKAGE_NAME%\run-server.cmd"

call :log_success "Run script created successfully"

call :log_info "Listing files..."
dir
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to list files"
    exit /b 1
)

REM Check if package directory already exists
cd ..
if exist "%PACKAGE_NAME%" (
    call :log_info "Listing parent directory..."
    call :log_warning "Package directory already exists: %PACKAGE_NAME%"
    call :log_info "Listing package directory..."
) else (
    call :log_info "Creating package directory: %PACKAGE_NAME%"
    mkdir "%PACKAGE_NAME%"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to create package directory"
        exit /b 1
    )
    call :log_success "Package directory created successfully"
)

REM Move whisper-server package out of whisper.cpp to PACKAGE_NAME
if exist "%PACKAGE_NAME%" (
    call :log_info "Copying package contents to existing directory..."
    xcopy /E /Y /I "whisper.cpp\%PACKAGE_NAME%\*" "%PACKAGE_NAME%\"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to copy package contents"
        exit /b 1
    )
) else (
    call :log_info "Copying whisper-server and model to %PACKAGE_NAME%"
    copy "whisper.cpp\%MODEL_DIR%\%MODEL_NAME%" "%PACKAGE_NAME%\models\"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to copy model"
        exit /b 1
    )
    
    copy "whisper.cpp\%PACKAGE_NAME%\run-server.cmd" "%PACKAGE_NAME%\"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to copy run script"
        exit /b 1
    )
    
    xcopy /E /Y /I "whisper.cpp\%PACKAGE_NAME%\public" "%PACKAGE_NAME%\public\"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to copy public directory"
        exit /b 1
    )
    
    copy "whisper.cpp\%PACKAGE_NAME%\whisper-server.exe" "%PACKAGE_NAME%\"
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to copy whisper-server"
        exit /b 1
    )
)

call :log_section "Environment Setup"
call :log_info "Setting up environment variables..."
copy temp.env .env
if %ERRORLEVEL% neq 0 (
    call :log_error "Failed to copy environment variables"
    exit /b 1
)

call :log_info "If you want to use Models hosted on Anthropic, OpenAi or GROQ, add the API keys to the .env file."

call :log_section "Build Process Complete"
call :log_success "Whisper.cpp server build and setup completed successfully!"

call :log_section "Installing python dependencies"
call :log_info "Installing python dependencies..."

REM Create virtual environment only if it doesn't exist
if not exist "venv" (
    call :log_info "Creating virtual environment..."
    python -m venv venv
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to create virtual environment"
        exit /b 1
    )
    
    call venv\Scripts\activate.bat
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to activate virtual environment"
        exit /b 1
    )
    
    pip install -r requirements.txt
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to install dependencies"
        exit /b 1
    )
) else (
    call :log_info "Virtual environment already exists"
    call venv\Scripts\activate.bat
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to activate virtual environment"
        exit /b 1
    )
    
    pip install -r requirements.txt
    if %ERRORLEVEL% neq 0 (
        call :log_error "Failed to install dependencies"
        exit /b 1
    )
)

call :log_success "Dependencies installed successfully"

echo %GREEN%You can now proceed with running the server by running 'clean_start_backend.cmd'%NC%

exit /b 0
