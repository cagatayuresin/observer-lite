//! Status-code DSL and response body matching.
//!
//! `2xx` is 200–299, `!503` negates one term, and `|` is OR. The first matching
//! term wins, so `2xx|!503` is also true for 404.

#[derive(Debug, PartialEq, Eq)]
pub struct StatusTerm {
    pub negated: bool,
    pub pattern: String,
}

pub fn parse_status_expression(expr: &str) -> Result<Vec<StatusTerm>, String> {
    let terms: Vec<&str> = expr
        .split('|')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    if terms.is_empty() {
        return Err("Status code expression must not be empty".into());
    }
    terms.into_iter().map(parse_term).collect()
}

fn parse_term(term: &str) -> Result<StatusTerm, String> {
    let (negated, rest) = if let Some(stripped) = term.strip_prefix('!') {
        (true, stripped)
    } else {
        (false, term)
    };
    if !is_range(rest) && !is_code(rest) {
        return Err(format!(
            "Invalid status code term: '{rest}'. Expected a 3-digit code (e.g. 200), a range (e.g. 2xx), optionally prefixed with ! for negation."
        ));
    }
    Ok(StatusTerm {
        negated,
        pattern: rest.to_string(),
    })
}

fn is_range(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 3
        && matches!(bytes[0], b'2' | b'3' | b'4' | b'5')
        && bytes[1] == b'x'
        && bytes[2] == b'x'
}

fn is_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|b| b.is_ascii_digit())
}

fn matches_pattern(code: i32, pattern: &str) -> bool {
    if is_range(pattern) {
        let hundreds = (pattern.as_bytes()[0] - b'0') as i32 * 100;
        return code >= hundreds && code < hundreds + 100;
    }
    pattern.parse::<i32>().ok() == Some(code)
}

pub fn matches_status_code(code: i32, expr: &str) -> bool {
    let terms = match parse_status_expression(expr) {
        Ok(terms) => terms,
        Err(_) => return false,
    };
    for term in terms {
        let mut result = matches_pattern(code, &term.pattern);
        if term.negated {
            result = !result;
        }
        if result {
            return true;
        }
    }
    false
}

pub fn matches_body(body: &str, match_type: Option<&str>, match_value: Option<&str>) -> bool {
    match (match_type, match_value) {
        (None, _) | (_, None) => true,
        (Some("contains"), Some(value)) => body.contains(value),
        (Some("equals"), Some(value)) => body == value,
        (Some("not_equals"), Some(value)) => body != value,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_dsl() {
        assert!(matches_status_code(204, "2xx"));
        assert!(!matches_status_code(301, "2xx"));
        assert!(matches_status_code(200, "200"));
        assert!(matches_status_code(301, "2xx|301"));
        assert!(matches_status_code(200, "!5xx"));
        assert!(!matches_status_code(503, "!5xx"));
        assert!(matches_status_code(200, "2xx|!503"));
        assert!(matches_status_code(404, "2xx|!503"));
        assert!(!matches_status_code(503, "2xx|!503"));
        assert!(!matches_status_code(200, "nope"));
        assert!(parse_status_expression("").is_err());
        assert!(parse_status_expression("!").is_err());
        assert!(parse_status_expression("99").is_err());
        assert!(parse_status_expression("1xx").is_err());
    }

    #[test]
    fn body_rules() {
        assert!(matches_body("abc", None, Some("x")));
        assert!(matches_body("hello world", Some("contains"), Some("world")));
        assert!(!matches_body("hello", Some("contains"), Some("world")));
        assert!(matches_body("ok", Some("equals"), Some("ok")));
        assert!(matches_body("ok", Some("not_equals"), Some("no")));
        assert!(!matches_body("ok", Some("not_equals"), Some("ok")));
        assert!(matches_body("ok", Some("regex"), Some("ok")));
    }
}
