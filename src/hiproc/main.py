"""Main FastAPI application for the hiproc server."""

from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.responses import HTMLResponse
from fastapi.templating import Jinja2Templates
from sqlalchemy.orm import Session

from . import crud, models, schemas
from .database import SessionLocal, engine

models.Base.metadata.create_all(bind=engine)

app = FastAPI(
    title="hiproc API",
    description="API for saving and recalling command-line commands.",
)

templates = Jinja2Templates(directory="src/hiproc/templates")


# Dependency
def get_db():
    """FastAPI dependency to get a database session."""
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


@app.get("/", response_class=HTMLResponse)
async def read_root(request: Request):
    """Serve the main web UI."""
    return templates.TemplateResponse("index.html", {"request": request})


@app.post("/commands/", response_model=schemas.Command)
def create_command(command: schemas.CommandCreate, db: Session = Depends(get_db)):
    """Create a new command in the database."""
    return crud.create_command(db=db, command=command)


@app.get("/commands/", response_model=list[schemas.Command])
def get_commands(
    q: str | None = None,
    namespace: str | None = None,
    user: str | None = None,
    scope: str | None = None,
    db: Session = Depends(get_db),
):
    """Get all commands, with optional filtering."""
    return crud.get_commands(db=db, q=q, namespace=namespace, user=user, scope=scope)


@app.post("/commands/recall", response_model=schemas.Command)
def recall_command(
    recall_request: schemas.RecallRequest, db: Session = Depends(get_db)
):
    """
    Recall the most relevant command based on context and increment its usage stats.
    """
    db_command = crud.recall_command(
        db=db,
        name=recall_request.name,
        namespace=recall_request.namespace,
        user=recall_request.user,
        hostname=recall_request.hostname,
        cwd=recall_request.cwd,
    )
    if db_command is None:
        raise HTTPException(status_code=404, detail="Command not found")
    return db_command


@app.get("/namespaces/", response_model=list[str])
def get_namespaces(db: Session = Depends(get_db)):
    """Get a list of all unique namespaces."""
    namespaces = crud.get_namespaces(db=db)
    return [item[0] for item in namespaces]


@app.get("/commands/all", response_model=list[schemas.Command])
def get_all_user_commands(user: str, db: Session = Depends(get_db)):
    """Get all commands for a specific user."""
    return crud.get_all_user_commands(db=db, user=user)


@app.put("/commands/{command_id}", response_model=schemas.Command)
def update_command(
    command_id: int,
    user: str,
    command_update: schemas.CommandUpdate,
    db: Session = Depends(get_db),
):
    """Update a command's command_string, ensuring user owns it."""
    db_command = crud.update_command(
        db=db, command_id=command_id, user=user, command_update=command_update
    )
    if db_command is None:
        raise HTTPException(
            status_code=404, detail="Command not found or user does not have permission"
        )
    return db_command


@app.patch("/commands/{command_id}", response_model=schemas.Command)
def rename_command(
    command_id: int,
    user: str,
    command_rename: schemas.CommandRename,
    db: Session = Depends(get_db),
):
    """Rename a command's name and namespace, ensuring it is owned by the user."""
    db_command = crud.rename_command(
        db=db, command_id=command_id, user=user, command_rename=command_rename
    )
    if db_command is None:
        raise HTTPException(
            status_code=404, detail="Command not found or user does not have permission"
        )
    return db_command


@app.delete("/commands/{command_id}", response_model=schemas.Command)
def delete_command(command_id: int, user: str, db: Session = Depends(get_db)):
    """Delete a command, ensuring it is owned by the user."""
    db_command = crud.delete_command(db=db, command_id=command_id, user=user)
    if db_command is None:
        raise HTTPException(
            status_code=404, detail="Command not found or user does not have permission"
        )
    return db_command


@app.get("/commands/by-id/{command_id}", response_model=schemas.Command)
def get_command_by_id(command_id: int, user: str, db: Session = Depends(get_db)):
    """Get a single command by ID, ensuring user has access to it."""
    db_command = crud.get_command_by_id(db=db, command_id=command_id, user=user)
    if db_command is None:
        raise HTTPException(
            status_code=404, detail="Command not found or user does not have permission"
        )
    return db_command


@app.post("/commands/{command_id}/execute", response_model=schemas.Command)
def track_command_execution(command_id: int, user: str, db: Session = Depends(get_db)):
    """Track command execution and return the command for execution."""
    db_command = crud.track_execution(db=db, command_id=command_id, user=user)
    if db_command is None:
        raise HTTPException(
            status_code=404, detail="Command not found or user does not have permission"
        )
    return db_command


@app.post("/commands/recall-by-name", response_model=schemas.Command)
def recall_command_by_name(
    recall_request: schemas.RecallByNameRequest, db: Session = Depends(get_db)
):
    """Recall command by name with enhanced contextual matching."""
    db_command = crud.recall_command_by_name(
        db=db,
        name=recall_request.name,
        user=recall_request.user,
        hostname=recall_request.hostname,
        cwd=recall_request.cwd,
        namespace_hint=recall_request.namespace_hint,
        scope_hint=recall_request.scope_hint,
    )
    if db_command is None:
        raise HTTPException(status_code=404, detail="Command not found")
    return db_command


@app.post("/suggestions", response_model=list[schemas.Command])
def get_contextual_suggestions(
    suggestions_request: schemas.SuggestionsRequest, db: Session = Depends(get_db)
):
    """Get intelligent command suggestions based on current context."""
    suggestions = crud.get_suggestions_for_context(
        db=db,
        user=suggestions_request.user,
        hostname=suggestions_request.hostname,
        cwd=suggestions_request.cwd,
        project_type=suggestions_request.project_type,
        limit=suggestions_request.limit,
    )
    return suggestions


@app.post("/project-context", response_model=schemas.ProjectContextResponse)
def detect_project_context(
    context_request: schemas.ProjectContextRequest, db: Session = Depends(get_db)
):
    """Detect project context and suggest appropriate namespace."""
    return crud.detect_project_context(
        db=db, directory_path=context_request.directory_path, user=context_request.user
    )


@app.get("/commands/{command_id}/similar", response_model=list[schemas.Command])
def get_similar_commands(
    command_id: int, limit: int = 5, db: Session = Depends(get_db)
):
    """Get commands similar to the specified command."""
    similar_commands = crud.get_command_similarity(
        db=db, command_id=command_id, limit=limit
    )
    return similar_commands


@app.post("/execution-history", response_model=dict)
def create_execution_record(
    execution: schemas.ExecutionHistoryCreate, db: Session = Depends(get_db)
):
    """Create an execution history record for analytics."""
    db_execution = crud.create_execution_history(db=db, execution=execution)
    return {"id": db_execution.id, "created": True}


@app.get("/analytics/execution", response_model=dict)
def get_execution_analytics(
    user: str | None = None, days: int = 30, db: Session = Depends(get_db)
):
    """Get execution analytics and insights."""
    analytics = crud.get_execution_analytics(db=db, user=user, days=days)
    return analytics
