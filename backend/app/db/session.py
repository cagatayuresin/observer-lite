"""Async SQLAlchemy engine, session factory, and ORM base class.

SQLite-specific pragmas are applied on every new connection via an event
listener so that WAL mode, foreign key enforcement, and the 64 MB page cache
are always active regardless of how the connection was acquired.
"""

from collections.abc import AsyncGenerator

from sqlalchemy import event
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase

from app.config import get_settings

settings = get_settings()

engine = create_async_engine(
    settings.database_url,
    echo=settings.debug,
    connect_args={"check_same_thread": False},
)


@event.listens_for(engine.sync_engine, "connect")
def _set_sqlite_pragmas(dbapi_conn, _):
    """Apply performance and correctness pragmas to every new SQLite connection.

    ``journal_mode=WAL`` allows concurrent reads during a write transaction.
    ``synchronous=NORMAL`` is safe with WAL and much faster than FULL.
    ``foreign_keys=ON`` enforces referential integrity at the DB level.
    ``cache_size=-65536`` pins 64 MB of pages in the SQLite page cache.
    ``temp_store=MEMORY`` keeps temp tables in RAM instead of on disk.
    """
    cursor = dbapi_conn.cursor()
    cursor.execute("PRAGMA journal_mode=WAL")
    cursor.execute("PRAGMA synchronous=NORMAL")
    cursor.execute("PRAGMA foreign_keys=ON")
    cursor.execute("PRAGMA cache_size=-65536")  # 64 MB
    cursor.execute("PRAGMA temp_store=MEMORY")
    cursor.close()


AsyncSessionLocal = async_sessionmaker(engine, expire_on_commit=False)


class Base(DeclarativeBase):
    """Shared declarative base for all ORM models."""


async def get_db() -> AsyncGenerator[AsyncSession, None]:
    """FastAPI dependency that yields an :class:`AsyncSession` per request.

    The session is automatically committed or rolled back by the context
    manager; callers should commit explicitly before the response is sent.

    Yields:
        An open :class:`sqlalchemy.ext.asyncio.AsyncSession`.
    """
    async with AsyncSessionLocal() as session:
        yield session
