// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// ECMAScript-to-Lua expression transformer.
//
// 1:1 port of Python ecmascript_to_lua.py (which was a 1:1 port of C++).
// Transforms the ECMAScript subset used in W3C SCXML tests into Lua 5.4.
//
// Used at codegen time so that generated Rust embeds Lua-ready literals.
// The Rust LuaEngine just compiles and executes Lua — it does not know
// about JavaScript syntax.

/// How the expression is used — affects truthiness handling.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExpressionContext {
    /// Assignment, log, data init — no truthiness wrapping.
    General,
    /// Transition cond, if/elseif — may need truthiness wrapping.
    Guard,
}

// ── Helper functions ────────────────────────────────────────────────

fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Push the UTF-8 character starting at `bytes[pos]` to `output`.
/// Returns the number of bytes consumed (1 for ASCII, 2-4 for multi-byte).
/// Prevents Non-ASCII corruption from `byte as char` casting.
fn push_utf8_char(output: &mut String, bytes: &[u8], pos: usize) -> usize {
    let b = bytes[pos];
    if b < 0x80 {
        output.push(b as char);
        return 1;
    }
    let width = match b {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    };
    let end = (pos + width).min(bytes.len());
    match std::str::from_utf8(&bytes[pos..end]) {
        Ok(s) => output.push_str(s),
        Err(_) => output.push(char::REPLACEMENT_CHARACTER),
    }
    end - pos
}

/// Find keyword with word boundaries (`\bword\b`). Returns `None` if not found.
fn find_word(s: &[u8], word: &[u8], start: usize) -> Option<usize> {
    let wlen = word.len();
    let slen = s.len();
    let mut pos = start;
    loop {
        if pos + wlen > slen {
            return None;
        }
        match s[pos..].windows(wlen).position(|w| w == word) {
            None => return None,
            Some(offset) => {
                let p = pos + offset;
                let left_ok = p == 0 || !is_word_char(s[p - 1]);
                let right_ok = p + wlen >= slen || !is_word_char(s[p + wlen]);
                if left_ok && right_ok {
                    return Some(p);
                }
                pos = p + 1;
            }
        }
    }
}

/// Replace all word-bounded occurrences: `\bword\b` -> replacement.
fn replace_word(s: &str, word: &str, replacement: &str) -> String {
    let sb = s.as_bytes();
    let wb = word.as_bytes();
    let wlen = word.len();
    let mut result = String::with_capacity(s.len());
    let mut last = 0;
    let mut pos = 0;
    loop {
        match find_word(sb, wb, pos) {
            None => break,
            Some(p) => {
                result.push_str(&s[last..p]);
                result.push_str(replacement);
                last = p + wlen;
                pos = last;
            }
        }
    }
    result.push_str(&s[last..]);
    result
}

/// Replace word-bounded occurrences only at bracket depth 0.
fn replace_word_at_top_level(s: &str, word: &str, replacement: &str) -> String {
    let sb = s.as_bytes();
    let wlen = word.len();
    let wb = word.as_bytes();
    let mut result = String::with_capacity(s.len());
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < sb.len() {
        let c = sb[i];
        if c == b'{' || c == b'(' || c == b'[' {
            depth += 1;
            result.push(c as char);
            i += 1;
        } else if c == b'}' || c == b')' || c == b']' {
            if depth > 0 {
                depth -= 1;
            }
            result.push(c as char);
            i += 1;
        } else if depth == 0
            && i + wlen <= sb.len()
            && &sb[i..i + wlen] == wb
            && (i == 0 || !is_word_char(sb[i - 1]))
            && (i + wlen >= sb.len() || !is_word_char(sb[i + wlen]))
        {
            result.push_str(replacement);
            i += wlen;
        } else {
            i += push_utf8_char(&mut result, sb, i);
        }
    }
    result
}

/// Replace `\bkeyword\s+` -> replacement (for var/let/const removal).
fn replace_keyword_prefix(s: &str, keyword: &str, replacement: &str) -> String {
    let sb = s.as_bytes();
    let kwb = keyword.as_bytes();
    let kwlen = keyword.len();
    let mut result = String::with_capacity(s.len());
    let mut last = 0;
    let mut pos = 0;
    loop {
        match find_word(sb, kwb, pos) {
            None => break,
            Some(p) => {
                let after_kw = p + kwlen;
                if after_kw < sb.len() && sb[after_kw].is_ascii_whitespace() {
                    result.push_str(&s[last..p]);
                    result.push_str(replacement);
                    let mut space_end = after_kw;
                    while space_end < sb.len() && sb[space_end].is_ascii_whitespace() {
                        space_end += 1;
                    }
                    last = space_end;
                    pos = last;
                } else {
                    pos = p + 1;
                }
            }
        }
    }
    result.push_str(&s[last..]);
    result
}

fn skip_spaces(s: &[u8], mut pos: usize) -> usize {
    while pos < s.len() && s[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn read_word(s: &[u8], mut pos: usize) -> usize {
    while pos < s.len() && is_word_char(s[pos]) {
        pos += 1;
    }
    pos
}

fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c == ' ' || c == '\t')
}

fn find_matching_close(s: &[u8], open_pos: usize, open_ch: u8, close_ch: u8) -> usize {
    let mut depth: i32 = 1;
    let mut pos = open_pos + 1;
    while pos < s.len() && depth > 0 {
        if s[pos] == open_ch {
            depth += 1;
        } else if s[pos] == close_ch {
            depth -= 1;
        }
        if depth > 0 {
            pos += 1;
        }
    }
    if depth == 0 {
        pos
    } else if s.is_empty() {
        0
    } else {
        // Unmatched brackets: return last index (matches Python reference)
        s.len() - 1
    }
}

/// Find start of identifier preceding `dot_pos`. Returns `None` if none.
fn find_preceding_identifier(s: &[u8], dot_pos: usize) -> Option<usize> {
    if dot_pos == 0 {
        return None;
    }
    let end = dot_pos;
    let mut start = dot_pos;
    while start > 0 && is_word_char(s[start - 1]) {
        start -= 1;
    }
    if start < end {
        Some(start)
    } else {
        None
    }
}

