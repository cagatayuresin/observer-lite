"""Simple pagination helper used by list endpoints."""

from dataclasses import dataclass


@dataclass
class PaginationParams:
    """Encapsulates page/per_page parameters and computes SQL offset/limit.

    Attributes:
        page: 1-based page number.
        per_page: Maximum number of items per page.
    """
    page: int = 1
    per_page: int = 50

    @property
    def offset(self) -> int:
        return (self.page - 1) * self.per_page

    @property
    def limit(self) -> int:
        return self.per_page
