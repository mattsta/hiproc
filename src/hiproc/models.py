"""
SQLAlchemy ORM models for the hiproc database.

This module defines the database schema for hiproc, including:
- Command storage with full context and metadata
- Execution history tracking for analytics
- User preferences for personalization
"""

from sqlalchemy import Column, DateTime, ForeignKey, Integer, String, Text
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func

from .database import Base


class Command(Base):
    """
    Represents a command stored in the database.

    Attributes:
        id: The primary key.
        command_string: The full command string that was executed.
        name: A user-defined friendly name for the command.
        namespace: A user-defined category for the command.
        user: The username of the person who saved the command.
        cwd: The current working directory where the command was saved.
        hostname: The hostname of the machine where the command was saved.
        scope: The scope of the command, e.g., "personal" or a team name.
        created_at: The timestamp when the command was saved.
        last_used_at: The timestamp when the command was last used.
        use_count: The number of times the command has been used.
    """

    __tablename__ = "commands"

    id = Column(Integer, primary_key=True, index=True)
    command_string = Column(String, nullable=False)
    name = Column(String, index=True)
    namespace = Column(String, index=True)
    user = Column(String, index=True)
    cwd = Column(String)
    hostname = Column(String, index=True)
    scope = Column(String, index=True, default="personal")
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    last_used_at = Column(DateTime(timezone=True))
    use_count = Column(Integer, default=0)

    # Relationships
    executions = relationship(
        "ExecutionHistory", back_populates="command", cascade="all, delete-orphan"
    )


class ExecutionHistory(Base):
    """
    Tracks individual command executions for analytics and recommendations.

    This table stores detailed execution information including:
    - When and where commands were executed
    - Execution context (directory, arguments)
    - Success/failure status
    - Performance metrics
    """

    __tablename__ = "execution_history"

    id = Column(Integer, primary_key=True, index=True)
    command_id = Column(Integer, ForeignKey("commands.id"), nullable=False, index=True)
    user = Column(String, index=True)
    hostname = Column(String, index=True)
    cwd = Column(String, index=True)
    executed_at = Column(DateTime(timezone=True), server_default=func.now(), index=True)
    arguments = Column(Text)  # JSON string of arguments passed
    execution_method = Column(String)  # "id", "name", "namespace_name", "find"
    duration_ms = Column(Integer)  # Execution duration in milliseconds
    exit_code = Column(Integer)  # Command exit code (if tracked)

    # Relationships
    command = relationship("Command", back_populates="executions")


class UserPreferences(Base):
    """
    Stores user preferences and personalization settings.

    This includes:
    - Default namespaces and scopes
    - UI preferences
    - Command recommendation settings
    """

    __tablename__ = "user_preferences"

    id = Column(Integer, primary_key=True, index=True)
    user = Column(String, unique=True, index=True, nullable=False)
    default_namespace = Column(String)
    default_scope = Column(String, default="personal")
    preferred_execution_method = Column(String)  # "id", "name", "find"
    enable_suggestions = Column(Integer, default=1)  # Boolean as integer
    max_suggestions = Column(Integer, default=5)
    settings_json = Column(Text)  # Additional settings as JSON
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    updated_at = Column(DateTime(timezone=True), onupdate=func.now())


class ProjectContext(Base):
    """
    Tracks project contexts for intelligent namespace detection.

    This helps with automatic namespace detection based on:
    - Project file patterns (package.json, Cargo.toml, etc.)
    - Directory structures
    - Git repository information
    """

    __tablename__ = "project_contexts"

    id = Column(Integer, primary_key=True, index=True)
    directory_pattern = Column(String, unique=True, index=True)
    detected_namespace = Column(String, index=True)
    project_type = Column(
        String, index=True
    )  # "npm", "cargo", "python", "maven", "git"
    confidence_score = Column(Integer, default=100)  # 0-100 confidence in detection
    last_detected = Column(DateTime(timezone=True), server_default=func.now())
    usage_count = Column(Integer, default=1)  # How often this pattern was used
