import os
import requests
from typing import Optional

class SummarizationService:
    def __init__(self):
        self.ollama_url = os.getenv("OLLAMA_URL", "http://localhost:11434")
        self.model_name = os.getenv("SUMMARIZATION_MODEL", "qwen2.5:3b")

    def summarize(self, text: str) -> Optional[str]:
        """Generate a summary of the provided text."""
        if not text:
            return ""

        # If text is very short (less than 100 characters), return as is
        if len(text.strip()) < 100:
            return text.strip()

        try:
            # Prepare the prompt
            prompt = f"""Please provide a VERY CONCISE summary of the following text in 1-2 sentences. The summary MUST be shorter than the original text:

{text}

Concise summary:"""

            # Call Ollama API
            response = requests.post(
                f"{self.ollama_url}/api/generate",
                json={
                    "model": self.model_name,
                    "prompt": prompt,
                    "stream": False,
                    "options": {
                        "temperature": 0.3,  # Lower temperature for more focused output
                        "top_p": 0.1,  # More focused sampling
                        "max_tokens": 100  # Limit response length
                    }
                }
            )
            response.raise_for_status()
            
            # Extract summary from response
            result = response.json()
            summary = result.get("response", "").strip()
            
            # If summary is longer than original text, return a truncated version
            if len(summary) >= len(text):
                words = text.split()
                return " ".join(words[:len(words)//2]) + "..."
                
            return summary
        except Exception as e:
            print(f"Error in summarization: {e}")
            return None

# Create a mock summarization service for testing
class MockSummarizationService:
    def summarize(self, text: str) -> str:
        if not text:
            return ""
            
        # Return short text as is
        if len(text.strip()) < 100:
            return text.strip()
            
        # For longer text, return a truncated version
        words = text.split()
        return " ".join(words[:len(words)//2]) + "..."

# Global instance of the summarization service
# Use mock service if in test environment
if os.getenv("TESTING") == "true":
    summarization_service = MockSummarizationService()
else:
    summarization_service = SummarizationService()
