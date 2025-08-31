"""CRUD (Create, Read, Update, Delete) operations for the database."""

from datetime import datetime, timedelta

from sqlalchemy import and_, distinct, func
from sqlalchemy.orm import Session

from . import models, schemas


def get_commands(
    db: Session,
    q: str | None = None,
    namespace: str | None = None,
    user: str | None = None,
    scope: str | None = None,
):
    """Get all commands, with optional filtering."""
    query = db.query(models.Command)
    if q:
        query = query.filter(models.Command.command_string.contains(q))
    if namespace:
        query = query.filter(models.Command.namespace == namespace)
    if user:
        query = query.filter(models.Command.user == user)
    if scope:
        query = query.filter(models.Command.scope == scope)
    return query.all()


def recall_command(
    db: Session,
    name: str,
    namespace: str,
    user: str | None = None,
    hostname: str | None = None,
    cwd: str | None = None,
):
    """
    Recall a command with contextual and scope-based prioritization.

    Search order:
    1. Personal scope with full context (user, hostname, cwd)
    2. Personal scope with partial context (user, hostname)
    3. Shared scopes with context (hostname, cwd)
    4. Shared scopes with partial context (hostname)
    5. Global fallback in any scope
    """
    # 1. Search in "personal" scope with full context
    personal_filters = [
        and_(
            models.Command.user == user,
            models.Command.hostname == hostname,
            models.Command.cwd == cwd,
        ),
        and_(models.Command.user == user, models.Command.hostname == hostname),
    ]
    for f in personal_filters:
        command = (
            db.query(models.Command)
            .filter(
                and_(
                    models.Command.name == name,
                    models.Command.namespace == namespace,
                    models.Command.scope == "personal",
                    f,
                )
            )
            .order_by(models.Command.created_at.desc())
            .first()
        )
        if command:
            command.use_count += 1
            command.last_used_at = datetime.utcnow()
            db.commit()
            db.refresh(command)
            return command

    # 2. Search in shared scopes with context
    shared_filters = [
        and_(models.Command.hostname == hostname, models.Command.cwd == cwd),
        and_(models.Command.hostname == hostname),
    ]
    for f in shared_filters:
        command = (
            db.query(models.Command)
            .filter(
                and_(
                    models.Command.name == name,
                    models.Command.namespace == namespace,
                    models.Command.scope != "personal",
                    f,
                )
            )
            .order_by(models.Command.created_at.desc())
            .first()
        )
        if command:
            command.use_count += 1
            command.last_used_at = datetime.utcnow()
            db.commit()
            db.refresh(command)
            return command

    # 3. Global fallback (any scope, no context)
    command = (
        db.query(models.Command)
        .filter(
            and_(models.Command.name == name, models.Command.namespace == namespace)
        )
        .order_by(models.Command.created_at.desc())
        .first()
    )

    if command:
        command.use_count += 1
        command.last_used_at = datetime.utcnow()
        db.commit()
        db.refresh(command)

    return command


def get_all_user_commands(db: Session, user: str):
    """Get all commands belonging to a specific user."""
    return db.query(models.Command).filter(models.Command.user == user).all()


def get_namespaces(db: Session):
    """Get a list of all unique namespaces."""
    return db.query(distinct(models.Command.namespace)).all()


def create_command(db: Session, command: schemas.CommandCreate):
    """
    Create a new command, preventing exact duplicates.

    If a command with the exact same attributes already exists, the existing
    command is returned with `is_new` set to False. Otherwise, a new one is
    created with `is_new` set to True.
    """
    # Check for an existing exact match
    command_data = command.model_dump()
    existing_command = (
        db.query(models.Command)
        .filter(
            and_(
                models.Command.command_string == command_data["command_string"],
                models.Command.name == command_data["name"],
                models.Command.namespace == command_data["namespace"],
                models.Command.user == command_data["user"],
                models.Command.cwd == command_data["cwd"],
                models.Command.hostname == command_data["hostname"],
                models.Command.scope == command_data["scope"],
            )
        )
        .first()
    )

    if existing_command:
        existing_command.is_new = False
        return existing_command

    # If no exact match, create a new command
    db_command = models.Command(**command_data)
    db.add(db_command)
    db.commit()
    db.refresh(db_command)
    db_command.is_new = True
    return db_command


def update_command(
    db: Session, command_id: int, user: str, command_update: schemas.CommandUpdate
):
    """Update a command's command_string, ensuring the user owns it."""
    db_command = (
        db.query(models.Command)
        .filter(and_(models.Command.id == command_id, models.Command.user == user))
        .first()
    )
    if db_command:
        db_command.command_string = command_update.command_string
        db.commit()
        db.refresh(db_command)
        return db_command
    return None


