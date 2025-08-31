"""
Comprehensive tests for enhanced hiproc features.

Tests the new functionality including:
- Direct command execution by ID
- Smart contextual command recall by name
- Enhanced matching algorithms
- Command access controls
"""

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from hiproc.database import Base
from hiproc.main import app, get_db

# Test database setup
SQLALCHEMY_DATABASE_URL = "sqlite:///./test_enhanced.db"
engine = create_engine(
    SQLALCHEMY_DATABASE_URL, connect_args={"check_same_thread": False}
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


def override_get_db():
    try:
        db = TestingSessionLocal()
        yield db
    finally:
        db.close()


app.dependency_overrides[get_db] = override_get_db
client = TestClient(app)


@pytest.fixture(scope="function")
def db_setup():
    """Set up test database for each test."""
    Base.metadata.drop_all(bind=engine)  # Clean first
    Base.metadata.create_all(bind=engine)
    yield
    Base.metadata.drop_all(bind=engine)


class TestDirectCommandExecution:
    """Test direct command execution by ID functionality."""

    def test_get_command_by_id_owned(self, db_setup):
        """Test retrieving a command by ID that the user owns."""
        # Create a test command
        response = client.post(
            "/commands/",
            json={
                "command_string": "echo 'test command'",
                "name": "test",
                "namespace": "testing",
                "user": "alice",
                "cwd": "/home/alice",
                "hostname": "laptop",
                "scope": "personal",
            },
        )
        assert response.status_code == 200
        command_id = response.json()["id"]

        # Retrieve it by ID
        response = client.get(f"/commands/by-id/{command_id}?user=alice")
        assert response.status_code == 200

        data = response.json()
        assert data["id"] == command_id
        assert data["command_string"] == "echo 'test command'"
        assert data["user"] == "alice"

    def test_get_command_by_id_not_owned(self, db_setup):
        """Test retrieving a command by ID that the user doesn't own."""
        # Create a test command as alice
        response = client.post(
            "/commands/",
            json={
                "command_string": "echo 'alice command'",
                "name": "test",
                "namespace": "testing",
                "user": "alice",
                "scope": "personal",
            },
        )
        assert response.status_code == 200
        command_id = response.json()["id"]

        # Try to retrieve as bob
        response = client.get(f"/commands/by-id/{command_id}?user=bob")
        assert response.status_code == 404

    def test_get_shared_command_by_id(self, db_setup):
        """Test that shared scope commands are accessible to all users."""
        # Create a shared command as alice
        response = client.post(
            "/commands/",
            json={
                "command_string": "echo 'shared command'",
                "name": "shared-test",
                "namespace": "team",
                "user": "alice",
                "scope": "team-shared",
            },
        )
        assert response.status_code == 200
        command_id = response.json()["id"]

        # Bob should be able to access it
        response = client.get(f"/commands/by-id/{command_id}?user=bob")
        assert response.status_code == 200
        assert response.json()["command_string"] == "echo 'shared command'"

    def test_track_command_execution(self, db_setup):
        """Test tracking command execution updates usage stats."""
        # Create a test command
        response = client.post(
            "/commands/",
            json={
                "command_string": "echo 'tracked command'",
                "name": "track-test",
                "namespace": "testing",
                "user": "alice",
            },
        )
        assert response.status_code == 200
        command_id = response.json()["id"]

        # Execute it multiple times
        for i in range(3):
            response = client.post(f"/commands/{command_id}/execute?user=alice")
            assert response.status_code == 200

            data = response.json()
            assert data["use_count"] == i + 1
            if i > 0:
                assert data["last_used_at"] is not None


class TestSmartContextualRecall:
    """Test smart contextual command recall by name."""

    def test_exact_context_match(self, db_setup):
        """Test exact context matching gets highest priority."""
        # Create commands with different contexts
        commands_data = [
            {
                "command_string": "echo 'exact match'",
                "name": "deploy",
                "namespace": "webapp",
                "user": "alice",
                "hostname": "laptop",
                "cwd": "/home/alice/projects/webapp",
                "scope": "personal",
            },
            {
                "command_string": "echo 'different directory'",
                "name": "deploy",
                "namespace": "webapp",
                "user": "alice",
                "hostname": "laptop",
                "cwd": "/home/alice/projects/other",
                "scope": "personal",
            },
        ]

        for cmd_data in commands_data:
            response = client.post("/commands/", json=cmd_data)
            assert response.status_code == 200

        # Request with exact context should match first command
        response = client.post(
            "/commands/recall-by-name",
            json={
                "name": "deploy",
                "user": "alice",
                "hostname": "laptop",
                "cwd": "/home/alice/projects/webapp",
                "namespace_hint": "webapp",
            },
        )
        assert response.status_code == 200
        assert response.json()["command_string"] == "echo 'exact match'"

    def test_namespace_hint_priority(self, db_setup):
        """Test namespace hint affects command selection."""
        # Create commands in different namespaces
        commands_data = [
            {
                "command_string": "echo 'webapp deploy'",
                "name": "deploy",
                "namespace": "webapp",
                "user": "alice",
                "hostname": "laptop",
            },
            {
                "command_string": "echo 'api deploy'",
                "name": "deploy",
                "namespace": "api",
                "user": "alice",
                "hostname": "laptop",
            },
        ]

        for cmd_data in commands_data:
            response = client.post("/commands/", json=cmd_data)
            assert response.status_code == 200

        # Request with namespace hint should prefer that namespace
        response = client.post(
            "/commands/recall-by-name",
            json={
                "name": "deploy",
                "user": "alice",
                "hostname": "laptop",
                "namespace_hint": "api",
            },
        )
        assert response.status_code == 200
        assert response.json()["command_string"] == "echo 'api deploy'"

    def test_frequency_based_fallback(self, db_setup):
        """Test that frequently used commands get priority in fallback."""
        # Create two similar commands
        commands_data = [
            {
                "command_string": "echo 'rarely used'",
                "name": "build",
                "namespace": "project",
                "user": "alice",
            },
            {
                "command_string": "echo 'frequently used'",
                "name": "build",
                "namespace": "project",
                "user": "alice",
            },
        ]

        command_ids = []
        for cmd_data in commands_data:
            response = client.post("/commands/", json=cmd_data)
            assert response.status_code == 200
            command_ids.append(response.json()["id"])

        # Make second command more frequently used
        for _ in range(5):
            client.post(f"/commands/{command_ids[1]}/execute?user=alice")

        # Recall should prefer frequently used command
        response = client.post(
            "/commands/recall-by-name", json={"name": "build", "user": "alice"}
        )
        assert response.status_code == 200
        assert response.json()["command_string"] == "echo 'frequently used'"

    def test_directory_pattern_matching(self, db_setup):
        """Test directory pattern matching for similar projects."""
        # Create command in specific directory
        response = client.post(
            "/commands/",
            json={
                "command_string": "npm test",
                "name": "test",
                "namespace": "frontend",
                "user": "alice",
                "cwd": "/home/alice/projects/my-webapp",
            },
        )
        assert response.status_code == 200

        # Request from similar directory should match
        response = client.post(
            "/commands/recall-by-name",
            json={
                "name": "test",
                "user": "bob",  # Different user
                "cwd": "/home/bob/projects/other-webapp",  # Similar directory structure
            },
        )
        assert response.status_code == 200
        assert response.json()["command_string"] == "npm test"

    def test_scope_hint_preference(self, db_setup):
        """Test scope hint affects command selection."""
        # Create personal and team commands
        commands_data = [
            {
                "command_string": "echo 'personal version'",
                "name": "deploy",
                "namespace": "project",
                "user": "alice",
                "scope": "personal",
            },
            {
                "command_string": "echo 'team version'",
                "name": "deploy",
                "namespace": "project",
                "user": "alice",
                "scope": "team",
            },
        ]

        for cmd_data in commands_data:
            response = client.post("/commands/", json=cmd_data)
            assert response.status_code == 200

        # Request with team scope hint
        response = client.post(
            "/commands/recall-by-name",
            json={"name": "deploy", "user": "alice", "scope_hint": "team"},
        )
        assert response.status_code == 200
        assert response.json()["command_string"] == "echo 'team version'"

    def test_command_not_found(self, db_setup):
        """Test appropriate error when command name not found."""
        response = client.post(
            "/commands/recall-by-name",
            json={"name": "nonexistent-command", "user": "alice"},
        )
        assert response.status_code == 404


class TestAccessControl:
    """Test command access control and permissions."""

    def test_personal_command_isolation(self, db_setup):
        """Test that personal commands are isolated between users."""
        # Alice creates a personal command
        response = client.post(
            "/commands/",
            json={
                "command_string": "echo 'alice secret'",
                "name": "secret",
                "namespace": "personal",
                "user": "alice",
                "scope": "personal",
            },
        )
        assert response.status_code == 200
        command_id = response.json()["id"]

        # Bob should not be able to access it
        response = client.get(f"/commands/by-id/{command_id}?user=bob")
        assert response.status_code == 404

        # Bob should not be able to execute it
        response = client.post(f"/commands/{command_id}/execute?user=bob")
        assert response.status_code == 404

    def test_shared_command_access(self, db_setup):
        """Test that shared commands are accessible to all users."""
        # Alice creates a shared command
        response = client.post(
            "/commands/",
            json={
                "command_string": "echo 'shared utility'",
                "name": "utility",
                "namespace": "tools",
                "user": "alice",
                "scope": "shared",
            },
        )
        assert response.status_code == 200
        command_id = response.json()["id"]

        # Bob should be able to access and execute it
        response = client.get(f"/commands/by-id/{command_id}?user=bob")
        assert response.status_code == 200

        response = client.post(f"/commands/{command_id}/execute?user=bob")
        assert response.status_code == 200


class TestEdgeCases:
    """Test edge cases and error conditions."""

    def test_invalid_command_id(self, db_setup):
        """Test handling of invalid command IDs."""
        response = client.get("/commands/by-id/99999?user=alice")
        assert response.status_code == 404

        response = client.post("/commands/99999/execute?user=alice")
        assert response.status_code == 404

    def test_missing_user_parameter(self, db_setup):
        """Test API behavior when user parameter is missing."""
        # This should be handled by FastAPI validation
        response = client.get("/commands/by-id/1")
        assert response.status_code == 422  # Validation error

    def test_empty_command_name(self, db_setup):
        """Test recall with empty command name."""
        response = client.post(
            "/commands/recall-by-name", json={"name": "", "user": "alice"}
        )
        assert response.status_code == 404
