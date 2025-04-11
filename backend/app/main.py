from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel
import uvicorn
from typing import Optional
import logging
import os
from dotenv import load_dotenv
from db import DatabaseManager
import json
from threading import Lock
from transcript_processor import TranscriptProcessor

# Load environment variables
load_dotenv()

# Configure logger with line numbers and function names
logger = logging.getLogger(__name__)
logger.setLevel(logging.DEBUG)

# Create console handler with formatting
console_handler = logging.StreamHandler()
console_handler.setLevel(logging.DEBUG)

# Create formatter with line numbers and function names
formatter = logging.Formatter(
    '%(asctime)s - %(levelname)s - [%(filename)s:%(lineno)d - %(funcName)s()] - %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
console_handler.setFormatter(formatter)

# Add handler to logger if not already added
if not logger.handlers:
    logger.addHandler(console_handler)

app = FastAPI(
    title="Meeting Summarizer API",
    description="API for processing and summarizing meeting transcripts",
    version="1.0.0"
)

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],     # Allow all origins for testing
    allow_credentials=True,
    allow_methods=["*"],     # Allow all methods
    allow_headers=["*"],     # Allow all headers
    max_age=3600,            # Cache preflight requests for 1 hour
)


class TranscriptRequest(BaseModel):
    """Request model for transcript text"""
    text: str
    model: str
    model_name: str
    chunk_size: Optional[int] = 5000
    overlap: Optional[int] = 1000

class SummaryProcessor:
    """Handles the processing of summaries in a thread-safe way"""
    def __init__(self):
        try:
            self.db = DatabaseManager()
            self._lock = Lock() # Lock might be unnecessary if process_summary is removed, but keep for now

            # Load API key and validate
            api_key = os.getenv('ANTHROPIC_API_KEY')
            if not api_key:
                logger.error("ANTHROPIC_API_KEY environment variable not set")
                raise ValueError("ANTHROPIC_API_KEY environment variable not set")

            logger.info("Initializing SummaryProcessor components")
            self.transcript_processor = TranscriptProcessor()
            logger.info("SummaryProcessor initialized successfully (core components)")
        except Exception as e:
            logger.error(f"Failed to initialize SummaryProcessor: {str(e)}", exc_info=True)
            raise

    # This method IS used by the background task of /process-transcript
    async def process_transcript(self, text: str, model: str, model_name: str, chunk_size: int = 5000, overlap: int = 1000) -> tuple:
        """Process a transcript text"""
        try:
            if not text:
                raise ValueError("Empty transcript text provided")

            # Validate chunk_size and overlap
            if chunk_size <= 0:
                raise ValueError("chunk_size must be positive")
            if overlap < 0:
                raise ValueError("overlap must be non-negative")
            if overlap >= chunk_size:
                overlap = chunk_size - 1  # Ensure overlap is less than chunk_size

            # Ensure step size is positive
            step_size = chunk_size - overlap
            if step_size <= 0:
                chunk_size = overlap + 1  # Adjust chunk_size to ensure positive step

            logger.info(f"Processing transcript of length {len(text)} with chunk_size={chunk_size}, overlap={overlap}")
            # Pass text as positional arg, chunk_size and overlap as keyword args
            num_chunks, all_json_data = await self.transcript_processor.process_transcript(
                text=text,  # Pass as keyword arg to be explicit
                model=model,
                model_name=model_name,
                chunk_size=chunk_size,
                overlap=overlap
            )
            logger.info(f"Successfully processed transcript into {num_chunks} chunks")

            return num_chunks, all_json_data
        except Exception as e:
            logger.error(f"Error processing transcript: {str(e)}", exc_info=True)
            raise

    def cleanup(self):
        """Cleanup resources"""
        try:
            logger.info("Cleaning up resources")
            if hasattr(self, 'transcript_processor'): # Check if initialized
                self.transcript_processor.cleanup()
            logger.info("Cleanup completed successfully")
        except Exception as e:
            logger.error(f"Error during cleanup: {str(e)}", exc_info=True)

