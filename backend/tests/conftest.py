import pytest
from fastapi.testclient import TestClient
import os
import tempfile
import shutil
from pathlib import Path
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

# Set testing environment variable
os.environ["TESTING"] = "true"

from app.main import app
from app.database import Base, get_db

# Test database URL
TEST_DB_URL = "sqlite:///:memory:"

# Create test engine
engine = create_engine(
    TEST_DB_URL,
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)

# Test SessionLocal
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

# Override the dependency
def override_get_db():
    try:
        db = TestingSessionLocal()
        Base.metadata.create_all(bind=engine)  # Create tables for each test
        yield db
    finally:
        db.close()

app.dependency_overrides[get_db] = override_get_db

@pytest.fixture(scope="session")
def test_app():
    """Create a test application"""
    return app

@pytest.fixture(scope="session")
def client():
    """Create a test client"""
    return TestClient(app)

@pytest.fixture(autouse=True)
def test_db():
    """Create test database tables"""
    Base.metadata.create_all(bind=engine)
    yield
    Base.metadata.drop_all(bind=engine)

@pytest.fixture
def db_session():
    """Create a new database session for a test"""
    connection = engine.connect()
    transaction = connection.begin()
    session = TestingSessionLocal(bind=connection)
    
    yield session
    
    session.close()
    transaction.rollback()
    connection.close()

@pytest.fixture(scope="session")
def test_data_dir():
    """Create a temporary directory for test data"""
    temp_dir = tempfile.mkdtemp()
    yield temp_dir
    shutil.rmtree(temp_dir)

@pytest.fixture(scope="session")
def test_audio_dir(test_data_dir):
    """Create a directory for test audio files"""
    audio_dir = Path(test_data_dir) / "audio"
    audio_dir.mkdir(exist_ok=True)
    return audio_dir

@pytest.fixture(autouse=True)
def clean_test_dir(test_data_dir):
    """Clean the test directory before each test"""
    for item in Path(test_data_dir).iterdir():
        if item.is_file():
            item.unlink()
        elif item.is_dir():
            shutil.rmtree(item)
    return test_data_dir

@pytest.fixture(autouse=True)
def mock_env_vars():
    """Set up test environment variables"""
    original_vars = {}
    test_vars = {
        "DATABASE_URL": TEST_DB_URL,
        "OLLAMA_URL": "http://localhost:11434",
        "RECORDINGS_DIR": "test_recordings"
    }
    
    # Save original values and set test values
    for key, value in test_vars.items():
        if key in os.environ:
            original_vars[key] = os.environ[key]
        os.environ[key] = value
    
    yield test_vars
    
    # Restore original values
    for key in test_vars:
        if key in original_vars:
            os.environ[key] = original_vars[key]
        else:
            del os.environ[key]
