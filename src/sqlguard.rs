//! Guards for the arbitrary SQL endpoint: only single, read-only statements.

use anyhow::bail;

const ALLOWED_START: [&str; 6] = ["select", "with", "show", "describe", "explain", "summarize"];

const FORBIDDEN: &[&str] = &[
    "insert",
    "update",
    "delete",
    "replace",
    "merge",
    "drop",
    "create",
    "alter",
    "truncate",
    "attach",
    "detach",
    "copy",
    "export",
    "import",
    "call",
    "pragma",
    "set",
    "install",
    "load",
    "use",
    "checkpoint",
    "vacuum",
    "analyze",
    "analyse",
    "grant",
    "revoke",
    "begin",
    "commit",
    "rollback",
    "prepare",
    "execute",
    "comment",
    "snapshot",
    "restore",
];

/// Validate that `sql` is a single read-only statement. Returns the cleaned
/// statement (trailing semicolon removed).
pub fn validate_readonly(sql: &str) -> anyhow::Result<String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        bail!("empty query");
    }
    let stripped = strip_literals(trimmed);

    let stmts = stripped.split(';').filter(|s| !s.trim().is_empty()).count();
    if stmts > 1 {
        bail!("only a single statement is allowed");
    }

    let first = stripped
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ALLOWED_START.contains(&first.as_str()) {
        bail!(
            "only read-only queries are allowed (SELECT, WITH, SHOW, DESCRIBE, EXPLAIN, SUMMARIZE), got '{first}'"
        );
    }

    let lower = stripped.to_ascii_lowercase();
    for word in FORBIDDEN {
        if contains_word(&lower, word) {
            bail!("forbidden keyword '{word}' in query");
        }
    }

    Ok(trimmed.trim_end_matches(';').trim_end().to_string())
}

/// Append a LIMIT to unbounded SELECT/WITH queries so the console cannot pull
/// unbounded rows.
pub fn maybe_add_limit(sql: &str, limit: usize) -> String {
    let stripped = strip_literals(sql);
    let lower = stripped.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or("");
    if (first == "select" || first == "with") && !contains_word(&lower, "limit") {
        format!("SELECT * FROM ({sql}) _otv_sub LIMIT {limit}")
    } else {
        sql.to_string()
    }
}

/// Replace string literals and comments with spaces so keyword scanning cannot
/// be confused by their contents.
fn strip_literals(sql: &str) -> String {
    let cs: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\'' | '"' | '`' => {
                let q = c;
                i += 1;
                while i < cs.len() {
                    if cs[i] == q {
                        if i + 1 < cs.len() && cs[i + 1] == q {
                            i += 2; // escaped quote ('' or "")
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push(' ');
            }
            '-' if i + 1 < cs.len() && cs[i + 1] == '-' => {
                while i < cs.len() && cs[i] != '\n' {
                    i += 1;
                }
                out.push('\n');
            }
            '/' if i + 1 < cs.len() && cs[i + 1] == '*' => {
                i += 2;
                while i + 1 < cs.len() && !(cs[i] == '*' && cs[i + 1] == '/') {
                    i += 1;
                }
                i = (i + 2).min(cs.len());
                out.push(' ');
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Word-boundary substring check (no regex dependency).
fn contains_word(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return false;
    }
    let mut i = 0;
    while i + n.len() <= h.len() {
        if &h[i..i + n.len()] == n {
            let before = i == 0 || !is_word(h[i - 1]);
            let after = i + n.len() == h.len() || !is_word(h[i + n.len()]);
            if before && after {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_selects() {
        assert!(validate_readonly("SELECT 1").is_ok());
        assert!(validate_readonly("select * from spans;").is_ok());
        assert!(validate_readonly("WITH t AS (SELECT 1) SELECT * FROM t").is_ok());
        assert!(validate_readonly("SHOW TABLES").is_ok());
        assert!(validate_readonly("DESCRIBE spans").is_ok());
        assert!(validate_readonly("SUMMARIZE spans").is_ok());
        // keywords inside identifiers or literals must be fine
        assert!(validate_readonly("SELECT created_at, drop_me FROM t").is_ok());
        assert!(validate_readonly("SELECT 'drop table users' AS s").is_ok());
        assert!(validate_readonly("select 'set'").is_ok());
    }

    #[test]
    fn rejects_writes() {
        assert!(validate_readonly("DELETE FROM spans").is_err());
        assert!(validate_readonly("DROP TABLE spans").is_err());
        assert!(validate_readonly("select 1; drop table spans").is_err());
        assert!(validate_readonly("select 1; select 2").is_err());
        assert!(validate_readonly("INSERT INTO t VALUES (1)").is_err());
        // WITH ... INSERT smuggling
        assert!(validate_readonly("WITH t AS (SELECT 1) INSERT INTO t SELECT * FROM t").is_err());
        assert!(validate_readonly("COPY t TO 'x.csv'").is_err());
        assert!(validate_readonly("").is_err());
    }

    #[test]
    fn adds_limit() {
        assert_eq!(
            maybe_add_limit("SELECT * FROM spans", 10),
            "SELECT * FROM (SELECT * FROM spans) _otv_sub LIMIT 10"
        );
        assert_eq!(
            maybe_add_limit("SELECT * FROM spans LIMIT 3", 10),
            "SELECT * FROM spans LIMIT 3"
        );
        assert_eq!(maybe_add_limit("SHOW TABLES", 10), "SHOW TABLES");
        // limit inside a literal does not count
        let q = "SELECT 'limit' AS x FROM t";
        assert_eq!(
            maybe_add_limit(q, 7),
            format!("SELECT * FROM ({q}) _otv_sub LIMIT 7")
        );
    }
}