# Initialize processor
processor = SummaryProcessor()

# This function IS used by /process-transcript
async def process_transcript_background(process_id: str, transcript: TranscriptRequest):
    """Background task to process transcript"""
    try:
        logger.info(f"Starting background processing for process_id: {process_id}")

        # Process transcript using the existing processor method
        # Note: This uses processor.transcript_processor internally
        num_chunks, all_json_data = await processor.process_transcript(
            text=transcript.text,
            model=transcript.model,
            model_name=transcript.model_name,
            chunk_size=transcript.chunk_size,
            overlap=transcript.overlap
        )

        # Create final summary structure by aggregating chunk results
        final_summary = {
            "MeetingName": "",
            "SectionSummary": {
                "title": "Section Summary",
                "blocks": []
            },
            "CriticalDeadlines": {
                "title": "Critical Deadlines",
                "blocks": []
            },
            "KeyItemsDecisions": {
                "title": "Key Items & Decisions",
                "blocks": []
            },
            "ImmediateActionItems": {
                "title": "Immediate Action Items",
                "blocks": []
            },
            "NextSteps": {
                "title": "Next Steps",
                "blocks": []
            },
            "OtherImportantPoints": {
                "title": "Other Important Points",
                "blocks": []
            },
            "ClosingRemarks": {
                "title": "Closing Remarks",
                "blocks": []
            }
        }

        # Process each chunk's data
        for json_str in all_json_data:
            try:
                json_dict = json.loads(json_str)
                # Safely update MeetingName
                if "MeetingName" in json_dict and json_dict["MeetingName"]:
                     final_summary["MeetingName"] = json_dict["MeetingName"]
                # Safely extend blocks
                for key in final_summary:
                    if key != "MeetingName" and key in json_dict and isinstance(json_dict[key], dict) and "blocks" in json_dict[key]:
                         if isinstance(json_dict[key]["blocks"], list):
                              final_summary[key]["blocks"].extend(json_dict[key]["blocks"])
            except json.JSONDecodeError as e:
                logger.error(f"Failed to parse JSON chunk for {process_id}: {e}. Chunk: {json_str[:100]}...")
            except Exception as e:
                 logger.error(f"Error processing chunk data for {process_id}: {e}. Chunk: {json_str[:100]}...")


        # Update database with meeting name
        if final_summary["MeetingName"]:
            await processor.db.update_meeting_name(process_id, final_summary["MeetingName"])

        # Save final result
        await processor.db.update_process(process_id, status="completed", result=json.dumps(final_summary))
        logger.info(f"Background processing completed for process_id: {process_id}")

    except Exception as e:
        error_msg = str(e)
        logger.error(f"Error in background processing for {process_id}: {error_msg}", exc_info=True)
        # Ensure DB update happens even if transcript processing fails partially
        try:
            await processor.db.update_process(process_id, status="failed", error=error_msg)
        except Exception as db_e:
            logger.error(f"Failed to update DB status to failed for {process_id}: {db_e}", exc_info=True)


@app.post("/process-transcript")
async def process_transcript_api(
    transcript: TranscriptRequest,
    background_tasks: BackgroundTasks
):
    """Process a transcript text with background processing"""
    try:
        # Create new process
        process_id = await processor.db.create_process()

        # Save transcript data
        await processor.db.save_transcript(
            process_id,
            transcript.text,
            transcript.model,
            transcript.model_name,
            transcript.chunk_size,
            transcript.overlap
        )

        # Start background processing
        background_tasks.add_task(
            process_transcript_background,
            process_id,
            transcript
        )

        return JSONResponse({
            "message": "Processing started",
            "process_id": process_id
        })

    except Exception as e:
        logger.error(f"Error in process_transcript_api: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/get-summary/{process_id}")