/// Check if expression is a pure `In()` predicate chain.
fn is_pure_in_predicate(expr: &str) -> bool {
    let b = expr.as_bytes();
    let length = b.len();
    let mut i = 0;
    let mut found_in = false;
    while i < length {
        let c = b[i];
        if c.is_ascii_whitespace() || c == b'(' || c == b')' {
            i += 1;
            continue;
        }
        if i + 3 <= length && &b[i..i + 3] == b"In(" {
            found_in = true;
            i += 3;
            while i < length && b[i] != b')' {
                i += 1;
            }
            if i < length {
                i += 1;
            }
            continue;
        }
        if i + 3 <= length && &b[i..i + 3] == b"and" && (i + 3 >= length || !is_word_char(b[i + 3]))
        {
            i += 3;
            continue;
        }
        if i + 2 <= length && &b[i..i + 2] == b"or" && (i + 2 >= length || !is_word_char(b[i + 2]))
        {
            i += 2;
            continue;
        }
        return false;
    }
    found_in
}

// ── Stage 1: String Literal Protection ──────────────────────────────

fn protect_string_literals(input: &str) -> (String, Vec<String>) {
    let b = input.as_bytes();
    let length = b.len();
    let mut literals: Vec<String> = Vec::new();
    let mut output = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        let c = b[i];

        // Strip JS block comments /* ... */
        if c == b'/' && i + 1 < length && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < length && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < length {
                i += 1; // skip closing '/'
            }
            i += 1;
            continue;
        }

        // Strip JS line comments // ...
        if c == b'/' && i + 1 < length && b[i + 1] == b'/' {
            i += 2;
            while i < length && b[i] != b'\n' {
                i += 1;
            }
            if i < length {
                output.push('\n');
            }
            i += 1;
            continue;
        }

        if c == b'\'' || c == b'"' {
            let quote = c;
            let mut literal = String::new();
            literal.push(c as char); // quote char is ASCII
            i += 1;
            while i < length {
                if b[i] == b'\\' && i + 1 < length {
                    literal.push('\\');
                    i += 1;
                    i += push_utf8_char(&mut literal, b, i);
                    continue;
                }
                if b[i] == quote {
                    literal.push(b[i] as char); // closing quote is ASCII
                    break;
                }
                i += push_utf8_char(&mut literal, b, i);
            }

            let idx = literals.len();
            literals.push(literal);
            output.push('\x01');
            output.push_str("STR");
            output.push_str(&idx.to_string());
            output.push('\x01');
        } else {
            i += push_utf8_char(&mut output, b, i) - 1; // -1: outer loop does i += 1
        }
        i += 1;
    }

    (output, literals)
}

fn restore_string_literals(processed: &str, literals: &[String]) -> String {
    let mut result = processed.to_string();
    for (i, literal) in literals.iter().enumerate() {
        let placeholder = format!("\x01STR{}\x01", i);
        result = result.replace(&placeholder, literal);
    }
    result
}

// ── Pre-pass: typeof Transformation ─────────────────────────────────

fn transform_typeof_patterns(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        let typeof_pos = match input[i..].find("typeof") {
            Some(offset) => i + offset,
            None => {
                result.push_str(&input[i..]);
                break;
            }
        };

        // Check word boundary
        if typeof_pos > 0 && is_word_char(b[typeof_pos - 1]) {
            result.push_str(&input[i..typeof_pos + 6]);
            i = typeof_pos + 6;
            continue;
        }

        result.push_str(&input[i..typeof_pos]);

        let mut j = skip_spaces(b, typeof_pos + 6);

        let has_paren = j < length && b[j] == b'(';
        if has_paren {
            j = skip_spaces(b, j + 1);
        }

        let var_start = j;
        let var_end = read_word(b, j);

        if var_end == var_start {
            result.push_str("typeof");
            i = typeof_pos + 6;
            continue;
        }

        let var_name = &input[var_start..var_end];

        let mut after_typeof_expr = skip_spaces(b, var_end);
        if has_paren && after_typeof_expr < length && b[after_typeof_expr] == b')' {
            after_typeof_expr += 1;
        }

        // Check for comparison operator
        let k = skip_spaces(b, after_typeof_expr);
        let mut op_len: usize = 0;
        let mut is_neg = false;

        if k + 3 <= length && &b[k..k + 3] == b"!==" {
            op_len = 3;
            is_neg = true;
        } else if k + 3 <= length && &b[k..k + 3] == b"===" {
            op_len = 3;
        } else if k + 2 <= length && &b[k..k + 2] == b"!=" {
            op_len = 2;
            is_neg = true;
        } else if k + 2 <= length && &b[k..k + 2] == b"==" {
            op_len = 2;
        }

        if op_len > 0 {
            let mut m = skip_spaces(b, k + op_len);
            if m < length && (b[m] == b'\'' || b[m] == b'"') {
                let quote = b[m];
                m += 1;
                let type_start = m;
                while m < length && b[m] != quote {
                    m += 1;
                }
                let type_str = &input[type_start..m];
                if m < length {
                    m += 1;
                }

                let lua_op = if is_neg { "~=" } else { "==" };

                if type_str == "undefined" {
                    result.push_str(&format!("{} {} nil", var_name, lua_op));
                } else {
                    let lua_type = if type_str == "object" {
                        "table"
                    } else {
                        type_str
                    };
                    result.push_str(&format!("type({}) {} '{}'", var_name, lua_op, lua_type));
                }
                i = m;
                continue;
            }
        }

        // Standalone typeof -> _typeof(varName)
        result.push_str(&format!("_typeof({})", var_name));
        i = after_typeof_expr;
    }

    result
}

// ── Stage 2: Pattern Transformations ────────────────────────────────

fn transform_compound_assignment(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        let eq_pos = match input[i..].find('=') {
            Some(offset) => i + offset,
            None => {
                result.push_str(&input[i..]);
                break;
            }
        };

        if eq_pos == 0 {
            result.push_str(&input[i..]);
            break;
        }

        let prev = b[eq_pos - 1];
        if prev == b'=' || prev == b'!' || prev == b'<' || prev == b'>' {
            result.push_str(&input[i..eq_pos + 1]);
            i = eq_pos + 1;
            continue;
        }

        if prev != b'+' && prev != b'-' && prev != b'*' && prev != b'/' && prev != b'%' {
            result.push_str(&input[i..eq_pos + 1]);
            i = eq_pos + 1;
            continue;
        }

        let op_pos = eq_pos - 1;

        let mut var_end = op_pos;
        while var_end > i && b[var_end - 1].is_ascii_whitespace() {
            var_end -= 1;
        }
        let mut var_start = var_end;
        while var_start > i && is_word_char(b[var_start - 1]) {
            var_start -= 1;
        }

        if var_start == var_end {
            result.push_str(&input[i..eq_pos + 1]);
            i = eq_pos + 1;
            continue;
        }

        let var_name = &input[var_start..var_end];
        let op = prev as char;

        let mut rhs_start = eq_pos + 1;
        while rhs_start < length && b[rhs_start].is_ascii_whitespace() {
            rhs_start += 1;
        }
        let mut rhs_end = rhs_start;
        while rhs_end < length && b[rhs_end] != b';' && b[rhs_end] != b'\n' {
            rhs_end += 1;
        }

        let rhs = &input[rhs_start..rhs_end];

        result.push_str(&input[i..var_start]);
        result.push_str(&format!("{} = {} {} ({})", var_name, var_name, op, rhs));
        i = rhs_end;
    }

    result
}