def rename_command(
    db: Session, command_id: int, user: str, command_rename: schemas.CommandRename
):
    """Rename a command's name and namespace, ensuring the user owns it."""
    db_command = (
        db.query(models.Command)
        .filter(and_(models.Command.id == command_id, models.Command.user == user))
        .first()
    )
    if db_command:
        db_command.name = command_rename.name
        db_command.namespace = command_rename.namespace
        db.commit()
        db.refresh(db_command)
        return db_command
    return None


def delete_command(db: Session, command_id: int, user: str):
    """Delete a command by its ID, ensuring the user owns it."""
    db_command = (
        db.query(models.Command)
        .filter(and_(models.Command.id == command_id, models.Command.user == user))
        .first()
    )
    if db_command:
        db.delete(db_command)
        db.commit()
        return db_command
    return None


def get_command_by_id(db: Session, command_id: int, user: str):
    """
    Get a single command by ID, ensuring the user has access to it.

    This checks both personal commands owned by the user and shared commands
    that are accessible to them.
    """
    # First check if user owns the command
    command = (
        db.query(models.Command)
        .filter(and_(models.Command.id == command_id, models.Command.user == user))
        .first()
    )

    if command:
        return command

    # If not owned, check if it's a shared command they can access
    # (shared scope commands are accessible to all users)
    command = (
        db.query(models.Command)
        .filter(
            and_(models.Command.id == command_id, models.Command.scope != "personal")
        )
        .first()
    )

    return command


def track_execution(db: Session, command_id: int, user: str):
    """
    Track command execution by incrementing usage stats.

    Returns the command if user has access to it, None otherwise.
    """
    # Get the command (this checks permissions)
    command = get_command_by_id(db, command_id, user)
    if command:
        command.use_count += 1
        command.last_used_at = datetime.utcnow()
        db.commit()
        db.refresh(command)
        return command
    return None


