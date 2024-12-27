from fastapi import FastAPI, HTTPException, UploadFile, File, Depends, Body
from fastapi.middleware.cors import CORSMiddleware
from sqlalchemy.orm import Session
from typing import Optional, List
import os
from datetime import datetime
from pydantic import BaseModel

from . import models
from . import schemas
from .database import SessionLocal, engine, get_db
from .services.summarization_service import summarization_service
from .services.transcription_service import transcription_service
from .services.audio_service import audio_service

app = FastAPI()

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Create database tables
models.Base.metadata.create_all(bind=engine)

class TextInput(BaseModel):
    text: str

@app.get("/health")
def health_check():
    """Health check endpoint."""
    return {"status": "healthy"}

@app.post("/recording/start")
def start_recording(db: Session = Depends(get_db)):
    """Start recording audio."""
    try:
        session_id = audio_service.start_recording()
        
        # Create a new meeting record
        new_meeting = models.Meeting(
            title=f"Meeting {datetime.now().strftime('%Y-%m-%d %H:%M')}",
            status="recording",
            audio_path=os.path.join(audio_service.recording_dir, f"{session_id}.wav")
        )
        db.add(new_meeting)
        db.commit()
        db.refresh(new_meeting)
        
        return {"session_id": session_id, "meeting_id": new_meeting.id, "message": "Recording started"}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/recording/stop/{session_id}")
async def stop_recording(session_id: str, db: Session = Depends(get_db)):
    """Stop recording and save the audio data."""
    try:
        # Find the meeting record
        meeting = db.query(models.Meeting).filter(
            models.Meeting.audio_path.like(f"%{session_id}.wav")
        ).first()
        
        if not meeting:
            raise ValueError(f"No meeting found for recording session {session_id}")
            
        # Stop the recording
        audio_service.stop_recording(session_id)
        
        # Update meeting status
        meeting.status = "recorded"
        db.commit()
        
        return {"status": "stopped", "meeting_id": meeting.id}
    except ValueError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        print(f"Error stopping recording: {str(e)}")  # Add logging
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/transcribe")
async def transcribe_audio_file(file: UploadFile = File(...)):
    """Transcribe an uploaded audio file."""
    try:
        # Save the uploaded file temporarily
        temp_path = f"temp_{datetime.now().strftime('%Y%m%d_%H%M%S')}.wav"
        try:
            # Write the file content
            with open(temp_path, "wb") as f:
                content = await file.read()
                f.write(content)
            
            # Transcribe the audio
            transcription = transcription_service.transcribe_file(temp_path)
            return {"transcription": transcription}
            
        finally:
            # Clean up
            if os.path.exists(temp_path):
                os.remove(temp_path)
                
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.get("/transcribe/{session_id}")
async def get_session_transcription(session_id: str):
    """Get transcription for a recording session."""
    try:
        # Get the path to the audio file
        audio_path = audio_service.get_recording_path(session_id)
        
        # Transcribe the audio
        transcription = transcription_service.transcribe_file(audio_path)
        return {"transcription": transcription}
        
    except ValueError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        print(f"Error in transcription: {str(e)}")  # Add logging
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/summarize")
def summarize_text(text_input: TextInput):
    """Generate a summary of the provided text."""
    try:
        summary = summarization_service.summarize(text_input.text)
        if not summary:
            raise HTTPException(status_code=400, detail="Failed to generate summary")
        return {"summary": summary}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/meetings", response_model=schemas.Meeting)
def create_meeting(meeting: schemas.MeetingCreate, db: Session = Depends(get_db)):
    """Create a new meeting record."""
    db_meeting = models.Meeting(**meeting.dict())
    db.add(db_meeting)
    db.commit()
    db.refresh(db_meeting)
    return db_meeting

@app.get("/meetings/{meeting_id}", response_model=schemas.Meeting)
def get_meeting(meeting_id: int, db: Session = Depends(get_db)):
    """Get a specific meeting by ID."""
    meeting = db.query(models.Meeting).filter(models.Meeting.id == meeting_id).first()
    if meeting is None:
        raise HTTPException(status_code=404, detail="Meeting not found")
    return meeting

@app.get("/meetings", response_model=List[schemas.Meeting])
def list_meetings(skip: int = 0, limit: int = 10, db: Session = Depends(get_db)):
    """List all meetings with pagination."""
    meetings = db.query(models.Meeting).offset(skip).limit(limit).all()
    return meetings

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8000)