fn transform_increment_decrement(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        if b[i] == b'+' && i + 1 < length && b[i + 1] == b'+' {
            // Check for postfix: var++
            let var_end = i;
            let mut var_start = i;
            while var_start > 0 && is_word_char(b[var_start - 1]) {
                var_start -= 1;
            }

            if var_start < var_end {
                // Postfix var++ — remove already-appended var from result
                let var_len = var_end - var_start;
                result.truncate(result.len() - var_len);
                let var = &input[var_start..var_end];
                result.push_str(&format!(
                    "(function() local _t = {} {} = {} + 1 return _t end)()",
                    var, var, var
                ));
                i += 2;
                continue;
            }

            // Prefix: ++var
            let after_op = i + 2;
            let word_end = read_word(b, after_op);
            if word_end > after_op {
                let var = &input[after_op..word_end];
                result.push_str(&format!(
                    "(function() {} = {} + 1 return {} end)()",
                    var, var, var
                ));
                i = word_end;
                continue;
            }
        }

        if b[i] == b'-' && i + 1 < length && b[i + 1] == b'-' {
            let var_end = i;
            let mut var_start = i;
            while var_start > 0 && is_word_char(b[var_start - 1]) {
                var_start -= 1;
            }

            if var_start < var_end {
                let var_len = var_end - var_start;
                result.truncate(result.len() - var_len);
                let var = &input[var_start..var_end];
                result.push_str(&format!(
                    "(function() local _t = {} {} = {} - 1 return _t end)()",
                    var, var, var
                ));
                i += 2;
                continue;
            }

            let after_op = i + 2;
            let word_end = read_word(b, after_op);
            if word_end > after_op {
                let var = &input[after_op..word_end];
                result.push_str(&format!(
                    "(function() {} = {} - 1 return {} end)()",
                    var, var, var
                ));
                i = word_end;
                continue;
            }
        }

        i += push_utf8_char(&mut result, b, i);
    }

    result
}

fn transform_instanceof_patterns(input: &str) -> String {
    let b = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;

    loop {
        match find_word(b, b"instanceof", i) {
            None => {
                result.push_str(&input[i..]);
                break;
            }
            Some(pos) => {
                let after_inst = skip_spaces(b, pos + 10);
                let array_end = read_word(b, after_inst);
                if array_end > after_inst && &input[after_inst..array_end] == "Array" {
                    let mut expr_end = pos;
                    while expr_end > i && b[expr_end - 1].is_ascii_whitespace() {
                        expr_end -= 1;
                    }

                    let expr_start;
                    if expr_end > i && b[expr_end - 1] == b')' {
                        let mut depth: i32 = 1;
                        let mut k = expr_end as i64 - 2;
                        while k > i as i64 && depth > 0 {
                            if b[k as usize] == b')' {
                                depth += 1;
                            } else if b[k as usize] == b'(' {
                                depth -= 1;
                            }
                            if depth > 0 {
                                k -= 1;
                            }
                        }
                        expr_start = k as usize;
                    } else {
                        let mut es = expr_end;
                        while es > i && is_word_char(b[es - 1]) {
                            es -= 1;
                        }
                        expr_start = es;
                    }

                    let expr = &input[expr_start..expr_end];
                    result.push_str(&input[i..expr_start]);
                    result.push_str(&format!("_isArray({})", expr));
                    i = array_end;
                } else {
                    result.push_str(&input[i..pos + 10]);
                    i = pos + 10;
                }
            }
        }
    }

    result
}

fn transform_null_undefined(input: &str) -> String {
    let result = replace_word(input, "undefined", "nil");
    replace_word(&result, "null", "nil")
}

fn parenthesize_bitwise_operands(input: &str) -> String {
    let b = input.as_bytes();
    let has_bitwise = b.iter().any(|&c| c == b'|' || c == b'&' || c == b'^');
    if !has_bitwise {
        return input.to_string();
    }

    let mut has_comparison = input.contains("==")
        || input.contains("~=")
        || input.contains("<=")
        || input.contains(">=");

    if !has_comparison {
        for (i, &c) in b.iter().enumerate() {
            if (c == b'<' || c == b'>') && (i + 1 >= b.len() || b[i + 1] != b'=') {
                has_comparison = true;
                break;
            }
        }
    }

    if !has_comparison {
        return input.to_string();
    }

    let mut parts = String::with_capacity(input.len() + 16);
    let mut start = 0;
    for (i, &c) in b.iter().enumerate() {
        if c == b'|' || c == b'&' || c == b'^' {
            let operand = trim(&input[start..i]);
            parts.push('(');
            parts.push_str(operand);
            parts.push_str(") ");
            parts.push(c as char);
            parts.push(' ');
            start = i + 1;
        }
    }
    let last_op = trim(&input[start..]);
    parts.push('(');
    parts.push_str(last_op);
    parts.push(')');
    parts
}