def recall_command_by_name(
    db: Session,
    name: str,
    user: str | None = None,
    hostname: str | None = None,
    cwd: str | None = None,
    namespace_hint: str | None = None,
    scope_hint: str | None = None,
):
    """
    Enhanced command recall by name with intelligent contextual matching.

    Matching Priority:
    1. Exact context match (user + hostname + cwd + namespace_hint)
    2. User + hostname + namespace_hint
    3. User + hostname + cwd (any namespace)
    4. User + hostname (any namespace/directory)
    5. Directory pattern matching (same project structure)
    6. Namespace preference (most used in this namespace)
    7. Scope preference (if scope_hint provided)
    8. Frequency-based (most recently/frequently used)
    9. Global fallback (any matching name)
    """

    # Build base query for the command name
    base_query = db.query(models.Command).filter(models.Command.name == name)

    # Priority 1: Exact context match with namespace hint
    if all([user, hostname, cwd, namespace_hint]):
        command = (
            base_query.filter(
                and_(
                    models.Command.user == user,
                    models.Command.hostname == hostname,
                    models.Command.cwd == cwd,
                    models.Command.namespace == namespace_hint,
                    models.Command.scope == "personal",
                )
            )
            .order_by(
                models.Command.last_used_at.desc().nullslast(),
                models.Command.created_at.desc(),
            )
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 2: User + hostname + namespace hint
    if all([user, hostname, namespace_hint]):
        command = (
            base_query.filter(
                and_(
                    models.Command.user == user,
                    models.Command.hostname == hostname,
                    models.Command.namespace == namespace_hint,
                    models.Command.scope == "personal",
                )
            )
            .order_by(
                models.Command.last_used_at.desc().nullslast(),
                models.Command.created_at.desc(),
            )
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 3: User + hostname + cwd (any namespace)
    if all([user, hostname, cwd]):
        command = (
            base_query.filter(
                and_(
                    models.Command.user == user,
                    models.Command.hostname == hostname,
                    models.Command.cwd == cwd,
                    models.Command.scope == "personal",
                )
            )
            .order_by(
                models.Command.use_count.desc(),
                models.Command.last_used_at.desc().nullslast(),
            )
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 4: User + hostname (any namespace/directory)
    if all([user, hostname]):
        command = (
            base_query.filter(
                and_(
                    models.Command.user == user,
                    models.Command.hostname == hostname,
                    models.Command.scope == "personal",
                )
            )
            .order_by(
                models.Command.use_count.desc(),
                models.Command.last_used_at.desc().nullslast(),
            )
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 5: Directory pattern matching (similar project structure)
    if cwd:
        # Look for commands in similar directory structures
        similar_pattern = f"%{cwd.split('/')[-1]}%" if "/" in cwd else f"%{cwd}%"
        command = (
            base_query.filter(
                and_(
                    models.Command.cwd.like(similar_pattern),
                    models.Command.user == user if user else True,
                )
            )
            .order_by(models.Command.use_count.desc())
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 6: Namespace preference
    if namespace_hint:
        command = (
            base_query.filter(models.Command.namespace == namespace_hint)
            .order_by(models.Command.use_count.desc())
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 7: Scope preference
    if scope_hint:
        command = (
            base_query.filter(models.Command.scope == scope_hint)
            .order_by(models.Command.use_count.desc())
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 8: User's most frequently used
    if user:
        command = (
            base_query.filter(models.Command.user == user)
            .order_by(models.Command.use_count.desc())
            .first()
        )
        if command:
            return _update_usage_stats(db, command)

    # Priority 9: Global fallback (most popular overall)
    command = base_query.order_by(
        models.Command.use_count.desc(), models.Command.last_used_at.desc().nullslast()
    ).first()

    if command:
        return _update_usage_stats(db, command)

    return None


def _update_usage_stats(db: Session, command: models.Command) -> models.Command:
    """Helper function to update usage statistics for a command."""
    command.use_count += 1
    command.last_used_at = datetime.utcnow()
    db.commit()
    db.refresh(command)
    return command


def create_execution_history(db: Session, execution: schemas.ExecutionHistoryCreate):
    """Create a new execution history record for analytics."""
    db_execution = models.ExecutionHistory(**execution.model_dump())
    db.add(db_execution)
    db.commit()
    db.refresh(db_execution)
    return db_execution


def get_suggestions_for_context(
    db: Session,
    user: str | None = None,
    hostname: str | None = None,
    cwd: str | None = None,
    project_type: str | None = None,
    limit: int = 5,
):
    """
    Get intelligent command suggestions based on current context.

    This function analyzes the user's context and suggests relevant commands
    based on:
    1. Commands frequently used in similar directories
    2. Commands recently executed in this project type
    3. Popular commands for this user's pattern
    4. Team commands relevant to the context
    """
    suggestions = []

    # Priority 1: Commands frequently used in similar directories
    if cwd:
        similar_dir_commands = (
            db.query(models.Command)
            .join(models.ExecutionHistory)
            .filter(models.ExecutionHistory.cwd.like(f"%{cwd.split('/')[-1]}%"))
            .group_by(models.Command.id)
            .order_by(func.count(models.ExecutionHistory.id).desc())
            .limit(limit)
            .all()
        )
        suggestions.extend(similar_dir_commands)

    # Priority 2: User's frequently used commands
    if user and len(suggestions) < limit:
        user_commands = (
            db.query(models.Command)
            .filter(models.Command.user == user)
            .order_by(models.Command.use_count.desc())
            .limit(limit - len(suggestions))
            .all()
        )
        suggestions.extend(user_commands)

    # Priority 3: Recently popular commands in shared scopes
    if len(suggestions) < limit:
        popular_shared = (
            db.query(models.Command)
            .filter(models.Command.scope != "personal")
            .order_by(models.Command.use_count.desc())
            .limit(limit - len(suggestions))
            .all()
        )
        suggestions.extend(popular_shared)

    # Remove duplicates while preserving order
    seen = set()
    unique_suggestions = []
    for cmd in suggestions:
        if cmd.id not in seen and len(unique_suggestions) < limit:
            seen.add(cmd.id)
            unique_suggestions.append(cmd)

    return unique_suggestions


def detect_project_context(db: Session, directory_path: str, user: str | None = None):
    """
    Detect and learn project context for intelligent namespace suggestions.

    Analyzes the directory structure and project files to suggest appropriate
    namespaces and related commands.
    """
    # Check if we've seen this directory pattern before
    dir_name = directory_path.split("/")[-1]
    existing_context = (
        db.query(models.ProjectContext)
        .filter(models.ProjectContext.directory_pattern == dir_name)
        .first()
    )

    if existing_context:
        existing_context.usage_count += 1
        existing_context.last_detected = datetime.utcnow()
        db.commit()
        db.refresh(existing_context)

        # Get commands in this namespace for suggestions
        similar_commands = (
            db.query(models.Command.name)
            .filter(models.Command.namespace == existing_context.detected_namespace)
            .limit(5)
            .all()
        )

        return schemas.ProjectContextResponse(
            detected_namespace=existing_context.detected_namespace,
            project_type=existing_context.project_type,
            confidence_score=min(100, existing_context.confidence_score + 5),
            similar_commands=[cmd.name for cmd in similar_commands],
        )

    # New directory - try to detect project type
    import os

    project_files = {
        "package.json": ("npm", "javascript"),
        "Cargo.toml": ("cargo", "rust"),
        "pyproject.toml": ("python", "python"),
        "pom.xml": ("maven", "java"),
        ".git": ("git", "git"),
        "requirements.txt": ("pip", "python"),
        "go.mod": ("go", "go"),
        "Dockerfile": ("docker", "docker"),
    }

    detected_type = None
    confidence = 50

    for filename, (proj_type, category) in project_files.items():
        file_path = os.path.join(directory_path, filename)
        if os.path.exists(file_path):
            detected_type = proj_type
            confidence = 80
            break

    # Use directory name as default namespace
    suggested_namespace = dir_name

    # Try to extract better name from project files
    if detected_type == "npm":
        try:
            import json

            with open(os.path.join(directory_path, "package.json")) as f:
                package_data = json.load(f)
                if "name" in package_data:
                    suggested_namespace = package_data["name"]
                    confidence = 90
        except:
            pass
    elif detected_type == "cargo":
        try:
            with open(os.path.join(directory_path, "Cargo.toml")) as f:
                for line in f:
                    if line.strip().startswith("name ="):
                        name = line.split("=", 1)[1].strip().strip('"')
                        suggested_namespace = name
                        confidence = 90
                        break
        except:
            pass

    # Store the learned context
    new_context = models.ProjectContext(
        directory_pattern=dir_name,
        detected_namespace=suggested_namespace,
        project_type=detected_type or "unknown",
        confidence_score=confidence,
        usage_count=1,
    )
    db.add(new_context)
    db.commit()

    return schemas.ProjectContextResponse(
        detected_namespace=suggested_namespace,
        project_type=detected_type,
        confidence_score=confidence,
        similar_commands=[],
    )


def get_command_similarity(db: Session, command_id: int, limit: int = 5):
    """
    Find commands similar to the given command based on:
    1. Same namespace
    2. Similar command strings (basic text similarity)
    3. Same user patterns
    4. Execution context similarity
    """
    base_command = (
        db.query(models.Command).filter(models.Command.id == command_id).first()
    )
    if not base_command:
        return []

    # Find commands in same namespace
    namespace_similar = (
        db.query(models.Command)
        .filter(
            and_(
                models.Command.namespace == base_command.namespace,
                models.Command.id != command_id,
            )
        )
        .order_by(models.Command.use_count.desc())
        .limit(limit)
        .all()
    )

    if len(namespace_similar) >= limit:
        return namespace_similar

    # Find commands with similar strings (basic approach)
    base_words = set(base_command.command_string.lower().split())
    remaining_commands = (
        db.query(models.Command).filter(models.Command.id != command_id).all()
    )

    # Simple word overlap similarity
    similar_commands = []
    for cmd in remaining_commands:
        cmd_words = set(cmd.command_string.lower().split())
        overlap = len(base_words.intersection(cmd_words))
        if overlap > 0:
            similar_commands.append((cmd, overlap))

    # Sort by overlap and take top results
    similar_commands.sort(key=lambda x: x[1], reverse=True)
    result = namespace_similar + [
        cmd for cmd, _ in similar_commands[: limit - len(namespace_similar)]
    ]

    return result[:limit]


def get_execution_analytics(db: Session, user: str | None = None, days: int = 30):
    """
    Get execution analytics for insights and recommendations.

    Returns:
    - Most executed commands
    - Execution patterns by time
    - Context-based usage patterns
    - Performance metrics
    """
    from_date = datetime.utcnow() - timedelta(days=days)

    query = db.query(models.ExecutionHistory).filter(
        models.ExecutionHistory.executed_at >= from_date
    )

    if user:
        query = query.filter(models.ExecutionHistory.user == user)

    # Most executed commands
    most_executed_raw = (
        query.join(models.Command)
        .with_entities(
            models.Command.name,
            models.Command.namespace,
            func.count(models.ExecutionHistory.id).label("execution_count"),
        )
        .group_by(models.Command.id, models.Command.name, models.Command.namespace)
        .order_by(func.count(models.ExecutionHistory.id).desc())
        .limit(10)
        .all()
    )

    # Convert to serializable format
    most_executed = []
    for row in most_executed_raw:
        most_executed.append(
            {
                "name": row.name,
                "namespace": row.namespace,
                "execution_count": row.execution_count,
            }
        )

    # Execution methods distribution
    method_stats_raw = (
        query.with_entities(
            models.ExecutionHistory.execution_method,
            func.count(models.ExecutionHistory.id).label("count"),
        )
        .group_by(models.ExecutionHistory.execution_method)
        .order_by(func.count(models.ExecutionHistory.id).desc())
        .all()
    )

    # Convert to serializable format
    method_stats = {}
    for row in method_stats_raw:
        method_stats[row.execution_method] = row.count

    # Additional statistics
    total_executions = query.count()

    # Calculate average executions per day
    unique_commands_count = (
        query.join(models.Command).with_entities(models.Command.id).distinct().count()
    )

    avg_per_day = total_executions / days if days > 0 else 0

    return {
        "total_executions": total_executions,
        "unique_commands": unique_commands_count,
        "average_executions_per_day": avg_per_day,
        "most_used_commands": most_executed,
        "execution_methods": method_stats,
    }
