#!/bin/bash

# Exit on any error
set -e
# Exit on pipe error
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Model name
MODEL_NAME="qwen2.5:3b"

echo -e "${GREEN}Installing Meeting Minutes Assistant Backend...${NC}"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if a Homebrew package is installed
brew_package_installed() {
    local package=$1
    echo "Checking package: $package"
    
    if [[ "$package" == "blackhole-2ch" ]]; then
        if brew list --cask | grep -w "$package" > /dev/null 2>&1; then
            echo "$package is installed (cask)"
            return 0
        fi
    else
        if brew list --formula | grep -w "$package" > /dev/null 2>&1; then
            echo "$package is installed (formula)"
            return 0
        fi
    fi
    
    echo "$package is NOT installed"
    return 1
}

# Function to handle errors
handle_error() {
    echo -e "${RED}Error: $1${NC}"
    exit 1
}

# Function to check if model is available
check_model() {
    local model=$1
    # Try different variations of the model name
    if ollama list 2>/dev/null | grep -iE "^$model|/$model" >/dev/null; then
        return 0
    fi
    return 1
}

# Check if Homebrew is installed
if ! command -v brew >/dev/null 2>&1; then
    echo -e "${YELLOW}Installing Homebrew...${NC}"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || handle_error "Failed to install Homebrew"
else
    echo -e "${BLUE}Homebrew is already installed${NC}"
fi

# Install Homebrew packages
BREW_PACKAGES="portaudio blackhole-2ch ffmpeg switchaudio-osx"
for pkg in $BREW_PACKAGES; do
    if ! brew_package_installed $pkg; then
        echo -e "${YELLOW}Installing $pkg...${NC}"
        if [[ "$pkg" == "blackhole-2ch" ]]; then
            brew install --cask $pkg || handle_error "Failed to install $pkg"
            
            echo -e "${YELLOW}Setting up audio routing...${NC}"
            # Create Multi-Output Device
            osascript <<EOT
tell application "Audio MIDI Setup"
    try
        -- Check if device already exists
        set deviceExists to false
        repeat with d in (get name of every aggregate device)
            if d is equal to "Meeting Audio" then
                set deviceExists to true
                exit repeat
            end if
        end repeat
        
        if deviceExists is false then
            -- Create new aggregate device
            make new aggregate device with properties {name:"Meeting Audio", uid:"com.meeting.audio.device"}
        end if
        
        -- Get the Meeting Audio device
        set meetingDevice to (first aggregate device whose name is "Meeting Audio")
        
        -- Get Built-in Output device
        set builtInDevice to (first audio device whose name contains "Built-in Output")
        
        -- Get BlackHole device
        set blackholeDevice to (first audio device whose name contains "BlackHole")
        
        -- Configure master device
        set master device of meetingDevice to builtInDevice
        
        -- Configure subdevices
        set subdevices of meetingDevice to {builtInDevice, blackholeDevice}
        
        -- Enable drift correction
        set drift compensation enabled of meetingDevice to true
        
        log "Successfully configured Meeting Audio device"
    on error errMsg
        log "Error configuring Meeting Audio device: " & errMsg
    end try
end tell
EOT
            
            # Wait for device to be ready
            sleep 2
            
            # Set as default output device
            if command -v SwitchAudioSource >/dev/null 2>&1; then
                echo -e "${YELLOW}Setting Meeting Audio as default output...${NC}"
                SwitchAudioSource -s "Meeting Audio" || echo "Could not switch to Meeting Audio device"
            fi
            
            echo -e "${GREEN}Audio routing setup complete!${NC}"
            echo -e "${BLUE}Important: For Slack, Google Meet, etc.:${NC}"
            echo -e "${BLUE}1. Open System Settings > Sound${NC}"
            echo -e "${BLUE}2. Select 'Meeting Audio' as the output device${NC}"
            echo -e "${BLUE}3. In your meeting app settings, select 'Meeting Audio' as the output device${NC}"
        else
            brew install $pkg || handle_error "Failed to install $pkg"
        fi
        echo -e "${GREEN}$pkg installed successfully!${NC}"
    else
        echo -e "${BLUE}$pkg is already installed${NC}"
    fi
done

# Check and install Ollama
if ! command_exists ollama; then
    echo -e "${YELLOW}Installing Ollama...${NC}"
    brew install ollama || handle_error "Failed to install Ollama"
else
    echo -e "${BLUE}Ollama is already installed${NC}"
fi

# Check if Ollama service is running
if ! pgrep -x "ollama" >/dev/null; then
    echo -e "${YELLOW}Starting Ollama service...${NC}"
    ollama serve &
    sleep 5  # Wait for Ollama to start
    if ! pgrep -x "ollama" >/dev/null; then
        handle_error "Failed to start Ollama service"
    fi
else
    echo -e "${BLUE}Ollama service is already running${NC}"
fi

