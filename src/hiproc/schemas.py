"""
Pydantic schemas for the hiproc API.

These schemas define the shape of the data for API requests and responses,
providing validation and serialization.
"""

from datetime import UTC, datetime

from pydantic import BaseModel, ConfigDict, field_serializer


class CommandBase(BaseModel):
    """Base schema for a command, containing all common fields."""

    command_string: str
    name: str
    namespace: str
    user: str | None = None
    cwd: str | None = None
    hostname: str | None = None
    scope: str = "personal"


class CommandCreate(CommandBase):
    """Schema used for creating a new command."""

    pass


class CommandUpdate(BaseModel):
    """Schema for updating a command's command_string."""

    command_string: str


class RecallRequest(BaseModel):
    """Schema for a recall request."""

    name: str
    namespace: str
    user: str | None = None
    hostname: str | None = None
    cwd: str | None = None


class CommandRename(BaseModel):
    """Schema for renaming a command."""

    name: str
    namespace: str


class RecallByNameRequest(BaseModel):
    """Schema for recalling a command by name with enhanced context."""

    name: str
    user: str | None = None
    hostname: str | None = None
    cwd: str | None = None
    namespace_hint: str | None = None
    scope_hint: str | None = None


class QuickSaveRequest(BaseModel):
    """Schema for quick-saving a command."""

    command_string: str
    name: str
    namespace: str | None = None
    user: str | None = None
    cwd: str | None = None
    hostname: str | None = None
    scope: str = "personal"


class SuggestionsRequest(BaseModel):
    """Schema for getting contextual suggestions."""

    user: str | None = None
    hostname: str | None = None
    cwd: str | None = None
    project_type: str | None = None
    limit: int = 5


class ExecutionHistoryCreate(BaseModel):
    """Schema for creating execution history records."""

    command_id: int
    user: str | None = None
    hostname: str | None = None
    cwd: str | None = None
    arguments: str | None = None
    execution_method: str = "unknown"
    duration_ms: int | None = None
    exit_code: int | None = None


class ProjectContextRequest(BaseModel):
    """Schema for project context detection."""

    directory_path: str
    user: str | None = None


class ProjectContextResponse(BaseModel):
    """Schema for project context response."""

    detected_namespace: str | None = None
    project_type: str | None = None
    confidence_score: int = 0
    similar_commands: list[str] = []


class Command(CommandBase):
    """Schema for a command as it is stored in the database."""

    id: int
    created_at: datetime
    last_used_at: datetime | None = None
    use_count: int
    is_new: bool = False

    model_config = ConfigDict(from_attributes=True)

    @field_serializer("created_at", "last_used_at")
    def serialize_dt(self, dt: datetime, _info):
        if dt is None:
            return None
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=UTC)
        return dt.isoformat()