async def get_summary(process_id: str):
    """Get the summary for a given process ID"""
    try:
        result = await processor.db.get_transcript_data(process_id)
        if not result:
            return JSONResponse(
                status_code=404,
                content={
                    "status": "error",
                    "meetingName": None,
                    "process_id": process_id,
                    "data": None,
                    "start": None,
                    "end": None,
                    "error": "Process ID not found"
                }
            )

        status = result.get("status", "unknown").lower() # Default to unknown and ensure lowercase

        # Parse result data if available
        summary_data = None
        if result.get("result"):
            try:
                # The result is already a JSON string, so we need to parse it
                parsed_result = json.loads(result["result"])
                # Check if it was double-encoded
                if isinstance(parsed_result, str):
                     summary_data = json.loads(parsed_result)
                else:
                     summary_data = parsed_result

                # Basic validation if it's a dictionary as expected
                if not isinstance(summary_data, dict):
                    logger.error(f"Parsed summary data is not a dictionary for process {process_id}")
                    summary_data = None # Reset if format is wrong

            except json.JSONDecodeError as e:
                logger.error(f"Failed to parse JSON data for process {process_id}: {str(e)}")
                status = "failed" # Mark as failed if result JSON is invalid
                result["error"] = f"Invalid summary data format: {str(e)}"
            except Exception as e:
                 logger.error(f"Unexpected error parsing summary data for {process_id}: {str(e)}")
                 status = "failed"
                 result["error"] = f"Error processing summary data: {str(e)}"


        # Build response
        response = {
            "status": "processing" if status in ["processing", "pending", "started"] else status, # Treat 'started' as processing
            "meetingName": summary_data.get("MeetingName") if isinstance(summary_data, dict) else None,
            "process_id": process_id,
            "start": result.get("start_time"),
            "end": result.get("end_time"),
            "data": summary_data if status == "completed" else None # Only return data if completed successfully
        }

        if status == "failed":
            response["status"] = "error"
            response["error"] = result.get("error", "Unknown processing error")
            # Ensure data is None on error
            response["data"] = None
            response["meetingName"] = None # Clear meeting name on error too
            return JSONResponse(status_code=400, content=response)

        elif status in ["processing", "pending", "started"]:
             # Ensure data is None while processing
            response["data"] = None
            return JSONResponse(status_code=202, content=response) # 202 Accepted indicates processing

        elif status == "completed":
            if not summary_data: # Check if summary_data is valid after parsing
                response["status"] = "error"
                response["error"] = "Completed but summary data is missing or invalid"
                response["data"] = None
                response["meetingName"] = None
                return JSONResponse(status_code=500, content=response) # Internal error if completed but no data
            return JSONResponse(status_code=200, content=response)

        else:
            # Handle any other unexpected status
            response["status"] = "error"
            response["error"] = f"Unknown or unexpected status: {status}"
            response["data"] = None
            response["meetingName"] = None
            return JSONResponse(status_code=500, content=response) # Internal error for unknown status

    except Exception as e:
        logger.error(f"Error getting summary for {process_id}: {str(e)}", exc_info=True)
        # Generic error response
        return JSONResponse(
            status_code=500,
            content={
                "status": "error",
                "meetingName": None,
                "process_id": process_id,
                "data": None,
                "start": None,
                "end": None,
                "error": f"Internal server error: {str(e)}"
            }
        )

@app.on_event("shutdown")
async def shutdown_event():
    """Cleanup on API shutdown"""
    logger.info("API shutting down, cleaning up resources")
    try:
        # Call the cleanup method on the processor instance
        processor.cleanup() # Corrected: call method on instance
        logger.info("Successfully cleaned up resources")
    except Exception as e:
        logger.error(f"Error during cleanup: {str(e)}", exc_info=True)

if __name__ == "__main__":
    import multiprocessing
    multiprocessing.freeze_support() # Good practice for potential freezing issues
    uvicorn.run(app, host="0.0.0.0", port=5167)
