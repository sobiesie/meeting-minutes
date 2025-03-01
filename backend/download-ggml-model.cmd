@echo off
setlocal enabledelayedexpansion

REM This script downloads Whisper model files that have already been converted to ggml format.
REM This way you don't have to convert them yourself.

set "src=https://huggingface.co/ggerganov/whisper.cpp"
set "pfx=resolve/main/ggml"

set "BOLD=[1m"
set "RESET=[0m"

REM Get the path of this script
set "models_path=%~dp0"
if not "%~2"=="" set "models_path=%~2"

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

REM List available models
:list_models
echo.
echo Available models:
set "model_class="
for %%m in (%models%) do (
    for /f "tokens=1 delims=.-" %%c in ("%%m") do (
        if not "%%c"=="!model_class!" (
            echo.
            set "model_class=%%c"
        )
        echo  %%m
    )
)
echo.
exit /b

REM Main script
if "%~1"=="" (
    echo Usage: %~nx0 ^<model^> [models_path]
    call :list_models
    echo ___________________________________________________________
    echo %BOLD%.en%RESET% = english-only %BOLD%-q5_[01]%RESET% = quantized %BOLD%-tdrz%RESET% = tinydiarize
    exit /b 1
)

set "model=%~1"

REM Check if model is valid
set "valid_model=0"
for %%m in (%models%) do (
    if "%%m"=="%model%" set "valid_model=1"
)

if "%valid_model%"=="0" (
    echo Invalid model: %model%
    call :list_models
    exit /b 1
)

REM Check if model contains `tdrz` and update the src and pfx accordingly
echo %model% | findstr /C:"tdrz" >nul
if %ERRORLEVEL% equ 0 (
    set "src=https://huggingface.co/akashmjn/tinydiarize-whisper.cpp"
    set "pfx=resolve/main/ggml"
)

REM Download ggml model
echo Downloading ggml model %model% from '%src%' ...

cd /d "%models_path%"

if exist "whisper.cpp\models" (
    cd whisper.cpp\models
) else if exist "models" (
    cd models
) else (
    mkdir models
    cd models
)

if exist "ggml-%model%.bin" (
    echo Model %model% already exists. Skipping download.
    exit /b 0
)

REM Use PowerShell to download the file
powershell -Command "& {[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '%src%/%pfx%-%model%.bin' -OutFile 'ggml-%model%.bin' -UseBasicParsing}"

if %ERRORLEVEL% neq 0 (
    echo Failed to download ggml model %model%
    echo Please try again later or download the original Whisper model files and convert them yourself.
    exit /b 1
)

echo Done! Model '%model%' saved in '%CD%\ggml-%model%.bin'
echo You can now use it with the Whisper server.
echo.

exit /b 0