# Check if Qwen model is already pulled
if ! check_model "$MODEL_NAME"; then
    echo -e "${YELLOW}Pulling Qwen model...${NC}"
    if ! ollama pull "$MODEL_NAME"; then
        handle_error "Failed to pull Qwen model"
    fi
    # Verify model was pulled successfully
    if ! check_model "$MODEL_NAME"; then
        handle_error "Model installation verification failed"
    fi
else
    echo -e "${BLUE}Qwen model is already installed${NC}"
fi

# Check if virtual environment exists
if [ ! -d "venv" ]; then
    echo -e "${YELLOW}Creating Python virtual environment...${NC}"
    python3 -m venv venv || handle_error "Failed to create virtual environment"
else
    echo -e "${BLUE}Virtual environment already exists${NC}"
fi

# Activate virtual environment
echo -e "${YELLOW}Activating virtual environment...${NC}"
source venv/bin/activate || handle_error "Failed to activate virtual environment"

# Install/Update certificates
echo -e "${YELLOW}Installing SSL certificates...${NC}"
python3 -m pip install --upgrade certifi || handle_error "Failed to install SSL certificates"

# Check if pip needs upgrading
if ! PIP_VERSION=$(pip -V | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); then
    handle_error "Failed to get pip version"
fi

if ! LATEST_PIP_VERSION=$(curl -s https://pypi.org/pypi/pip/json | grep -o '"version":"[^"]*"' | cut -d'"' -f4); then
    handle_error "Failed to get latest pip version"
fi

if [ "$PIP_VERSION" != "$LATEST_PIP_VERSION" ]; then
    echo -e "${YELLOW}Upgrading pip...${NC}"
    pip install --upgrade pip || handle_error "Failed to upgrade pip"
else
    echo -e "${BLUE}pip is already up to date${NC}"
fi

# Install Python dependencies
echo -e "${YELLOW}Installing Python dependencies...${NC}"
if ! PYTHONWARNINGS="ignore::DeprecationWarning" pip install -r requirements.txt; then
    handle_error "Failed to install Python dependencies"
fi

# Check and create .env file
if [ ! -f .env ]; then
    echo -e "${YELLOW}Creating .env file...${NC}"
    if [ ! -f .env.example ]; then
        handle_error ".env.example file not found"
    fi
    cp .env.example .env || handle_error "Failed to create .env file"
else
    echo -e "${BLUE}.env file already exists${NC}"
fi

# Check and create recordings directory
if [ ! -d "recordings" ]; then
    echo -e "${YELLOW}Creating recordings directory...${NC}"
    mkdir -p recordings || handle_error "Failed to create recordings directory"
else
    echo -e "${BLUE}Recordings directory already exists${NC}"
fi

echo -e "${GREEN}Installation complete!${NC}"
echo -e "\nTo start the application:"
echo -e "1. If Ollama is not running, start it with:"
echo -e "   ${YELLOW}ollama serve${NC}"
echo -e "\n2. In a new terminal, activate the virtual environment:"
echo -e "   ${YELLOW}source venv/bin/activate${NC}"
echo -e "\n3. Start the FastAPI server:"
echo -e "   ${YELLOW}uvicorn app.main:app --reload --host 0.0.0.0 --port 8000${NC}"

# Check if everything is installed correctly
echo -e "\n${YELLOW}Checking installation...${NC}"
INSTALL_OK=true

# Check system dependencies
for cmd in ffmpeg ollama python3 pip; do
    if ! command_exists $cmd; then
        echo -e "${RED}Error: $cmd is not installed correctly${NC}"
        INSTALL_OK=false
    fi
done

# Check Homebrew packages
echo -e "\nVerifying installed packages..."
for pkg in $BREW_PACKAGES; do
    echo -e "\nChecking $pkg..."
    if ! brew_package_installed $pkg; then
        echo -e "${RED}Error: $pkg is not installed correctly${NC}"
        INSTALL_OK=false
    else
        echo -e "${GREEN}$pkg is installed correctly${NC}"
    fi
done

# Check if virtual environment is activated
if [[ "$VIRTUAL_ENV" != *"venv"* ]]; then
    echo -e "${RED}Warning: Virtual environment is not activated${NC}"
    INSTALL_OK=false
fi

# Check SSL certificates
if ! python3 -c "import ssl; ssl.create_default_context()" 2>/dev/null; then
    echo -e "${RED}Warning: SSL certificates might not be properly configured${NC}"
    INSTALL_OK=false
fi

# Check if Ollama is running
if ! pgrep -x "ollama" >/dev/null; then
    echo -e "${RED}Warning: Ollama service is not running${NC}"
    INSTALL_OK=false
fi

# Check if Qwen model is available
if ! check_model "$MODEL_NAME"; then
    echo -e "${RED}Warning: Qwen model ($MODEL_NAME) is not installed${NC}"
    INSTALL_OK=false
fi

# Final status
if [ "$INSTALL_OK" = false ]; then
    echo -e "${RED}Some components may not be installed correctly. Please check the errors above.${NC}"
    exit 1
else
    echo -e "${GREEN}All components are installed correctly!${NC}"
fi
