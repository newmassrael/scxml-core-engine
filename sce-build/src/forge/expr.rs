// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Forge expression transpiler — ECMAScript subset to target language.
//
// Parses the ECMAScript expression subset defined in SCE_FORGE.md Section 3.4
// and transpiles to equivalent target-language code. This is an AOT operation:
// expressions are compile-time constants, not runtime-evaluated.
//
// All transformations protect string literal contents from modification.

use std::sync::LazyLock;

/// Target language for expression transpilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprTarget {
    Cpp,
    Kotlin,
    Rust,
    Go,
    Python,
}

/// Transpile an ECMAScript expression to the target language.
///
/// Handles the supported subset from SCE_FORGE.md Section 3.4:
/// arithmetic, comparison, logical, bitwise, shift, ternary,
/// member access, call expressions, literals.
pub fn transpile(expr: &str, target: ExprTarget) -> Result<String, String> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err("Empty expression".to_string());
    }

    validate_expression(expr)?;

    match target {
        ExprTarget::Cpp => transpile_to_cpp(expr),
        ExprTarget::Kotlin => transpile_to_kotlin(expr),
        ExprTarget::Rust => transpile_to_rust(expr),
        ExprTarget::Go => transpile_to_go(expr),
        ExprTarget::Python => transpile_to_python(expr),
    }
}

// ── Validation ─────────────────────────────────────────────────

static RE_UNSUPPORTED: LazyLock<Vec<(regex::Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (regex::Regex::new(r"\bnew\b").unwrap(), "new"),
        (regex::Regex::new(r"\bdelete\b").unwrap(), "delete"),
        (regex::Regex::new(r"\btypeof\b").unwrap(), "typeof"),
        (regex::Regex::new(r"\binstanceof\b").unwrap(), "instanceof"),
        (regex::Regex::new(r"=>").unwrap(), "arrow function"),
        (regex::Regex::new(r"\bthis\b").unwrap(), "this"),
        (regex::Regex::new(r"\beval\b\s*\(").unwrap(), "eval()"),
        (regex::Regex::new(r"\basync\b").unwrap(), "async"),
        (regex::Regex::new(r"\bawait\b").unwrap(), "await"),
        (regex::Regex::new(r"\byield\b").unwrap(), "yield"),
        (regex::Regex::new(r"`").unwrap(), "template literal"),
        (regex::Regex::new(r"\.\.\.\w").unwrap(), "spread/rest"),
        (regex::Regex::new(r"\?\.\s*\w").unwrap(), "optional chaining"),
        (regex::Regex::new(r"\?\?").unwrap(), "nullish coalescing"),
        (regex::Regex::new(r"[^=!]==[^=]").unwrap(), "loose equality (==); use === instead"),
    ]
});

fn validate_expression(expr: &str) -> Result<(), String> {
    let stripped = strip_string_literals(expr);

    for (re, construct) in RE_UNSUPPORTED.iter() {
        if re.is_match(&stripped) {
            return Err(format!(
                "Unsupported ECMAScript construct: {construct}. \
                 Extended SCXML expressions must use the stateless subset."
            ));
        }
    }
    Ok(())
}

// ── String literal protection ──────────────────────────────────
//
// All transformations must protect string literal contents.
// Strategy: extract literals -> transform code -> restore literals.

/// Sentinel prefix for string literal placeholders (unlikely to appear in real code).
const SENTINEL: &str = "\x00STR";

/// Extract string literals, replacing them with indexed sentinels.
/// Returns (modified_expr, extracted_literals).
fn extract_string_literals(expr: &str) -> (String, Vec<String>) {
    let mut result = String::with_capacity(expr.len());
    let mut literals = Vec::new();
    let bytes = expr.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // closing quote
            }
            let literal = &expr[start..i];
            result.push_str(&format!("{SENTINEL}{}", literals.len()));
            literals.push(literal.to_string());
        } else {
            result.push(expr[i..].chars().next().unwrap());
            i += expr[i..].chars().next().unwrap().len_utf8();
        }
    }

    (result, literals)
}

/// Restore string literals from sentinels.
fn restore_string_literals(expr: &str, literals: &[String]) -> String {
    let mut result = expr.to_string();
    for (idx, lit) in literals.iter().enumerate() {
        result = result.replace(&format!("{SENTINEL}{idx}"), lit);
    }
    result
}

/// Replace string literal contents with spaces (for validation only).
fn strip_string_literals(expr: &str) -> String {
    static RE_STR: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"'[^']*'|"[^"]*""#).unwrap()
    });
    RE_STR
        .replace_all(expr, |caps: &regex::Captures| " ".repeat(caps[0].len()))
        .to_string()
}

// ── C++ transpilation ──────────────────────────────────────────

fn transpile_to_cpp(expr: &str) -> Result<String, String> {
    let (mut code, literals) = extract_string_literals(expr);

    code = code.replace("===", "==");
    code = code.replace("!==", "!=");

    // Restore literals, then convert single-quoted to double-quoted
    let result = restore_string_literals(&code, &literals);
    Ok(convert_single_to_double_quotes(&result))
}

// ── Kotlin transpilation ───────────────────────────────────────

fn transpile_to_kotlin(expr: &str) -> Result<String, String> {
    let (mut code, literals) = extract_string_literals(expr);

    code = code.replace("===", "==");
    code = code.replace("!==", "!=");

    let result = restore_string_literals(&code, &literals);
    Ok(convert_single_to_double_quotes(&result))
}

// ── Rust transpilation ─────────────────────────────────────────

