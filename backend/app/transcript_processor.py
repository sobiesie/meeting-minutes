from chromadb import Client as ChromaClient, Settings
from pydantic import BaseModel
from typing import List, Optional, Tuple
from pydantic_ai import Agent
from pydantic_ai.models.anthropic import AnthropicModel
from pydantic_ai.models.ollama import OllamaModel
from pydantic_ai.models.groq import GroqModel
import logging
import os
from dotenv import load_dotenv

# Set up logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(levelname)s - [%(filename)s:%(lineno)d] - %(message)s' # Simplified format slightly
)
logger = logging.getLogger(__name__)

load_dotenv()  # Load environment variables from .env file

class Block(BaseModel):
    """Represents a block of content in a section"""
    id: str
    type: str
    content: str
    color: str

class Section(BaseModel):
    """Represents a section in the meeting summary"""
    title: str
    blocks: List[Block]

class SummaryResponse(BaseModel):
    """Represents the meeting summary response based on a section of the transcript"""
    MeetingName : str
    SectionSummary : Section
    CriticalDeadlines: Section
    KeyItemsDecisions: Section
    ImmediateActionItems: Section
    NextSteps: Section
    OtherImportantPoints: Section
    ClosingRemarks: Section

# --- Main Class Used by main.py ---

class TranscriptProcessor:
    """Handles the processing of meeting transcripts using AI models."""
    def __init__(self):
        """Initialize the transcript processor."""
        self.collection_name = "all_transcripts" # Although not used for adding, keep for potential consistency
        self.chroma_client: Optional[ChromaClient] = None
        self.collection = None
        # Initialization moved to explicit method call as done in main.py
        # self.initialize_collection() # Avoid initializing here, let main.py control it

    def initialize_collection(self):
        """Initialize or get the ChromaDB collection."""
        try:
            # Ensure cleanup if re-initializing
            if self.chroma_client:
                self.cleanup() # Use cleanup method

            logger.info("Initializing ChromaDB client and collection...")
            # Create new client with settings
            settings = Settings(
                # allow_reset=True, # Potentially dangerous, remove unless specifically needed
                is_persistent=True,
                # Consider adding persist_directory=... if needed
                # persist_directory=".chromadb_persist"
            )
            self.chroma_client = ChromaClient(settings)

            # Get or create collection
            self.collection = self.chroma_client.get_or_create_collection(name=self.collection_name)
            logger.info(f"Using collection: {self.collection_name}")

            if not self.collection:
                raise RuntimeError("Failed to initialize ChromaDB collection")

        except Exception as e:
            logger.error(f"Error initializing ChromaDB: {e}", exc_info=True)
            raise

    def cleanup(self):
        """Cleanup ChromaDB resources."""
        logger.info("Cleaning up TranscriptProcessor resources...")
        if self.chroma_client:
            try:
                # Resetting the client effectively cleans up connections for non-persistent clients
                # For persistent clients, the data remains unless explicitly deleted.
                # No explicit 'close' method, rely on garbage collection or reset.
                self.chroma_client.reset() # Resets the entire client state, including collections
                logger.info("ChromaDB client reset.")
            except Exception as e:
                logger.error(f"Error during ChromaDB client reset: {e}", exc_info=True)
            finally:
                 self.collection = None
                 self.chroma_client = None
        else:
            logger.info("ChromaDB client was not initialized or already cleaned up.")


    async def process_transcript(self, text: str, model: str, model_name: str, chunk_size: int = 5000, overlap: int = 1000) -> Tuple[int, List[str]]:
        """
        Process transcript text into chunks and generate structured summaries for each chunk using an AI model.

        Args:
            text: The transcript text.
            model: The AI model provider ('claude', 'ollama', 'groq').
            model_name: The specific model name.
            chunk_size: The size of each text chunk.
            overlap: The overlap between consecutive chunks.

        Returns:
            A tuple containing:
            - The number of chunks processed.
            - A list of JSON strings, where each string is the summary of a chunk.
        """
        if not self.collection:
             # This shouldn't happen if main.py calls initialize_collection first, but as a safeguard:
            logger.warning("ChromaDB collection not initialized before processing. Attempting initialization.")
            self.initialize_collection()
            if not self.collection:
                 raise RuntimeError("Failed to initialize ChromaDB collection during processing.")

        # Note: The original code cleared the collection here. Removed as main.py doesn't seem to rely on
        # data persistence *within* this processor across calls via the collection itself.
        # If clearing is desired before processing, it should be done explicitly before calling this method.

        logger.info(f"Processing transcript (length {len(text)}) with model={model}/{model_name}, chunk_size={chunk_size}, overlap={overlap}")

        all_json_data = []
        agent = None # Define agent variable

        try:
            # Select and initialize the AI model and agent
            if model == "claude":
                api_key = os.getenv("ANTHROPIC_API_KEY")
                if not api_key: raise ValueError("ANTHROPIC_API_KEY not set")
                llm = AnthropicModel(model_name, api_key=api_key)
            elif model == "ollama":
                # Note: Ollama might require different chunk/overlap defaults
                # chunk_size = 1900 # Example adjustment if needed
                # overlap = 200
                llm = OllamaModel(model_name) # Assumes Ollama server is running locally
            elif model == "groq":
                api_key = os.getenv("GROQ_API_KEY")
                if not api_key: raise ValueError("GROQ_API_KEY not set")
                llm = GroqModel(model_name, api_key=api_key)
            else:
                raise ValueError(f"Unsupported model provider: {model}")

            agent = Agent(
                llm,
                result_type=SummaryResponse,
                result_retries=5, # Reduced retries slightly for faster failure
            )

            # Split transcript into chunks
            # Ensure step is positive, adjust if overlap is too large
            step = chunk_size - overlap
            if step <= 0:
                logger.warning(f"Overlap ({overlap}) >= chunk_size ({chunk_size}). Adjusting overlap.")
                overlap = max(0, chunk_size - 100) # Ensure at least a small step
                step = chunk_size - overlap

            chunks = [text[i:i+chunk_size] for i in range(0, len(text), step)]
            num_chunks = len(chunks)
            logger.info(f"Split transcript into {num_chunks} chunks.")

            for i, chunk in enumerate(chunks):
                logger.info(f"Processing chunk {i+1}/{num_chunks}...")
                try:
                    # Run the agent to get the structured summary for the chunk
                    summary_result = await agent.run(
                        f"""Given the following meeting transcript chunk, extract the relevant information according to the required JSON structure. If a specific section (like Critical Deadlines) has no relevant information in this chunk, return an empty list for its 'blocks'. Ensure the output is only the JSON data.

                        Transcript Chunk:
                        ---
                        {chunk}
                        ---
                        """,
                    )

                    # pydantic-ai might return the data directly or nested under .data
                    if hasattr(summary_result, 'data') and isinstance(summary_result.data, SummaryResponse):
                         final_summary_pydantic = summary_result.data
                    elif isinstance(summary_result, SummaryResponse):
                         final_summary_pydantic = summary_result
                    else:
                         logger.error(f"Unexpected result type from agent for chunk {i+1}: {type(summary_result)}")
                         # Handle error - perhaps add a placeholder or skip
                         continue # Skip this chunk

                    # Convert the Pydantic model to a JSON string
                    chunk_summary_json = final_summary_pydantic.model_dump_json()
                    all_json_data.append(chunk_summary_json)
                    logger.info(f"Successfully generated summary for chunk {i+1}.")

                    # Removed self.collection.add(...) as it's not used by main.py's logic

                except Exception as chunk_error:
                    logger.error(f"Error processing chunk {i+1}: {chunk_error}", exc_info=True)
                    # Decide how to handle chunk errors: skip, add placeholder, or raise?
                    # For now, we log and continue. Consider adding a placeholder JSON.
                    # all_json_data.append(json.dumps({"error": f"Failed to process chunk {i+1}", "details": str(chunk_error)}))


            logger.info(f"Finished processing all {num_chunks} chunks.")
            return num_chunks, all_json_data

        except Exception as e:
            logger.error(f"Error during transcript processing: {str(e)}", exc_info=True)
            # Re-raise the exception so the background task in main.py can catch it
            raise