fn transform_operators(input: &str) -> String {
    let mut result = input.to_string();

    // Order matters: !== before !=, === before ==

    // !== -> ~=
    {
        let mut pos = 0;
        while let Some(offset) = result[pos..].find("!==") {
            let p = pos + offset;
            result.replace_range(p..p + 3, "~=");
            pos = p + 2;
        }
    }

    // === -> ==
    {
        let mut pos = 0;
        while let Some(offset) = result[pos..].find("===") {
            let p = pos + offset;
            result.replace_range(p..p + 3, "==");
            pos = p + 2;
        }
    }

    // != -> ~= (must not re-match already-converted ~=)
    {
        let rb = result.as_bytes().to_vec();
        let mut temp = String::with_capacity(result.len());
        let mut i = 0;
        while i < rb.len() {
            if rb[i] == b'!' && i + 1 < rb.len() && rb[i + 1] == b'=' {
                temp.push_str("~=");
                i += 2;
            } else {
                i += push_utf8_char(&mut temp, &rb, i);
            }
        }
        result = temp;
    }

    // && -> and
    result = result.replace("&&", " and ");
    // || -> or
    result = result.replace("||", " or ");

    // Logical NOT: !expr -> not expr (must not match ~=)
    {
        let rb = result.as_bytes().to_vec();
        let mut temp = String::with_capacity(result.len());
        let mut i = 0;
        while i < rb.len() {
            if rb[i] == b'!' {
                if i + 1 < rb.len() && rb[i + 1] == b'=' {
                    temp.push('!');
                } else {
                    temp.push_str("not ");
                }
                i += 1;
            } else {
                i += push_utf8_char(&mut temp, &rb, i);
            }
        }
        result = temp;
    }

    // W3C SCXML B.2 (test 459): Parenthesize operands of bitwise OR/AND/XOR
    result = parenthesize_bitwise_operands(&result);

    result
}

fn transform_array_literals(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        if b[i] == b'[' {
            let is_property_access = if i > 0 {
                let prev = b[i - 1];
                prev.is_ascii_alphanumeric()
                    || prev == b'_'
                    || prev == b')'
                    || prev == b']'
                    || prev == b'\x01'
            } else {
                false
            };

            if !is_property_access {
                let close_pos = find_matching_close(b, i, b'[', b']');
                let contents = &input[i + 1..close_pos];
                let contents = transform_array_literals(contents);
                // W3C SCXML 4.6: Replace null/undefined with sentinels at top level
                let contents = replace_word_at_top_level(&contents, "null", "_NULL");
                let contents = replace_word_at_top_level(&contents, "undefined", "_UNDEFINED");
                result.push('{');
                result.push_str(&contents);
                result.push('}');
                i = close_pos + 1;
            } else {
                result.push('[');
                i += 1;
            }
        } else {
            i += push_utf8_char(&mut result, b, i);
        }
    }

    result
}

fn transform_array_indexing(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        if b[i] == b'[' {
            let is_property_access = if i > 0 {
                let prev = b[i - 1];
                prev.is_ascii_alphanumeric()
                    || prev == b'_'
                    || prev == b')'
                    || prev == b']'
                    || prev == b'\x01'
            } else {
                false
            };

            if is_property_access {
                let end = find_matching_close(b, i, b'[', b']');
                let index_expr = trim(&input[i + 1..end]);

                let is_int_literal = !index_expr.is_empty()
                    && index_expr.len() <= 9
                    && index_expr.bytes().all(|c| c.is_ascii_digit());

                if is_int_literal {
                    let idx: usize = index_expr.parse().unwrap();
                    result.push_str(&format!("[{}]", idx + 1));
                    i = end + 1;
                } else {
                    result.push('[');
                    i += 1;
                }
            } else {
                result.push('[');
                i += 1;
            }
        } else {
            i += push_utf8_char(&mut result, b, i);
        }
    }

    result
}

fn transform_array_methods(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    while i < length {
        if b[i] == b'.' {
            // .length\b -> # prefix
            if i + 7 <= length
                && &b[i + 1..i + 7] == b"length"
                && (i + 7 >= length || !is_word_char(b[i + 7]))
            {
                if let Some(id_start) = find_preceding_identifier(result.as_bytes(), result.len()) {
                    let var_name = result[id_start..].to_string();
                    result.truncate(id_start);
                    result.push('#');
                    result.push_str(&var_name);
                    i += 7;
                    continue;
                }
            }

            // .indexOf( -> _indexOf(var,
            if i + 9 <= length && &b[i + 1..i + 9] == b"indexOf(" {
                if let Some(id_start) = find_preceding_identifier(result.as_bytes(), result.len()) {
                    let var_name = result[id_start..].to_string();
                    result.truncate(id_start);
                    let paren_open = i + 8;
                    let arg_end = find_matching_close(b, paren_open, b'(', b')');
                    let args = &input[paren_open + 1..arg_end];
                    result.push_str(&format!("_indexOf({}, {})", var_name, args));
                    i = arg_end + 1;
                    continue;
                }
            }

            // .concat( -> _concat(var,
            if i + 8 <= length && &b[i + 1..i + 8] == b"concat(" {
                if let Some(id_start) = find_preceding_identifier(result.as_bytes(), result.len()) {
                    let var_name = result[id_start..].to_string();
                    result.truncate(id_start);
                    result.push_str(&format!("_concat({}, ", var_name));
                    i += 8;
                    continue;
                }
                // Handle {}.concat( or [].concat(
                let rlen = result.len();
                if rlen >= 2 {
                    let last2 = result[rlen - 2..].to_string();
                    if last2 == "{}" || last2 == "[]" {
                        result.truncate(rlen - 2);
                        result.push_str(&format!("_concat({}, ", last2));
                        i += 8;
                        continue;
                    }
                }
            }

            // .push( -> table.insert(var,
            if i + 6 <= length && &b[i + 1..i + 6] == b"push(" {
                if let Some(id_start) = find_preceding_identifier(result.as_bytes(), result.len()) {
                    let var_name = result[id_start..].to_string();
                    result.truncate(id_start);
                    let paren_open = i + 5;
                    let arg_end = find_matching_close(b, paren_open, b'(', b')');
                    let args = &input[paren_open + 1..arg_end];
                    result.push_str(&format!("table.insert({}, {})", var_name, args));
                    i = arg_end + 1;
                    continue;
                }
            }

            // .join( -> table.concat(var,
            if i + 6 <= length && &b[i + 1..i + 6] == b"join(" {
                if let Some(id_start) = find_preceding_identifier(result.as_bytes(), result.len()) {
                    let var_name = result[id_start..].to_string();
                    result.truncate(id_start);
                    let paren_open = i + 5;
                    let arg_end = find_matching_close(b, paren_open, b'(', b')');
                    let args = &input[paren_open + 1..arg_end];
                    let args = if args.is_empty() { "','" } else { args };
                    result.push_str(&format!("table.concat({}, {})", var_name, args));
                    i = arg_end + 1;
                    continue;
                }
            }
        }

        i += push_utf8_char(&mut result, b, i);
    }

    result
}

