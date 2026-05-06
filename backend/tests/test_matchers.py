"""Unit tests for the status code DSL parser (matchers.py)."""

import pytest

from app.checkers.matchers import (
    matches_body,
    matches_status_code,
    parse_status_expression,
)


class TestParseStatusExpression:
    def test_exact_code(self):
        terms = parse_status_expression("200")
        assert terms == [(False, "200")]

    def test_range(self):
        terms = parse_status_expression("2xx")
        assert terms == [(False, "2xx")]

    def test_negated_range(self):
        terms = parse_status_expression("!5xx")
        assert terms == [(True, "5xx")]

    def test_or_expression(self):
        terms = parse_status_expression("200|404")
        assert len(terms) == 2
        assert (False, "200") in terms
        assert (False, "404") in terms

    def test_mixed_or(self):
        terms = parse_status_expression("2xx|!503")
        assert (False, "2xx") in terms
        assert (True, "503") in terms

    def test_whitespace_stripped(self):
        terms = parse_status_expression(" 200 | 404 ")
        assert (False, "200") in terms
        assert (False, "404") in terms

    def test_empty_expression_raises(self):
        with pytest.raises(ValueError):
            parse_status_expression("")

    def test_invalid_term_raises(self):
        with pytest.raises(ValueError, match="Invalid status code term"):
            parse_status_expression("ok")

    def test_two_digit_code_raises(self):
        with pytest.raises(ValueError):
            parse_status_expression("20")

    def test_invalid_range_letter_raises(self):
        with pytest.raises(ValueError):
            parse_status_expression("1xx")


class TestMatchesStatusCode:
    # --- exact codes ---
    def test_exact_match(self):
        assert matches_status_code(200, "200") is True

    def test_exact_no_match(self):
        assert matches_status_code(201, "200") is False

    # --- range ---
    def test_2xx_match(self):
        for code in (200, 201, 204, 299):
            assert matches_status_code(code, "2xx") is True

    def test_2xx_no_match(self):
        assert matches_status_code(300, "2xx") is False
        assert matches_status_code(404, "2xx") is False

    def test_3xx_range(self):
        assert matches_status_code(301, "3xx") is True
        assert matches_status_code(200, "3xx") is False

    def test_4xx_range(self):
        assert matches_status_code(404, "4xx") is True

    def test_5xx_range(self):
        assert matches_status_code(500, "5xx") is True
        assert matches_status_code(503, "5xx") is True

    # --- negation ---
    def test_negated_5xx_rejects_500(self):
        assert matches_status_code(500, "!5xx") is False

    def test_negated_5xx_accepts_200(self):
        assert matches_status_code(200, "!5xx") is True

    def test_negated_exact_code(self):
        assert matches_status_code(503, "!503") is False
        assert matches_status_code(200, "!503") is True

    # --- OR ---
    def test_or_first_matches(self):
        assert matches_status_code(200, "200|404") is True

    def test_or_second_matches(self):
        assert matches_status_code(404, "200|404") is True

    def test_or_neither_matches(self):
        assert matches_status_code(500, "200|404") is False

    # --- complex ---
    def test_2xx_or_not_503(self):
        # 200 is 2xx → True
        assert matches_status_code(200, "2xx|!503") is True
        # 404 is not 2xx, but !503 is True → True
        assert matches_status_code(404, "2xx|!503") is True
        # 503 is not 2xx, and !503 is False → False
        assert matches_status_code(503, "2xx|!503") is False

    def test_invalid_expression_returns_false(self):
        # matches_status_code swallows ValueError from parse_status_expression
        assert matches_status_code(200, "") is False


class TestMatchesBody:
    def test_no_rule_always_true(self):
        assert matches_body("anything", None, None) is True

    def test_contains_match(self):
        assert matches_body("hello world", "contains", "world") is True

    def test_contains_no_match(self):
        assert matches_body("hello", "contains", "world") is False

    def test_equals_match(self):
        assert matches_body("ok", "equals", "ok") is True

    def test_equals_no_match(self):
        assert matches_body("ok!", "equals", "ok") is False

    def test_not_equals_match(self):
        assert matches_body("error", "not_equals", "ok") is True

    def test_not_equals_no_match(self):
        assert matches_body("ok", "not_equals", "ok") is False

    def test_unknown_type_returns_true(self):
        assert matches_body("body", "regex", ".*") is True

    def test_none_value_returns_true(self):
        assert matches_body("body", "contains", None) is True
