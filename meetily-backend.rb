class MeetilyBackend < Formula
  desc "FastAPI backend for Meetily meeting transcription and analysis"
  homepage "https://github.com/Zackriya-Solutions/meeting-minutes"
  url "https://github.com/Zackriya-Solutions/meeting-minutes/archive/refs/heads/main.zip"
  version "0.1.0"
  sha256 "9f9f2864577d1d07682caf1f0fcff5b5d46254d587e722845aab83ce05bcc959" # Update with actual SHA256

  depends_on "cmake" => :build
  depends_on "llvm" => :build
  depends_on "libomp" => :build
  depends_on "python@3.9"
  depends_on "ffmpeg"
  depends_on "git"

  def install
    # Create necessary directories
    mkdir_p "#{prefix}/backend"
    mkdir_p "#{prefix}/backend/app"
    mkdir_p "#{prefix}/backend/whisper-server-package/models"
    mkdir_p "#{prefix}/backend/transcripts"
    mkdir_p "#{prefix}/backend/chroma"

    # Copy backend files
    cp_r "backend/app", "#{prefix}/backend/"
    cp_r "backend/whisper-custom", "#{prefix}/backend/"
    cp "backend/requirements.txt", "#{prefix}/backend/"
    cp "backend/.env", "#{prefix}/backend/" if File.exist?("backend/.env")
    cp "backend/download-ggml-model.sh", "#{prefix}/backend/"
    
    # Create virtual environment and install dependencies
    system "python3", "-m", "venv", "#{prefix}/backend/venv"
    system "#{prefix}/backend/venv/bin/pip", "install", "--upgrade", "pip"
    system "#{prefix}/backend/venv/bin/pip", "install", "-r", "#{prefix}/backend/requirements.txt"

    # Build whisper.cpp from source
    # Use a specific version tag to ensure compatibility with our custom code
    mkdir_p "#{buildpath}/whisper.cpp"
    system "git", "clone", "--branch", "master", "--depth", "1", "https://github.com/Zackriya-Solutions/whisper.cpp.git", "#{buildpath}/whisper.cpp"
    
    # Copy custom server files - make sure we're copying all necessary files
    cp_r Dir["backend/whisper-custom/server/*"], "#{buildpath}/whisper.cpp/examples/server/"
    
    # Create necessary directories for the package
    mkdir_p "#{prefix}/backend/whisper-server-package"
    mkdir_p "#{prefix}/backend/whisper-server-package/models"
    
    # Build with CMake - following the approach in build_whisper.sh
    cd "#{buildpath}/whisper.cpp" do
      # Clean any previous build artifacts
      rm_rf "build"
      mkdir_p "build"
      
      cd "build" do
        # Build with static linking to avoid dependency issues
        system "cmake", "-DCMAKE_C_FLAGS=-w", "-DCMAKE_CXX_FLAGS=-w", "-DBUILD_SHARED_LIBS=OFF", ".."
        system "make", "-j4"
      end
      
      # Check if server binary was built successfully
      if File.exist?("build/bin/whisper-server")
        # Copy server binary to package directory
        cp "build/bin/whisper-server", "#{prefix}/backend/whisper-server-package/"
        
        # Also copy the libwhisper library if it exists
        if File.exist?("build/lib/libwhisper.1.dylib")
          cp "build/lib/libwhisper.1.dylib", "#{prefix}/backend/whisper-server-package/"
          # Fix the RPATH in the binary
          system "install_name_tool", "-add_rpath", "@executable_path", "#{prefix}/backend/whisper-server-package/whisper-server"
        end
        
        ohai "Whisper server binary copied successfully"
      else
        # If the build fails, try using the pre-compiled binary if available
        ohai "Whisper server build failed, checking for pre-compiled binary"
        if File.exist?("../backend/whisper-server-package/whisper-server")
          cp "../backend/whisper-server-package/whisper-server", "#{prefix}/backend/whisper-server-package/"
          # Also copy the libwhisper library if it exists
          if File.exist?("../backend/whisper-server-package/libwhisper.1.dylib")
            cp "../backend/whisper-server-package/libwhisper.1.dylib", "#{prefix}/backend/whisper-server-package/"
            # Fix the RPATH in the binary
            system "install_name_tool", "-add_rpath", "@executable_path", "#{prefix}/backend/whisper-server-package/whisper-server"
          end
          ohai "Using pre-compiled whisper-server binary"
        else
          opoo "No pre-compiled whisper-server binary found. The formula installation will continue, but the server may not work."
        end
      end
      
      # Create run script exactly as in build_whisper.sh
      (prefix/"backend/whisper-server-package/run-server.sh").write <<~EOS
        #!/bin/bash

        # Default configuration
        HOST="127.0.0.1"
        PORT="8178"
        MODEL="models/ggml-small.bin"

        # Parse command line arguments
        while [[ $# -gt 0 ]]; do
            case $1 in
                --host)
                    HOST="$2"
                    shift 2
                    ;;
                --port)
                    PORT="$2"
                    shift 2
                    ;;
                --model)
                    MODEL="$2"
                    shift 2
                    ;;
                *)
                    echo "Unknown option: $1"
                    exit 1
                    ;;
            esac
        done

        # Run the server
        ./whisper-server \\
            --model "$MODEL" \\
            --host "$HOST" \\
            --port "$PORT" \\
            --diarize \\
            --print-progress
      EOS
      
      # Make the run script executable
      chmod 0755, "#{prefix}/backend/whisper-server-package/run-server.sh"
    end

    # Create run scripts - update names to include backend
    (bin/"meetily-download-model").write <<~EOS
      #!/bin/bash
      cd #{prefix}/backend
      MODEL_SHORT_NAME=$1
      if [ -z "$MODEL_SHORT_NAME" ]; then
        echo "Please specify a model name (e.g. small, medium, base)"
        exit 1
      fi
      
      # Ensure the models directory exists
      mkdir -p whisper-server-package/models
      
      echo "Downloading ggml model $MODEL_SHORT_NAME from 'https://huggingface.co/ggerganov/whisper.cpp' ..."
      ./download-ggml-model.sh $MODEL_SHORT_NAME
      
      if [ -f "ggml-$MODEL_SHORT_NAME.bin" ]; then
        # Move the downloaded model to the models directory
        mv "ggml-$MODEL_SHORT_NAME.bin" "whisper-server-package/models/"
        echo "Done! Model '$MODEL_SHORT_NAME' saved in 'whisper-server-package/models/ggml-$MODEL_SHORT_NAME.bin'"
        echo "You can now start the Meetily backend server with: meetily-server"
      else
        echo "Failed to download model. Please try again later."
        exit 1
      fi
    EOS

    (bin/"meetily-server").write <<~EOS
      #!/bin/bash

      # Color codes
      GREEN='\\033[0;32m'
      BLUE='\\033[0;34m'
      YELLOW='\\033[1;33m'
      RED='\\033[0;31m'
      NC='\\033[0m' # No Color

      # Set paths
      BACKEND_DIR="#{prefix}/backend"
      WHISPER_DIR="$BACKEND_DIR/whisper-server-package"
      MODEL_DIR="$WHISPER_DIR/models"

      # Create models directory if it doesn't exist
      echo -e "[INFO] Creating models directory..."
      mkdir -p "$MODEL_DIR"

      # Check for Whisper models
      echo -e "[INFO] Checking for Whisper models..."
      MODEL_COUNT=$(find "$MODEL_DIR" -name "ggml-*.bin" | wc -l)
      
      if [ "$MODEL_COUNT" -eq 0 ]; then
        echo -e "[WARNING] No Whisper model found. Downloading the small model..."
        cd "$BACKEND_DIR"
        ./download-ggml-model.sh small
        
        # Move the model to the models directory
        if [ -f "$BACKEND_DIR/ggml-small.bin" ]; then
          mv "$BACKEND_DIR/ggml-small.bin" "$MODEL_DIR/"
          echo -e "Done! Model 'small' saved in '$MODEL_DIR/ggml-small.bin'"
          echo -e "You can now start the Meetily backend server with: meetily-server"
        else
          echo -e "[ERROR] Failed to download model. Please run meetily-download-model manually."
          exit 1
        fi
      fi

      # Find the first model
      MODEL=$(find "$MODEL_DIR" -name "ggml-*.bin" | head -n 1)
      if [ -z "$MODEL" ]; then
        echo -e "[ERROR] No model found. Please run meetily-download-model first."
        exit 1
      fi
      
      MODEL_BASENAME=$(basename "$MODEL")
      echo -e "[SUCCESS] Using model: whisper-server-package/models/$MODEL_BASENAME"

      # Start Whisper server
      echo -e "[INFO] Starting Whisper server..."
      cd "$WHISPER_DIR"
      
      # Check if libwhisper.1.dylib exists in the same directory
      if [ ! -f "$WHISPER_DIR/libwhisper.1.dylib" ] && [ -f "$BACKEND_DIR/whisper-custom/libwhisper.1.dylib" ]; then
        echo -e "[INFO] Copying libwhisper.1.dylib from whisper-custom directory..."
        cp "$BACKEND_DIR/whisper-custom/libwhisper.1.dylib" "$WHISPER_DIR/"
      fi
      
      # Try to fix RPATH if needed
      if [ -f "$WHISPER_DIR/whisper-server" ] && [ -f "$WHISPER_DIR/libwhisper.1.dylib" ]; then
        echo -e "[INFO] Fixing RPATH in whisper-server binary..."
        install_name_tool -add_rpath "@executable_path" "$WHISPER_DIR/whisper-server" 2>/dev/null || true
      fi
      
      # Start the server with error handling
      ./run-server.sh --model "$MODEL" --host "127.0.0.1" --port "8178" &
      WHISPER_PID=$!
      
      # Wait a moment to see if the server starts successfully
      sleep 2
      
      # Check if the process is still running
      if ! kill -0 $WHISPER_PID 2>/dev/null; then
        echo -e "[ERROR] Whisper server failed to start. Please check the logs."
        echo -e "[INFO] Checking whisper-server binary..."
        file "$WHISPER_DIR/whisper-server"
        echo -e "[INFO] You can try running it manually with:"
        echo -e "cd $WHISPER_DIR && ./run-server.sh --model $MODEL"
        
        # Try to build a static version as a fallback
        echo -e "[INFO] Attempting to build a static version of whisper-server..."
        cd "$BACKEND_DIR"
        mkdir -p whisper-static-build && cd whisper-static-build
        
        # Clone the repo
        git clone --depth 1 https://github.com/Zackriya-Solutions/whisper.cpp.git .
        
        # Copy custom server files
        cp -r "$BACKEND_DIR/whisper-custom/server/"* examples/server/
        
        # Build with static linking
        mkdir -p build && cd build
        cmake -DCMAKE_C_FLAGS="-w" -DCMAKE_CXX_FLAGS="-w" -DBUILD_SHARED_LIBS=OFF ..
        make -j4
        
        # Check if build succeeded
        if [ -f "bin/whisper-server" ]; then
          echo -e "[SUCCESS] Static build successful, replacing whisper-server..."
          cp bin/whisper-server "$WHISPER_DIR/"
          chmod +x "$WHISPER_DIR/whisper-server"
          
          # Try starting the server again
          cd "$WHISPER_DIR"
          ./run-server.sh --model "$MODEL" --host "127.0.0.1" --port "8178" &
          WHISPER_PID=$!
          sleep 2
        else
          echo -e "[ERROR] Static build failed. Please report this issue."
        fi
      fi
      
      # Start Python backend
      echo -e "[INFO] Starting Python backend..."
      cd "$BACKEND_DIR"
      source venv/bin/activate
      
      # Fix the Python import paths
      echo -e "[INFO] Fixing Python import paths..."
      
      # Fix db import
      if grep -q "from db import" "$BACKEND_DIR/app/main.py"; then
        sed -i.bak 's/from db import/from app.db import/g' "$BACKEND_DIR/app/main.py"
        echo -e "[SUCCESS] Fixed db import path"
      fi
      
      # Fix Process_transcrip import
      if grep -q "from Process_transcrip import" "$BACKEND_DIR/app/main.py"; then
        sed -i.bak 's/from Process_transcrip import/from app.Process_transcrip import/g' "$BACKEND_DIR/app/main.py"
        echo -e "[SUCCESS] Fixed Process_transcrip import path"
      fi
      
      # Run the Python backend
      python -m uvicorn app.main:app --host 0.0.0.0 --port 5167 &
      PYTHON_PID=$!

      # Print success message
      echo -e "${GREEN}Meetily backend started!${NC}"
      echo -e "${BLUE}Whisper Server running on http://localhost:8178${NC}"
      echo -e "${BLUE}FastAPI Backend running on http://localhost:5167${NC}"
      echo -e "${GREEN}API Documentation available at http://localhost:5167/docs${NC}"
      echo -e "${BLUE}Press Ctrl+C to stop all services${NC}"
      
      # Wait for Ctrl+C
      trap "kill $WHISPER_PID $PYTHON_PID 2>/dev/null; exit" INT TERM
      wait
    EOS

    chmod 0755, bin/"meetily-download-model"
    chmod 0755, bin/"meetily-server"
    
    # Ask for API keys
    ohai "Meetily Backend can use Anthropic Claude or Groq for enhanced meeting analysis."
    
    print "Would you like to configure an Anthropic API key? (y/n): "
    if $stdin.gets.chomp.downcase == "y"
      print "Enter your Anthropic API key: "
      anthropic_key = $stdin.gets.chomp
      if !anthropic_key.empty?
        system "echo \"ANTHROPIC_API_KEY=#{anthropic_key}\" > #{prefix}/backend/.env"
        ohai "Anthropic API key configured successfully!"
      end
    end
    
    print "Would you like to configure a Groq API key? (y/n): "
    if $stdin.gets.chomp.downcase == "y"
      print "Enter your Groq API key: "
      groq_key = $stdin.gets.chomp
      if !groq_key.empty?
        if File.exist?("#{prefix}/backend/.env")
          system "echo \"GROQ_API_KEY=#{groq_key}\" >> #{prefix}/backend/.env"
        else
          system "echo \"GROQ_API_KEY=#{groq_key}\" > #{prefix}/backend/.env"
        end
        ohai "Groq API key configured successfully!"
      end
    end
    
    ohai "Meetily Backend installation complete! Run 'meetily-download-model medium' to download a model, then 'meetily-server' to start the server."
  end

  def caveats
    <<~EOS
      Meetily Backend has been installed!
      
      To download a Whisper model:
        meetily-download-model [model_name]
      
      Available models: tiny, base, small, medium, large-v3, etc.
      
      To start the Meetily Backend server:
        meetily-server
      
      The server will be available at:
        - Whisper Server: http://localhost:8178
        - FastAPI Backend: http://localhost:5167
        - API Documentation: http://localhost:5167/docs
      
      If you want to update your Claude or Groq API keys for meeting analysis:
        echo "ANTHROPIC_API_KEY=your_key_here" > #{prefix}/backend/.env
        echo "GROQ_API_KEY=your_key_here" >> #{prefix}/backend/.env
      
      Ollama should be running for local LLM support:
        brew install ollama
        ollama pull mistral
        
      For the complete Meetily experience, install the frontend application:
        brew install --cask meetily
    EOS
  end

  test do
    system "#{bin}/meetily-download-model", "--help"
    # Add more tests as needed
  end
end 