fn transform_function_syntax(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();

    struct FuncInfo {
        depth: i32,
        is_constructor: bool,
    }

    let mut output = String::with_capacity(length);
    let mut func_stack: Vec<FuncInfo> = Vec::new();
    let mut brace_depth: i32 = 0;
    let mut i = 0;

    while i < length {
        // Detect function keyword
        if i + 8 <= length && &b[i..i + 8] == b"function" && (i == 0 || !is_word_char(b[i - 1])) {
            let func_start = i;
            let mut j = i + 8;
            while j < length && (b[j].is_ascii_whitespace() || is_word_char(b[j])) {
                j += 1;
            }

            if j < length && b[j] == b'(' {
                let mut paren_depth: i32 = 1;
                j += 1;
                while j < length && paren_depth > 0 {
                    if b[j] == b'(' {
                        paren_depth += 1;
                    } else if b[j] == b')' {
                        paren_depth -= 1;
                    }
                    j += 1;
                }

                while j < length && b[j].is_ascii_whitespace() {
                    j += 1;
                }

                if j < length && b[j] == b'{' {
                    // Detect JS constructor pattern (this.)
                    let mut is_constructor = false;
                    let mut check_depth: i32 = 1;
                    for k in j + 1..length {
                        if b[k] == b'{' {
                            check_depth += 1;
                        } else if b[k] == b'}' {
                            check_depth -= 1;
                        }
                        if check_depth <= 0 {
                            break;
                        }
                        if check_depth == 1 && k + 5 <= length && &b[k..k + 5] == b"this." {
                            is_constructor = true;
                            break;
                        }
                    }

                    output.push_str(&input[func_start..j]);
                    if is_constructor {
                        output.push_str(" local self = {} ");
                    } else {
                        output.push(' ');
                    }
                    func_stack.push(FuncInfo {
                        depth: brace_depth,
                        is_constructor,
                    });
                    brace_depth += 1;
                    i = j + 1;
                    continue;
                }
            }
        }

        if b[i] == b'{' {
            brace_depth += 1;
            output.push('{');
        } else if b[i] == b'}' {
            brace_depth -= 1;
            if !func_stack.is_empty() && brace_depth == func_stack.last().unwrap().depth {
                if func_stack.last().unwrap().is_constructor {
                    output.push_str(" return self end");
                } else {
                    output.push_str(" end");
                }
                func_stack.pop();
            } else {
                output.push('}');
            }
        } else {
            let n = push_utf8_char(&mut output, b, i);
            i += n;
            continue; // skip the i += 1 below
        }
        i += 1;
    }

    // Transform this.prop -> self.prop (only when followed by '.')
    let ob = output.as_bytes();
    let mut result = String::with_capacity(output.len());
    let mut last = 0;
    let mut pos = 0;
    loop {
        match find_word(ob, b"this", pos) {
            None => break,
            Some(p) => {
                if p + 4 < ob.len() && ob[p + 4] == b'.' {
                    result.push_str(&output[last..p]);
                    result.push_str("self");
                    last = p + 4;
                }
                pos = p + 4;
            }
        }
    }
    result.push_str(&output[last..]);
    result
}

fn transform_var_declarations(input: &str) -> String {
    let result = replace_keyword_prefix(input, "var", "");
    let result = replace_keyword_prefix(&result, "let", "");
    replace_keyword_prefix(&result, "const", "")
}

fn transform_new_expression(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    loop {
        match find_word(b, b"new", i) {
            None => {
                result.push_str(&input[i..]);
                break;
            }
            Some(pos) => {
                result.push_str(&input[i..pos]);

                let j = skip_spaces(b, pos + 3);
                let name_end = read_word(b, j);
                if name_end > j {
                    let k = skip_spaces(b, name_end);
                    if k < length && b[k] == b'(' {
                        result.push_str(&input[j..name_end]);
                        result.push('(');
                        i = k + 1;
                        continue;
                    }
                }

                result.push_str("new");
                i = pos + 3;
            }
        }
    }

    result
}

/// W3C SCXML test subset: single-level ternary only. Nested ternaries not supported
/// (matches Python/C++ reference implementation limitation).
fn transform_ternary_operator(input: &str) -> String {
    let q_pos = match input.find('?') {
        Some(p) => p,
        None => return input.to_string(),
    };
    let c_pos = match input[q_pos + 1..].find(':') {
        Some(p) => q_pos + 1 + p,
        None => return input.to_string(),
    };

    let cond = trim(&input[..q_pos]);
    let true_val = trim(&input[q_pos + 1..c_pos]);
    let false_val = trim(&input[c_pos + 1..]);

    if cond.is_empty() || true_val.is_empty() || false_val.is_empty() {
        return input.to_string();
    }

    format!("({} and {} or {})", cond, true_val, false_val)
}

fn transform_dom_methods(input: &str) -> String {
    let mut result = input.to_string();
    for method in &[".getElementsByTagName(", ".getAttribute(", ".getTagName("] {
        let replacement = format!(":{}", &method[1..]);
        result = result.replace(method, &replacement);
    }
    result
}

fn transform_object_literals(input: &str) -> String {
    let b = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut brace_depth: i32 = 0;
    let mut ternary_count: i32 = 0;
    let mut ternary_stack: Vec<i32> = Vec::new();
    let mut i = 0;

    while i < b.len() {
        let c = b[i];
        if c == b'{' {
            brace_depth += 1;
            ternary_stack.push(ternary_count);
            ternary_count = 0;
            result.push('{');
        } else if c == b'}' {
            brace_depth -= 1;
            if let Some(saved) = ternary_stack.pop() {
                ternary_count = saved;
            }
            result.push('}');
        } else if c == b'?' && brace_depth > 0 {
            ternary_count += 1;
            result.push('?');
        } else if c == b':' && brace_depth > 0 {
            if ternary_count > 0 {
                ternary_count -= 1;
                result.push(':');
            } else {
                // Check if preceded by identifier or string placeholder (object key)
                let mut key_end = i;
                while key_end > 0 && (b[key_end - 1] == b' ' || b[key_end - 1] == b'\t') {
                    key_end -= 1;
                }
                if key_end > 0 && (b[key_end - 1].is_ascii_alphanumeric() || b[key_end - 1] == b'_')
                {
                    result.push_str(" =");
                } else if key_end > 0 && b[key_end - 1] == b'\x01' {
                    // JSON string key placeholder
                    let placeholder_end = key_end;
                    let mut placeholder_start = key_end - 1;
                    while placeholder_start > 0 && b[placeholder_start - 1] != b'\x01' {
                        placeholder_start -= 1;
                    }
                    if placeholder_start > 0 {
                        placeholder_start -= 1;
                    }
                    let placeholder = &input[placeholder_start..placeholder_end];
                    let result_str = result.clone();
                    if let Some(ph_pos) = result_str.rfind(placeholder) {
                        result.clear();
                        result.push_str(&result_str[..ph_pos]);
                        result.push('[');
                        result.push_str(placeholder);
                        result.push(']');
                        result.push_str(&result_str[ph_pos + placeholder.len()..]);
                    }
                    result.push_str(" =");
                } else {
                    result.push(':');
                }
            }
        } else {
            i += push_utf8_char(&mut result, b, i);
            continue;
        }
        i += 1;
    }

    result
}

