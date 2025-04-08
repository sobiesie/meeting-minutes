import aiosqlite
import json
from datetime import datetime, timedelta
import uuid
from typing import Optional, Dict, Any
import logging
import asyncio
from contextlib import asynccontextmanager
import time
import sqlite3

logger = logging.getLogger(__name__)

class DatabaseManager:
    def __init__(self, db_path: str = "summaries.db"):
        self.db_path = db_path
        self._init_db()

    def _init_db(self):
        """Initialize the database with required tables"""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            
            # Create meetings table
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS meetings (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL UNIQUE,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )
            """)
            
            # Create transcripts table
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS transcripts (
                    id TEXT PRIMARY KEY,
                    meeting_id TEXT NOT NULL,
                    transcript TEXT NOT NULL,
                    summary TEXT,
                    action_items TEXT,
                    key_points TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    FOREIGN KEY (meeting_id) REFERENCES meetings(id)
                )
            """)
            
            # Create summary_processes table (keeping existing functionality)
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS summary_processes (
                    id TEXT PRIMARY KEY,
                    status TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    error TEXT,
                    result TEXT
                )
            """)
            
            conn.commit()

    @asynccontextmanager
    async def _get_connection(self):
        """Get a new database connection"""
        conn = await aiosqlite.connect(self.db_path)
        try:
            yield conn
        finally:
            await conn.close()

    async def create_process(self) -> str:
        """Create a new process entry and return its ID"""
        process_id = str(uuid.uuid4())
        now = datetime.utcnow().isoformat()
        
        async with self._get_connection() as conn:
            await conn.execute(
                "INSERT INTO summary_processes (id, status, created_at, updated_at, start_time) VALUES (?, ?, ?, ?, ?)",
                (process_id, "PENDING", now, now, now)
            )
            await conn.commit()
        
        return process_id

    async def update_process(self, process_id: str, status: str, result: Optional[Dict] = None, error: Optional[str] = None, 
                           chunk_count: Optional[int] = None, processing_time: Optional[float] = None, 
                           metadata: Optional[Dict] = None):
        """Update a process status and result"""
        now = datetime.utcnow().isoformat()
        
        async with self._get_connection() as conn:
            update_fields = ["status = ?", "updated_at = ?"]
            params = [status, now]
            
            if result:
                update_fields.append("result = ?")
                params.append(json.dumps(result))
            if error:
                update_fields.append("error = ?")
                params.append(error)
            if chunk_count is not None:
                update_fields.append("chunk_count = ?")
                params.append(chunk_count)
            if processing_time is not None:
                update_fields.append("processing_time = ?")
                params.append(processing_time)
            if metadata:
                update_fields.append("metadata = ?")
                params.append(json.dumps(metadata))
            if status == 'COMPLETED' or status == 'FAILED':
                update_fields.append("end_time = ?")
                params.append(now)
                
            params.append(process_id)
            query = f"UPDATE summary_processes SET {', '.join(update_fields)} WHERE id = ?"
            await conn.execute(query, params)
            await conn.commit()

    async def get_process(self, process_id: str) -> Optional[Dict[str, Any]]:
        """Get a process by its ID"""
        async with self._get_connection() as conn:
            async with conn.execute(
                "SELECT id, status, created_at, updated_at, result, error, start_time, end_time, chunk_count, processing_time, metadata FROM summary_processes WHERE id = ?",
                (process_id,)
            ) as cursor:
                row = await cursor.fetchone()
                
                if not row:
                    return None
                    
                result = {
                    "id": row[0],
                    "status": row[1],
                    "created_at": row[2],
                    "updated_at": row[3],
                    "start_time": row[6],
                    "end_time": row[7],
                    "chunk_count": row[8],
                    "processing_time": row[9]
                }
                
                if row[4]:  # result
                    result["result"] = json.loads(row[4])
                if row[5]:  # error
                    result["error"] = row[5]
                if row[10]:  # metadata
                    result["metadata"] = json.loads(row[10])
                    
                return result

    async def save_transcript(self, process_id: str, transcript_text: str, model: str, model_name: str, 
                            chunk_size: int, overlap: int):
        """Save transcript data"""
        now = datetime.utcnow().isoformat()
        async with self._get_connection() as conn:
            await conn.execute("""
                INSERT INTO transcripts (process_id, transcript_text, model, model_name, chunk_size, overlap, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            """, (process_id, transcript_text, model, model_name, chunk_size, overlap, now))
            await conn.commit()

    async def update_meeting_name(self, process_id: str, meeting_name: str):
        """Update meeting name for a transcript"""
        async with self._get_connection() as conn:
            await conn.execute("""
                UPDATE transcripts SET meeting_name = ? WHERE process_id = ?
            """, (meeting_name, process_id))
            await conn.commit()

    async def get_transcript_data(self, process_id: str):
        """Get transcript data for a process"""
        async with self._get_connection() as conn:
            async with conn.execute("""
                SELECT t.*, p.status, p.result 
                FROM transcripts t 
                JOIN summary_processes p ON t.process_id = p.id 
                WHERE t.process_id = ?
            """, (process_id,)) as cursor:
                row = await cursor.fetchone()
                if row:
                    return dict(zip([col[0] for col in cursor.description], row))
                return None

    async def cleanup_old_processes(self, hours: int = 24):
        """Clean up processes older than specified hours"""
        cutoff = (datetime.utcnow() - timedelta(hours=hours)).isoformat()
        
        async with self._get_connection() as conn:
            await conn.execute(
                "DELETE FROM summary_processes WHERE created_at < ?",
                (cutoff,)
            )
            await conn.commit()

    async def save_meeting_transcript(self, meeting_id: str, title: str, transcript: str, summary: str, action_items: str, key_points: str):
        """Save meeting transcript and summary to the database"""
        try:
            with sqlite3.connect(self.db_path) as conn:
                cursor = conn.cursor()
                
                # Check if a meeting with the same title already exists
                cursor.execute("SELECT id FROM meetings WHERE title = ?", (title,))
                existing_meeting = cursor.fetchone()
                
                if existing_meeting:
                    raise ValueError(f"A meeting with the title '{title}' already exists")
                
                # Create or update meeting
                cursor.execute("""
                    INSERT OR REPLACE INTO meetings (id, title, created_at, updated_at)
                    VALUES (?, ?, datetime('now'), datetime('now'))
                """, (meeting_id, title))
                
                # Save transcript
                cursor.execute("""
                    INSERT OR REPLACE INTO transcripts (
                        meeting_id, transcript, summary, action_items, key_points,
                        created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))
                """, (meeting_id, transcript, summary, action_items, key_points))
                
                conn.commit()
                return True
        except sqlite3.IntegrityError as e:
            if "UNIQUE constraint failed" in str(e):
                raise ValueError(f"A meeting with the title '{title}' already exists")
            raise
        except Exception as e:
            raise

    async def get_meeting(self, meeting_id: str):
        """Get a meeting by ID"""
        async with self._get_connection() as conn:
            cursor = await conn.execute("""
                SELECT id, title, created_at, updated_at
                FROM meetings
                WHERE id = ?
            """, (meeting_id,))
            meeting = await cursor.fetchone()
            if meeting:
                return {
                    'id': meeting[0],
                    'title': meeting[1],
                    'created_at': meeting[2],
                    'updated_at': meeting[3]
                }
            return None

    async def update_meeting_title(self, meeting_id: str, new_title: str):
        """Update a meeting's title"""
        now = datetime.utcnow().isoformat()
        async with self._get_connection() as conn:
            await conn.execute("""
                UPDATE meetings
                SET title = ?, updated_at = ?
                WHERE id = ?
            """, (new_title, now, meeting_id))
            await conn.commit()

    async def get_meeting_transcripts(self, meeting_id: str):
        """Get all transcripts for a meeting"""
        async with self._get_connection() as conn:
            cursor = await conn.execute("""
                SELECT id, text, timestamp, created_at
                FROM transcripts
                WHERE meeting_id = ?
                ORDER BY timestamp ASC
            """, (meeting_id,))
            rows = await cursor.fetchall()
            return [{
                'id': row[0],
                'text': row[1],
                'timestamp': row[2],
                'created_at': row[3]
            } for row in rows]

    async def get_all_meetings(self):
        """Get all meetings with their transcript counts"""
        async with self._get_connection() as conn:
            cursor = await conn.execute("""
                SELECT m.id, m.title, m.created_at, m.updated_at,
                       COUNT(t.id) as transcript_count
                FROM meetings m
                LEFT JOIN transcripts t ON m.id = t.meeting_id
                GROUP BY m.id
                ORDER BY m.created_at DESC
            """)
            rows = await cursor.fetchall()
            return [{
                'id': row[0],
                'title': row[1],
                'created_at': row[2],
                'updated_at': row[3],
                'transcript_count': row[4]
            } for row in rows]
