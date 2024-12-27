from sqlalchemy import Column, Integer, String, DateTime, Text
from sqlalchemy.orm import declarative_base
from datetime import datetime

Base = declarative_base()

class Meeting(Base):
    __tablename__ = "meetings"

    id = Column(Integer, primary_key=True, index=True)
    title = Column(String, index=True)
    created_at = Column(DateTime, default=datetime.utcnow)
    status = Column(String, default="in_progress")
    audio_path = Column(String, nullable=True)
    transcription = Column(Text, nullable=True)
    summary = Column(Text, nullable=True)

    def to_dict(self):
        return {
            "id": self.id,
            "title": self.title,
            "created_at": self.created_at.isoformat(),
            "status": self.status,
            "transcription": self.transcription,
            "summary": self.summary
        }