fn transform_semicolons(input: &str) -> String {
    let result = input.trim_end_matches(';');
    result.replace(';', "\n")
}

fn transform_math_builtins(input: &str) -> String {
    let mut result = input.to_string();

    // Math.pow(a, b) -> (a)^(b)
    {
        let mut temp = String::with_capacity(result.len());
        let mut i = 0;
        loop {
            match result[i..].find("Math.pow(") {
                None => {
                    temp.push_str(&result[i..]);
                    break;
                }
                Some(offset) => {
                    let pos = i + offset;
                    temp.push_str(&result[i..pos]);

                    let rb = result.as_bytes();
                    let arg_start = pos + 9;
                    let mut depth: i32 = 1;
                    let mut comma_pos: Option<usize> = None;
                    let mut j = arg_start;
                    while j < rb.len() && depth > 0 {
                        if rb[j] == b'(' {
                            depth += 1;
                        } else if rb[j] == b')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        } else if rb[j] == b',' && depth == 1 && comma_pos.is_none() {
                            comma_pos = Some(j);
                        }
                        j += 1;
                    }
                    if depth == 0 {
                        if let Some(cp) = comma_pos {
                            let a = trim(&result[arg_start..cp]);
                            let b_val = trim(&result[cp + 1..j]);
                            temp.push_str(&format!("({})^({})", a, b_val));
                            i = j + 1;
                        } else {
                            temp.push_str("Math.pow(");
                            i = arg_start;
                        }
                    } else {
                        temp.push_str("Math.pow(");
                        i = arg_start;
                    }
                }
            }
        }
        result = temp;
    }

    // Direct mappings
    let replacements = [
        ("Math.sqrt", "math.sqrt"),
        ("Math.abs", "math.abs"),
        ("Math.floor", "math.floor"),
        ("Math.ceil", "math.ceil"),
        ("Math.max", "math.max"),
        ("Math.min", "math.min"),
        ("Math.random", "math.random"),
        ("Math.log", "math.log"),
        ("Math.exp", "math.exp"),
        ("Math.sin", "math.sin"),
        ("Math.cos", "math.cos"),
        ("Math.tan", "math.tan"),
        ("Math.PI", "math.pi"),
        ("Math.E", "math.exp(1)"),
    ];
    for (from, to) in &replacements {
        result = result.replace(from, to);
    }

    result
}

// ── Structural transforms (script-only) ─────────────────────────────

fn transform_for_in_loops(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    loop {
        match find_word(b, b"for", i) {
            None => {
                result.push_str(&input[i..]);
                break;
            }
            Some(for_pos) => {
                result.push_str(&input[i..for_pos]);
                i = for_pos + 3;

                let paren_start = skip_spaces(b, i);
                if paren_start >= length || b[paren_start] != b'(' {
                    result.push_str("for");
                    continue;
                }

                let paren_end = find_matching_close(b, paren_start, b'(', b')');
                let header = trim(&input[paren_start + 1..paren_end]);

                // Strip var/let/const
                let header_stripped = if let Some(rest) = header.strip_prefix("var ") {
                    rest
                } else if let Some(rest) = header.strip_prefix("let ") {
                    rest
                } else if let Some(rest) = header.strip_prefix("const ") {
                    rest
                } else {
                    header
                };
                let header_stripped = trim(header_stripped);

                // Check for " in " keyword
                let mut is_for_in = false;
                let mut loop_var = "";
                let mut obj_expr = "";

                if let Some(in_pos) = find_word(header_stripped.as_bytes(), b"in", 0) {
                    let before_in = trim(&header_stripped[..in_pos]);
                    let after_in = trim(&header_stripped[in_pos + 2..]);
                    let is_single_ident =
                        !before_in.is_empty() && before_in.bytes().all(|c| is_word_char(c));
                    if is_single_ident && !after_in.is_empty() {
                        loop_var = before_in;
                        obj_expr = after_in;
                        is_for_in = true;
                    }
                }

                if !is_for_in {
                    result.push_str("for");
                    i = for_pos + 3;
                    continue;
                }

                i = paren_end + 1;

                let body_start = skip_spaces(b, i);
                if body_start >= length || b[body_start] != b'{' {
                    result.push_str(&format!(
                        "for {}, _ in pairs({}) do end\n",
                        loop_var, obj_expr
                    ));
                    continue;
                }
                let body_end = find_matching_close(b, body_start, b'{', b'}');
                let body = trim(&input[body_start + 1..body_end]);
                i = body_end + 1;

                result.push_str(&format!(
                    "for {}, _ in pairs({}) do\n{}\nend\n",
                    loop_var, obj_expr, body
                ));
            }
        }
    }

    result
}

