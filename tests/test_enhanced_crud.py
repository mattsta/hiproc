"""
Unit tests for enhanced CRUD operations.

Tests the enhanced database operations including:
- Smart contextual recall algorithm
- Command access control
- Usage statistics tracking
"""

from datetime import datetime, timedelta

import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from hiproc import crud, models
from hiproc.database import Base

# Test database setup
SQLALCHEMY_DATABASE_URL = "sqlite:///./test_enhanced_crud.db"
engine = create_engine(
    SQLALCHEMY_DATABASE_URL, connect_args={"check_same_thread": False}
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


@pytest.fixture(scope="function")
def db():
    """Create a fresh database for each test."""
    Base.metadata.create_all(bind=engine)
    db = TestingSessionLocal()
    yield db
    db.close()
    Base.metadata.drop_all(bind=engine)


class TestEnhancedRecallAlgorithm:
    """Test the enhanced command recall algorithm."""

    def test_exact_context_priority(self, db):
        """Test that exact context matches get highest priority."""
        # Create commands with different contexts
        commands = [
            models.Command(
                command_string="echo 'exact match'",
                name="deploy",
                namespace="webapp",
                user="alice",
                hostname="laptop",
                cwd="/home/alice/webapp",
                scope="personal",
                use_count=1,  # Lower usage
            ),
            models.Command(
                command_string="echo 'high usage but wrong context'",
                name="deploy",
                namespace="webapp",
                user="alice",
                hostname="desktop",  # Different hostname
                cwd="/home/alice/webapp",
                scope="personal",
                use_count=10,  # Higher usage
            ),
        ]

        for cmd in commands:
            db.add(cmd)
        db.commit()

        # Exact context match should win despite lower usage
        result = crud.recall_command_by_name(
            db=db,
            name="deploy",
            user="alice",
            hostname="laptop",
            cwd="/home/alice/webapp",
            namespace_hint="webapp",
        )

        assert result is not None
        assert result.command_string == "echo 'exact match'"
        assert result.use_count == 2  # Should be incremented
        assert result.last_used_at is not None

    def test_user_hostname_priority(self, db):
        """Test user + hostname matching priority."""
        commands = [
            models.Command(
                command_string="echo 'user + hostname match'",
                name="build",
                namespace="project",
                user="alice",
                hostname="laptop",
                cwd="/different/directory",
                scope="personal",
            ),
            models.Command(
                command_string="echo 'different user'",
                name="build",
                namespace="project",
                user="bob",
                hostname="laptop",
                cwd="/home/alice/project",  # Better directory match
                scope="personal",
            ),
        ]

        for cmd in commands:
            db.add(cmd)
        db.commit()

        result = crud.recall_command_by_name(
            db=db,
            name="build",
            user="alice",
            hostname="laptop",
            cwd="/home/alice/project",
        )

        assert result.command_string == "echo 'user + hostname match'"

    def test_directory_pattern_matching(self, db):
        """Test directory pattern matching works correctly."""
        command = models.Command(
            command_string="npm test",
            name="test",
            namespace="frontend",
            user="alice",
            cwd="/home/alice/projects/my-webapp",
        )
        db.add(command)
        db.commit()

        # Should match similar directory structure
        result = crud.recall_command_by_name(
            db=db,
            name="test",
            user="bob",  # Different user
            cwd="/home/bob/work/other-webapp",  # Similar ending
        )

        assert result is not None
        assert result.command_string == "npm test"

    def test_namespace_hint_priority(self, db):
        """Test namespace hint affects selection."""
        commands = [
            models.Command(
                command_string="echo 'webapp version'",
                name="deploy",
                namespace="webapp",
                user="alice",
                use_count=1,
            ),
            models.Command(
                command_string="echo 'api version'",
                name="deploy",
                namespace="api",
                user="alice",
                use_count=10,  # Higher usage
            ),
        ]

        for cmd in commands:
            db.add(cmd)
        db.commit()

        # Namespace hint should override usage frequency
        result = crud.recall_command_by_name(
            db=db, name="deploy", user="alice", namespace_hint="webapp"
        )

        assert result.command_string == "echo 'webapp version'"

    def test_scope_hint_priority(self, db):
        """Test scope hint affects selection."""
        commands = [
            models.Command(
                command_string="echo 'personal version'",
                name="deploy",
                namespace="project",
                user="alice",
                scope="personal",
                use_count=1,
            ),
            models.Command(
                command_string="echo 'team version'",
                name="deploy",
                namespace="project",
                user="alice",
                scope="team",
                use_count=10,
            ),
        ]

        for cmd in commands:
            db.add(cmd)
        db.commit()

        result = crud.recall_command_by_name(
            db=db, name="deploy", user="alice", scope_hint="team"
        )

        assert result.command_string == "echo 'team version'"

    def test_frequency_fallback(self, db):
        """Test frequency-based selection when no context matches."""
        commands = [
            models.Command(
                command_string="echo 'rarely used'",
                name="build",
                namespace="project",
                user="alice",
                use_count=1,
                last_used_at=datetime.utcnow() - timedelta(days=30),
            ),
            models.Command(
                command_string="echo 'frequently used'",
                name="build",
                namespace="project",
                user="alice",
                use_count=15,
                last_used_at=datetime.utcnow() - timedelta(hours=1),
            ),
        ]

        for cmd in commands:
            db.add(cmd)
        db.commit()

        result = crud.recall_command_by_name(db=db, name="build", user="alice")

        assert result.command_string == "echo 'frequently used'"

    def test_global_fallback(self, db):
        """Test global fallback when no user-specific commands exist."""
        command = models.Command(
            command_string="echo 'global command'",
            name="utility",
            namespace="tools",
            user="alice",
            use_count=5,
        )
        db.add(command)
        db.commit()

        # Bob searches for command he doesn't own
        result = crud.recall_command_by_name(db=db, name="utility", user="bob")

        assert result.command_string == "echo 'global command'"

    def test_no_matches_returns_none(self, db):
        """Test that non-existent command names return None."""
        result = crud.recall_command_by_name(
            db=db, name="nonexistent-command", user="alice"
        )

        assert result is None


class TestCommandAccessControl:
    """Test command access control functionality."""

    def test_get_owned_command(self, db):
        """Test retrieving a command owned by the user."""
        command = models.Command(
            command_string="echo 'my command'",
            name="test",
            namespace="personal",
            user="alice",
            scope="personal",
        )
        db.add(command)
        db.commit()

        result = crud.get_command_by_id(db, command.id, "alice")
        assert result is not None
        assert result.command_string == "echo 'my command'"

    def test_cannot_get_others_personal_command(self, db):
        """Test that users can't access others' personal commands."""
        command = models.Command(
            command_string="echo 'alice secret'",
            name="secret",
            namespace="personal",
            user="alice",
            scope="personal",
        )
        db.add(command)
        db.commit()

        result = crud.get_command_by_id(db, command.id, "bob")
        assert result is None

    def test_can_get_shared_command(self, db):
        """Test that users can access shared scope commands."""
        command = models.Command(
            command_string="echo 'shared utility'",
            name="utility",
            namespace="tools",
            user="alice",
            scope="team-shared",
        )
        db.add(command)
        db.commit()

        result = crud.get_command_by_id(db, command.id, "bob")
        assert result is not None
        assert result.command_string == "echo 'shared utility'"


class TestUsageTracking:
    """Test usage statistics tracking."""

    def test_track_execution_increments_stats(self, db):
        """Test that execution tracking updates usage stats."""
        command = models.Command(
            command_string="echo 'tracked command'",
            name="track-test",
            namespace="testing",
            user="alice",
            use_count=5,
        )
        db.add(command)
        db.commit()

        initial_time = command.last_used_at

        result = crud.track_execution(db, command.id, "alice")

        assert result is not None
        assert result.use_count == 6
        assert result.last_used_at != initial_time
        assert result.last_used_at is not None

    def test_track_execution_respects_permissions(self, db):
        """Test that execution tracking respects command permissions."""
        command = models.Command(
            command_string="echo 'personal command'",
            name="personal-test",
            namespace="testing",
            user="alice",
            scope="personal",
        )
        db.add(command)
        db.commit()

        # Bob shouldn't be able to track execution of Alice's personal command
        result = crud.track_execution(db, command.id, "bob")
        assert result is None

        # Original command should be unchanged
        db.refresh(command)
        assert command.use_count == 0

    def test_update_usage_stats_helper(self, db):
        """Test the usage stats helper function."""
        command = models.Command(
            command_string="echo 'test'",
            name="test",
            namespace="testing",
            user="alice",
            use_count=2,
            last_used_at=datetime.utcnow() - timedelta(hours=1),
        )
        db.add(command)
        db.commit()

        old_time = command.last_used_at
        result = crud._update_usage_stats(db, command)

        assert result.use_count == 3
        assert result.last_used_at > old_time


class TestErrorHandling:
    """Test error handling in CRUD operations."""

    def test_get_nonexistent_command(self, db):
        """Test getting a command that doesn't exist."""
        result = crud.get_command_by_id(db, 99999, "alice")
        assert result is None

    def test_track_nonexistent_command(self, db):
        """Test tracking execution of nonexistent command."""
        result = crud.track_execution(db, 99999, "alice")
        assert result is None

    def test_recall_with_all_none_parameters(self, db):
        """Test recall behavior when all optional parameters are None."""
        command = models.Command(
            command_string="echo 'fallback command'",
            name="fallback",
            namespace="test",
            user="alice",
        )
        db.add(command)
        db.commit()

        result = crud.recall_command_by_name(
            db=db,
            name="fallback",
            user=None,
            hostname=None,
            cwd=None,
            namespace_hint=None,
            scope_hint=None,
        )

        assert result is not None
        assert result.command_string == "echo 'fallback command'"
