"""
Status code DSL parser.

Supported syntax:
  200         — exact code
  2xx         — any 2xx code (200-299)
  !5xx        — any code that is NOT 5xx
  200|404     — 200 OR 404
  2xx|!503    — any 2xx OR (not 503)
"""

from __future__ import annotations

import re

_RANGE_RE = re.compile(r"^([2-5])xx$")
_CODE_RE = re.compile(r"^\d{3}$")


def _parse_term(term: str) -> tuple[bool, str]:
    """Parse a single term. Returns (negated, pattern)."""
    negated = False
    if term.startswith("!"):
        negated = True
        term = term[1:]
    if not (_RANGE_RE.match(term) or _CODE_RE.match(term)):
        raise ValueError(
            f"Invalid status code term: '{term}'. "
            "Expected a 3-digit code (e.g. 200), a range (e.g. 2xx), "
            "optionally prefixed with ! for negation."
        )
    return negated, term


def parse_status_expression(expr: str) -> list[tuple[bool, str]]:
    """Parse a full expression and return list of (negated, pattern) terms."""
    terms = [t.strip() for t in expr.split("|") if t.strip()]
    if not terms:
        raise ValueError("Status code expression must not be empty")
    return [_parse_term(t) for t in terms]


def _matches_pattern(code: int, pattern: str) -> bool:
    m = _RANGE_RE.match(pattern)
    if m:
        hundreds = int(m.group(1)) * 100
        return hundreds <= code < hundreds + 100
    return code == int(pattern)


def matches_status_code(code: int, expr: str) -> bool:
    """Return True if the HTTP status code satisfies the DSL expression."""
    try:
        terms = parse_status_expression(expr)
    except ValueError:
        return False

    for negated, pattern in terms:
        result = _matches_pattern(code, pattern)
        if negated:
            result = not result
        if result:
            return True
    return False


def matches_body(body: str, match_type: str | None, match_value: str | None) -> bool:
    """Return True if body satisfies the body matching rule."""
    if match_type is None or match_value is None:
        return True
    if match_type == "contains":
        return match_value in body
    if match_type == "equals":
        return body == match_value
    if match_type == "not_equals":
        return body != match_value
    return True
