import requests
import time
import argparse
import json
import sys

# --- Configuration ---
DEFAULT_BASE_URL = "http://localhost:5167"
DEFAULT_MODEL_PROVIDER = "claude"  # Or 'ollama', 'claude', 'groq'
DEFAULT_MODEL_NAME = "claude-3-5-sonnet-20241022" # Adjust if needed
DEFAULT_CHUNK_SIZE = 40000
DEFAULT_OVERLAP = 1000
DEFAULT_POLL_INTERVAL_SECONDS = 5  # How often to check the status
DEFAULT_MAX_POLL_ATTEMPTS = 24     # Max times to poll (e.g., 24 * 5s = 120s timeout)

# --- API Interaction Functions ---

def process_transcript(base_url, transcript_text, provider, model_name, chunk_size, overlap):
    """Sends the transcript to the processing endpoint."""
    url = f"{base_url}/process-transcript"
    payload = {
        "text": transcript_text,
        "model": provider,
        "model_name": model_name,
        "chunk_size": chunk_size,
        "overlap": overlap
    }
    headers = {'Content-Type': 'application/json'}
    print(f"Sending POST request to {url} with model '{provider}/{model_name}'...")

    try:
        response = requests.post(url, headers=headers, json=payload, timeout=30) # 30s timeout for initial request
        response.raise_for_status() # Raise an exception for bad status codes (4xx or 5xx)

        response_data = response.json()
        if "process_id" in response_data:
            print(f"Successfully initiated processing. Process ID: {response_data['process_id']}")
            return response_data['process_id']
        else:
            print(f"Error: 'process_id' not found in response: {response_data}")
            return None

    except requests.exceptions.RequestException as e:
        print(f"Error during transcript processing request: {e}")
        return None
    except json.JSONDecodeError:
        print(f"Error: Could not decode JSON response from {url}. Response text: {response.text}")
        return None

def poll_summary_status(base_url, process_id, interval, max_attempts):
    """Polls the summary status endpoint until completion or error."""
    url = f"{base_url}/get-summary/{process_id}"
    print(f"Polling status endpoint: {url} (every {interval}s)")

    for attempt in range(max_attempts):
        print(f"Polling attempt {attempt + 1}/{max_attempts}...")
        try:
            response = requests.get(url, timeout=20) # 20s timeout for polling request
            response.raise_for_status()

            status_data = response.json()
            status = status_data.get("status", "unknown")
            error_message = status_data.get("error")
            summary_data = status_data.get("data")

            print(f"  Status: {status}")

            if status == "completed":
                print("Processing completed successfully!")
                return summary_data
            elif status == "error":
                print(f"Error reported by backend: {error_message or 'Unknown error'}")
                return None
            elif status in ["processing", "summarizing", "pending"]: # Add any other intermediate statuses your backend might use
                # Wait before the next poll
                time.sleep(interval)
            else:
                print(f"Warning: Received unknown status '{status}'. Continuing to poll.")
                time.sleep(interval)


        except requests.exceptions.Timeout:
            print(f"Warning: Polling request timed out. Retrying...")
            time.sleep(interval) # Wait before retrying after timeout
        except requests.exceptions.RequestException as e:
            print(f"Error during polling request: {e}. Stopping polling.")
            return None
        except json.JSONDecodeError:
            print(f"Error: Could not decode JSON response from {url}. Response text: {response.text}")
            print("Stopping polling.")
            return None

    print(f"Error: Reached maximum polling attempts ({max_attempts}) without completion.")
    return None

# --- Main Execution ---

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Test the transcript summarization API workflow.")
    parser.add_argument("transcript_file", help="Path to the .txt transcript file.")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL, help=f"Base URL of the API (default: {DEFAULT_BASE_URL})")
    parser.add_argument("--provider", default=DEFAULT_MODEL_PROVIDER, help=f"Model provider (default: {DEFAULT_MODEL_PROVIDER})")
    parser.add_argument("--model-name", default=DEFAULT_MODEL_NAME, help=f"Specific model name (default: {DEFAULT_MODEL_NAME})")
    parser.add_argument("--interval", type=int, default=DEFAULT_POLL_INTERVAL_SECONDS, help=f"Polling interval in seconds (default: {DEFAULT_POLL_INTERVAL_SECONDS})")
    parser.add_argument("--attempts", type=int, default=DEFAULT_MAX_POLL_ATTEMPTS, help=f"Maximum polling attempts (default: {DEFAULT_MAX_POLL_ATTEMPTS})")
    parser.add_argument("--chunk-size", type=int, default=DEFAULT_CHUNK_SIZE, help=f"Chunk size for processing (default: {DEFAULT_CHUNK_SIZE})")
    parser.add_argument("--overlap", type=int, default=DEFAULT_OVERLAP, help=f"Overlap size for processing (default: {DEFAULT_OVERLAP})")


    args = parser.parse_args()

    # 1. Read transcript file
    try:
        with open(args.transcript_file, 'r', encoding='utf-8') as f:
            transcript_content = f.read()
        print(f"Successfully read transcript file: {args.transcript_file}")
        if not transcript_content.strip():
             print("Error: Transcript file is empty.")
             sys.exit(1)
    except FileNotFoundError:
        print(f"Error: Transcript file not found at '{args.transcript_file}'")
        sys.exit(1)
    except Exception as e:
        print(f"Error reading transcript file: {e}")
        sys.exit(1)

    # 2. Process Transcript (POST request)
    process_id = process_transcript(
        args.base_url,
        transcript_content,
        args.provider,
        args.model_name,
        args.chunk_size,
        args.overlap
    )

    if not process_id:
        print("Failed to initiate transcript processing. Exiting.")
        sys.exit(1)

    # 3. Poll for Summary (GET requests)
    summary_result = poll_summary_status(
        args.base_url,
        process_id,
        args.interval,
        args.attempts
    )

    # 4. Display Result
    if summary_result:
        print("\\n--- Summary Received ---")
        # Pretty print the JSON result
        print(json.dumps(summary_result, indent=2))
        print("------------------------")
    else:
        print("\\nFailed to retrieve summary.")
        sys.exit(1)

    print("Script finished.")