fn transform_for_loops(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    loop {
        match find_word(b, b"for", i) {
            None => {
                result.push_str(&input[i..]);
                break;
            }
            Some(for_pos) => {
                result.push_str(&input[i..for_pos]);
                i = for_pos + 3;

                let paren_start = skip_spaces(b, i);
                if paren_start >= length || b[paren_start] != b'(' {
                    result.push_str("for");
                    continue;
                }

                // Parse three semicolon-separated parts
                let mut paren_depth: i32 = 1;
                let mut pos = paren_start + 1;
                let mut semi_positions: Vec<usize> = Vec::new();
                while pos < length && paren_depth > 0 {
                    if b[pos] == b'(' {
                        paren_depth += 1;
                    } else if b[pos] == b')' {
                        paren_depth -= 1;
                    } else if b[pos] == b';' && paren_depth == 1 {
                        semi_positions.push(pos);
                    }
                    if paren_depth > 0 {
                        pos += 1;
                    }
                }

                if semi_positions.len() != 2 {
                    result.push_str("for");
                    i = for_pos + 3;
                    continue;
                }

                let init = trim(&input[paren_start + 1..semi_positions[0]]);
                let cond = trim(&input[semi_positions[0] + 1..semi_positions[1]]);
                let mut incr = trim(&input[semi_positions[1] + 1..pos]).to_string();
                i = pos + 1;

                // Simplify i++/i-- -> i = i + 1 / i = i - 1
                if incr.len() >= 3 {
                    let mut var_name = String::new();
                    let mut is_incr = false;
                    let mut is_decr = false;
                    if incr.ends_with("++") {
                        var_name = trim(&incr[..incr.len() - 2]).to_string();
                        is_incr = true;
                    } else if let Some(rest) = incr.strip_prefix("++") {
                        var_name = trim(rest).to_string();
                        is_incr = true;
                    } else if incr.ends_with("--") {
                        var_name = trim(&incr[..incr.len() - 2]).to_string();
                        is_decr = true;
                    } else if let Some(rest) = incr.strip_prefix("--") {
                        var_name = trim(rest).to_string();
                        is_decr = true;
                    }
                    if is_incr {
                        incr = format!("{} = {} + 1", var_name, var_name);
                    } else if is_decr {
                        incr = format!("{} = {} - 1", var_name, var_name);
                    }
                }

                // Find '{' for body
                let body_start = skip_spaces(b, i);
                if body_start >= length || b[body_start] != b'{' {
                    result.push_str(&input[for_pos..i]);
                    continue;
                }

                let body_end = find_matching_close(b, body_start, b'{', b'}');
                let body = trim(&input[body_start + 1..body_end]);
                i = body_end + 1;

                // Detect common pattern: for (var i = 0; i < arr.length; i++)
                let mut used_numeric_for = false;
                let init_trimmed = if let Some(rest) = init.strip_prefix("let ") {
                    rest
                } else if let Some(rest) = init.strip_prefix("var ") {
                    rest
                } else if let Some(rest) = init.strip_prefix("const ") {
                    rest
                } else {
                    init
                };
                let init_trimmed = trim(init_trimmed);
                if let Some(eq_pos) = init_trimmed.find('=') {
                    let loop_var = trim(&init_trimmed[..eq_pos]);
                    let start_val = trim(&init_trimmed[eq_pos + 1..]);
                    if start_val == "0" && !loop_var.is_empty() {
                        let cond_pattern = format!("{} < ", loop_var);
                        if cond.starts_with(&cond_pattern) {
                            let bound_expr = trim(&cond[cond_pattern.len()..]);
                            if bound_expr.len() > 7 && bound_expr.ends_with(".length") {
                                let arr_name = &bound_expr[..bound_expr.len() - 7];
                                result.push_str(&format!(
                                    "for {} = 1, #{} do\n{}\nend\n",
                                    loop_var, arr_name, body
                                ));
                                used_numeric_for = true;
                            }
                        }
                    }
                }

                if !used_numeric_for {
                    result.push_str(&format!(
                        "{}\nwhile {} do\n{}\n{}\nend\n",
                        init, cond, body, incr
                    ));
                }
            }
        }
    }

    result
}

fn transform_conditional_blocks(input: &str) -> String {
    let b = input.as_bytes();
    let length = b.len();
    let mut result = String::with_capacity(length);
    let mut i = 0;

    loop {
        match find_word(b, b"if", i) {
            None => {
                result.push_str(&input[i..]);
                break;
            }
            Some(if_pos) => {
                result.push_str(&input[i..if_pos]);
                i = if_pos + 2;

                let cond_start = skip_spaces(b, i);
                if cond_start >= length || b[cond_start] != b'(' {
                    result.push_str("if");
                    continue;
                }

                let cond_end = find_matching_close(b, cond_start, b'(', b')');
                let condition = trim(&input[cond_start + 1..cond_end]);
                i = cond_end + 1;

                let body_start = skip_spaces(b, i);
                if body_start >= length || b[body_start] != b'{' {
                    result.push_str(&format!("if ({})", condition));
                    i = body_start;
                    continue;
                }

                let body_end = find_matching_close(b, body_start, b'{', b'}');
                let body = trim(&input[body_start + 1..body_end]);
                i = body_end + 1;

                result.push_str(&format!("if {} then\n{}\n", condition, body));

                // Check for else/else if
                loop {
                    let else_pos = skip_spaces(b, i);
                    if else_pos + 4 <= length
                        && &b[else_pos..else_pos + 4] == b"else"
                        && (else_pos + 4 >= length || !is_word_char(b[else_pos + 4]))
                    {
                        i = else_pos + 4;
                        let after_else = skip_spaces(b, i);

                        // "else if" -> "elseif"
                        if after_else + 2 <= length
                            && &b[after_else..after_else + 2] == b"if"
                            && (after_else + 2 >= length || !is_word_char(b[after_else + 2]))
                        {
                            i = after_else + 2;
                            let ei_cond_start = skip_spaces(b, i);
                            if ei_cond_start < length && b[ei_cond_start] == b'(' {
                                let cond_end = find_matching_close(b, ei_cond_start, b'(', b')');
                                let condition = trim(&input[ei_cond_start + 1..cond_end]);
                                i = cond_end + 1;

                                let body_start = skip_spaces(b, i);
                                if body_start < length && b[body_start] == b'{' {
                                    let body_end = find_matching_close(b, body_start, b'{', b'}');
                                    let body = trim(&input[body_start + 1..body_end]);
                                    i = body_end + 1;
                                    result.push_str(&format!(
                                        "elseif {} then\n{}\n",
                                        condition, body
                                    ));
                                    continue;
                                }
                            }
                            i = else_pos + 4;
                        }

                        // Plain "else { ... }"
                        let body_start = skip_spaces(b, i);
                        if body_start < length && b[body_start] == b'{' {
                            let body_end = find_matching_close(b, body_start, b'{', b'}');
                            let body = trim(&input[body_start + 1..body_end]);
                            i = body_end + 1;
                            result.push_str(&format!("else\n{}\n", body));
                        }
                    }
                    break;
                }
                result.push_str("end\n");
            }
        }
    }

    result
}