fn transpile_to_rust(expr: &str) -> Result<String, String> {
    let (mut code, literals) = extract_string_literals(expr);

    code = code.replace("===", "==");
    code = code.replace("!==", "!=");

    // Ternary: a ? b : c -> if a { b } else { c }
    code = convert_ternary_to_if_else(&code);

    let result = restore_string_literals(&code, &literals);
    Ok(convert_single_to_double_quotes(&result))
}

// ── Go transpilation ───────────────────────────────────────────

fn transpile_to_go(expr: &str) -> Result<String, String> {
    let (mut code, literals) = extract_string_literals(expr);

    code = code.replace("===", "==");
    code = code.replace("!==", "!=");

    // Go has no ternary operator — reject expressions that use it
    if split_ternary(&code).is_some() {
        return Err(
            "Go does not support ternary expressions. \
             Refactor the SCXML expression to avoid `? :` for Go targets."
                .to_string(),
        );
    }

    let result = restore_string_literals(&code, &literals);
    Ok(convert_single_to_double_quotes(&result))
}

// ── Python transpilation ───────────────────────────────────────

fn transpile_to_python(expr: &str) -> Result<String, String> {
    let (mut code, literals) = extract_string_literals(expr);

    code = code.replace("===", "==");
    code = code.replace("!==", "!=");
    code = code.replace("&&", "and");
    code = code.replace("||", "or");

    // Prefix ! -> not (sentinels don't start with a-zA-Z_, so no false matches)
    static RE_NOT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"!([a-zA-Z_])").unwrap());
    code = RE_NOT
        .replace_all(&code, |caps: &regex::Captures| format!("not {}", &caps[1]))
        .to_string();

    // true -> True, false -> False
    static RE_TRUE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\btrue\b").unwrap());
    static RE_FALSE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\bfalse\b").unwrap());
    code = RE_TRUE.replace_all(&code, "True").to_string();
    code = RE_FALSE.replace_all(&code, "False").to_string();

    let result = restore_string_literals(&code, &literals);
    Ok(result) // Python uses single quotes natively, no conversion needed
}

// ── Shared helpers ─────────────────────────────────────────────

/// Convert 'single-quoted' string literals to "double-quoted".
fn convert_single_to_double_quotes(expr: &str) -> String {
    static RE_SINGLE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"'([^']*)'").unwrap());
    RE_SINGLE
        .replace_all(expr, |caps: &regex::Captures| format!("\"{}\"", &caps[1]))
        .to_string()
}

/// Convert ECMAScript ternary `a ? b : c` to Rust `if a { b } else { c }`.
fn convert_ternary_to_if_else(expr: &str) -> String {
    if let Some((cond, rest)) = split_ternary(expr) {
        if let Some((then_expr, else_expr)) = split_colon(rest) {
            return format!(
                "if {} {{ {} }} else {{ {} }}",
                cond.trim(),
                then_expr.trim(),
                else_expr.trim()
            );
        }
    }
    expr.to_string()
}

/// Split a ternary expression at the top-level `?`.
fn split_ternary(expr: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'?' if depth == 0 => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    i += 1;
                    continue;
                }
                return Some((&expr[..i], &expr[i + 1..]));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split at the top-level `:` for ternary else branch.
fn split_colon(expr: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b':' if depth == 0 => {
                return Some((&expr[..i], &expr[i + 1..]));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_arithmetic_cpp() {
        let result = transpile("raw * 0.1 - 40.0", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "raw * 0.1 - 40.0");
    }

    #[test]
    fn test_transpile_strict_equality_cpp() {
        let result = transpile("status === 'OK'", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "status == \"OK\"");
    }

    #[test]
    fn test_transpile_logical_cpp() {
        let result = transpile("engineStop && ignOn", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "engineStop && ignOn");
    }

    #[test]
    fn test_transpile_comparison_rust() {
        let result = transpile("rpm > 8000", ExprTarget::Rust).unwrap();
        assert_eq!(result, "rpm > 8000");
    }

    #[test]
    fn test_transpile_ternary_rust() {
        let result = transpile("status === 'OK' ? 1 : 0", ExprTarget::Rust).unwrap();
        assert_eq!(result, "if status == \"OK\" { 1 } else { 0 }");
    }

    #[test]
    fn test_transpile_logical_python() {
        let result = transpile("engineStop && ignOn", ExprTarget::Python).unwrap();
        assert_eq!(result, "engineStop and ignOn");
    }

    #[test]
    fn test_transpile_booleans_python() {
        let result =
            transpile("ignition === true && engineStop === false", ExprTarget::Python).unwrap();
        assert_eq!(result, "ignition == True and engineStop == False");
    }

    #[test]
    fn test_reject_arrow_function() {
        let result = transpile("() => x + 1", ExprTarget::Cpp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("arrow function"));
    }

    #[test]
    fn test_reject_new() {
        let result = transpile("new Map()", ExprTarget::Cpp);
        assert!(result.is_err());
    }

    #[test]
    fn test_bitwise_cpp() {
        let result = transpile("raw & 0x0F", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "raw & 0x0F");
    }

    #[test]
    fn test_shift_cpp() {
        let result = transpile("(raw[1] >> 4) & 0x0F", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "(raw[1] >> 4) & 0x0F");
    }

    #[test]
    fn test_string_literal_not_flagged() {
        let result = transpile("status === 'new'", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "status == \"new\"");
    }

    // Regression: string literal contents must not be transformed
    #[test]
    fn test_string_literal_contents_preserved() {
        let result = transpile("x === 'a === b'", ExprTarget::Cpp).unwrap();
        assert_eq!(result, "x == \"a === b\"");
    }

    #[test]
    fn test_go_rejects_ternary() {
        let result = transpile("x > 0 ? 1 : 0", ExprTarget::Go);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ternary"));
    }
}
