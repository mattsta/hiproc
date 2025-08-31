"""
Database configuration and session management for the hiproc server.

This module sets up the SQLAlchemy engine and session factory for the SQLite
database. It uses a file-based database named `hiproc.db` in the project root.
"""

from sqlalchemy import create_engine
from sqlalchemy.orm import declarative_base, sessionmaker

SQLALCHEMY_DATABASE_URL = "sqlite:///./hiproc.db"

engine = create_engine(
    SQLALCHEMY_DATABASE_URL, connect_args={"check_same_thread": False}
)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

Base = declarative_base()