fn transform_bare_expressions(input: &str) -> String {
    let lines: Vec<&str> = input.split('\n').collect();

    let mut last_non_empty: Option<usize> = None;
    for j in (0..lines.len()).rev() {
        if !trim(lines[j]).is_empty() {
            last_non_empty = Some(j);
            break;
        }
    }

    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len());
    for (j, &line) in lines.iter().enumerate() {
        let trimmed = trim(line);
        if trimmed.is_empty() || trimmed == "end" {
            result_lines.push(line.to_string());
            continue;
        }

        let mut is_statement = false;

        // Check for assignment (but not ==, ~=, <=, >=)
        let tb = trimmed.as_bytes();
        for (k, &c) in tb.iter().enumerate() {
            if c == b'=' && k > 0 {
                let prev = tb[k - 1];
                if prev != b'~' && prev != b'<' && prev != b'>' && prev != b'!'
                    && (k + 1 >= tb.len() || tb[k + 1] != b'=') {
                        is_statement = true;
                        break;
                    }
            }
        }

        if !is_statement {
            let keywords = [
                "local", "return", "if", "for", "while", "repeat", "do", "end", "else", "elseif",
                "then", "function", "break",
            ];
            for kw in &keywords {
                let kwlen = kw.len();
                if trimmed.len() >= kwlen
                    && &trimmed[..kwlen] == *kw
                    && (trimmed.len() == kwlen || !is_word_char(tb[kwlen]))
                {
                    is_statement = true;
                    break;
                }
            }
        }

        // Detect function calls: identifier immediately followed by '(' (e.g., foo(...))
        // Excludes bare parenthesized arithmetic like x + (y * z)
        if !is_statement {
            for (k, &c) in tb.iter().enumerate() {
                if c == b'(' && k > 0 && is_word_char(tb[k - 1]) {
                    is_statement = true;
                    break;
                }
            }
        }

        if !is_statement {
            if last_non_empty == Some(j) {
                result_lines.push(format!("return {}", trimmed));
            } else {
                result_lines.push(format!("_ = {}", trimmed));
            }
        } else {
            result_lines.push(line.to_string());
        }
    }

    let final_result = result_lines.join("\n");
    final_result.trim_end_matches('\n').to_string()
}

// ── Guard-specific truthiness wrapping ──────────────────────────────

fn needs_truthiness_wrapping(expr: &str) -> bool {
    if expr.contains("==") || expr.contains("~=") || expr.contains(">=") || expr.contains("<=") {
        return false;
    }
    if expr.contains("_scxml_truthy") || expr.contains("_isArray") {
        return false;
    }

    let eb = expr.as_bytes();
    for (i, &c) in eb.iter().enumerate() {
        if (c == b'<' || c == b'>') && (i + 1 >= eb.len() || eb[i + 1] != b'=') {
            return false;
        }
    }

    let trimmed = trim(expr);
    if trimmed == "true" || trimmed == "false" {
        return false;
    }

    if trimmed.contains("In(") && is_pure_in_predicate(trimmed) {
        return false;
    }

    if trimmed.starts_with("not ") {
        return false;
    }

    true
}

fn wrap_truthiness(input: &str) -> String {
    if !needs_truthiness_wrapping(input) {
        return input.to_string();
    }
    format!("_scxml_truthy({})", input)
}

// ── Public API ──────────────────────────────────────────────────────

/// Transform an ECMAScript expression to Lua.
pub fn transform_expression(ecma_script: &str, context: ExpressionContext) -> String {
    if ecma_script.is_empty() {
        return ecma_script.to_string();
    }

    // Pre-pass: typeof patterns must be transformed BEFORE string protection
    let pre_processed = transform_typeof_patterns(ecma_script);

    // Stage 1: Protect string literals
    let (mut processed, literals) = protect_string_literals(&pre_processed);

    // Stage 2: Apply transformation pipeline (order matters)
    processed = transform_math_builtins(&processed);
    processed = transform_compound_assignment(&processed);
    processed = transform_increment_decrement(&processed);
    // W3C SCXML B.2: ECMAScript function expressions in value-expression
    // context (e.g. `<data id="f" expr="function(x) {return x+1;}"/>`).
    // The script-pipeline already lowers `function name(args) { body }` and
    // `function(args) { body }` to Lua's `function ... end` form; running the
    // same pass here lets transform_expression handle anonymous function
    // expressions wherever an ECMAScript value is accepted (cond, location
    // expr, data expr, assign expr).
    processed = transform_function_syntax(&processed);
    processed = transform_instanceof_patterns(&processed);
    processed = transform_array_literals(&processed);
    processed = transform_null_undefined(&processed);
    processed = transform_new_expression(&processed);
    processed = transform_array_methods(&processed);
    processed = transform_array_indexing(&processed);
    processed = transform_object_literals(&processed);
    processed = transform_ternary_operator(&processed);
    processed = transform_operators(&processed);
    processed = transform_dom_methods(&processed);
    processed = transform_var_declarations(&processed);
    processed = transform_semicolons(&processed);

    // Stage 3: Restore string literals
    let mut result = restore_string_literals(&processed, &literals);

    // Stage 4: Guard-specific truthiness wrapping
    if context == ExpressionContext::Guard {
        result = wrap_truthiness(&result);
    }

    result
}

/// Transform a multi-line script block to Lua.
pub fn transform_script(script: &str) -> String {
    if script.is_empty() {
        return script.to_string();
    }

    // Pre-pass: typeof before string protection
    let pre_processed = transform_typeof_patterns(script);

    // Stage 1: Protect string literals
    let (mut processed, literals) = protect_string_literals(&pre_processed);

    // Stage 2: Structural transforms (must see original JS syntax with semicolons)
    processed = transform_for_in_loops(&processed);
    processed = transform_for_loops(&processed);

    // Stage 3: Apply transformations
    processed = transform_math_builtins(&processed);
    processed = transform_compound_assignment(&processed);
    processed = transform_increment_decrement(&processed);
    processed = transform_function_syntax(&processed);
    processed = transform_instanceof_patterns(&processed);
    processed = transform_array_literals(&processed);
    processed = transform_null_undefined(&processed);
    processed = transform_new_expression(&processed);
    processed = transform_array_methods(&processed);
    processed = transform_array_indexing(&processed);
    processed = transform_object_literals(&processed);
    processed = transform_ternary_operator(&processed);
    processed = transform_operators(&processed);
    processed = transform_dom_methods(&processed);
    processed = transform_var_declarations(&processed);
    processed = transform_semicolons(&processed);
    processed = transform_conditional_blocks(&processed);
    processed = transform_bare_expressions(&processed);

    // Stage 4: Restore string literals
    restore_string_literals(&processed, &literals)
}
