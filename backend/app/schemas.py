from pydantic import BaseModel
from typing import Optional
from datetime import datetime

class MeetingBase(BaseModel):
    title: str
    transcription: Optional[str] = None
    summary: Optional[str] = None

class MeetingCreate(MeetingBase):
    pass

class Meeting(MeetingBase):
    id: int
    created_at: datetime
    status: Optional[str] = None
    audio_path: Optional[str] = None

    class Config:
        from_attributes = True
