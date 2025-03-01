@echo off
setlocal enabledelayedexpansion

REM Default model name
set "MODEL_NAME=small"
if "%~1" neq "" (
    set "MODEL_NAME=%~1"
)

REM Default port
set "PORT=5167"
if "%~2" neq "" (
    set "PORT=%~2"
)

echo Starting Meeting Minutes Backend with visible output...
echo Model: %MODEL_NAME%
echo Port: %PORT%
echo.

REM Run the PowerShell script
powershell -ExecutionPolicy Bypass -File start_with_output.ps1 %MODEL_NAME% %PORT%

goto :eof
