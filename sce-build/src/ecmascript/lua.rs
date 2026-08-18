// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! ECMAScript AST → Lua source.
//!
//! Meaning-preserving, which is the whole reason this exists. The operators
//! that do not mean the same thing in both languages are the ones listed
//! here; everything else maps across unchanged.
//!
//! | ECMAScript | Lua emitted | why not the obvious one |
//! |---|---|---|
//! | `a && b` | `_scxml_truthy`-guarded | Lua's only falsy values are `nil` and `false`, so `0 && x` answers `x` |
//! | `a \|\| b` | same | as above |
//! | `!a` | `not _scxml_truthy(a)` | `not 0` is `false` in Lua and `true` in ECMAScript |
//! | `a ? b : c` | immediately-called function | the `and/or` idiom loses `b` when it is `false` or `nil` |
//! | `a == b` | `_scxml_eq(a, b)` | Lua's `==` is ECMAScript's `===` |
//! | `a + b` | `_scxml_add(a, b)` | `+` concatenates when either side is a string |
//! | `a % b` | `math.fmod(a, b)` | Lua's `%` is floor-mod; ECMAScript's truncates |
//! | `a & b` | `_scxml_bitand(a, b)` | operands are ToInt32, not integers, and go-lua has no `&` |
//! | `typeof a` | `_typeof(a)` | Lua's `type` names different things |
//! | `a instanceof Array` | `_isArray(a)` | no prototype chain to walk |
//! | `x++` | immediately-called function | Lua has no expression with a side effect |
//!
//! The helpers named `_scxml_*` come from `sce/include/scripting/
//! ecma_semantics.lua`, which every engine loads; `_typeof`, `_isArray`,
//! `_indexOf` and `_concat` are the vocabulary each engine already
//! installs natively.
//!
//! # Two contexts
//!
//! An expression is emitted either for its **value** (`<data expr>`,
//! `<assign expr>`) or as a **condition** (`cond`, `<if>`). The difference
//! is not cosmetic: in condition position the result is a Lua boolean
//! carrying ECMAScript truthiness, so `cond="Var1"` with `Var1 = 0` is
//! false — which is what makes `&&` collapse to a plain `and` there
//! instead of the value-preserving function call it needs in value
//! position.

use super::builtins::{self, DOM_METHODS};
use super::{BinOp, Expr, LogicalOp, Stmt, UnaryOp, UpdateOp};
use crate::forge::error::ExprError;

/// Emit an expression for its value.
pub fn emit_value(expr: &Expr) -> Result<String, ExprError> {
    value(expr)
}

/// Emit an expression as a condition — a Lua boolean under ECMAScript
/// truthiness.
pub fn emit_condition(expr: &Expr) -> Result<String, ExprError> {
    condition(expr)
}

/// Emit an assignment target.
pub fn emit_location(expr: &Expr) -> Result<String, ExprError> {
    match expr {
        Expr::Ident(_) | Expr::Member { .. } | Expr::Index { .. } => value(expr),
        other => Err(ExprError::InvalidLvalue {
            location: describe(other).to_string(),
            detail: "an assignment target must be an identifier or a member path".into(),
        }),
    }
}

/// Render text as a Lua string literal.
///
/// Exposed so a caller that has decided a piece of source is *data* rather
/// than an expression — inline `<data>` content — renders it with the same
/// escaping rules the emitter uses everywhere else.
pub fn string_literal(text: &str) -> String {
    lua_string(text)
}

/// Emit a statement list as a Lua chunk.
pub fn emit_script(stmts: &[Stmt]) -> Result<String, ExprError> {
    let mut out = String::new();
    let scope = Scope {
        depth: 0,
        in_function: false,
    };
    for stmt in stmts {
        statement(stmt, scope, &mut out)?;
    }
    Ok(out.trim_end().to_string())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Expressions
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn value(expr: &Expr) -> Result<String, ExprError> {
    // §scxml-B-2: the ECMAScript datamodel's value expressions, lowered to
    // the Lua the engine underneath actually runs.
    Ok(match expr {
        Expr::Number(n) => lua_number(n)?,
        Expr::Str(s) => lua_string(s),
        Expr::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        // §scxml-4.6: `null` is a datamodel value, and Lua has one empty
        // value for it and `undefined` both — which is why the engines bind
        // `_NULL` and `_UNDEFINED` to the same thing.
        Expr::Nullish => "nil".to_string(),
        // A namespace reaches here only when it stands somewhere a
        // member expression does not put it — on its own, under a
        // computed key, as an operand. `member` and `call` answer the
        // positions where the name is legal before recursing, so this
        // arm is the whole of the rest.
        Expr::Ident(name) => match builtins::namespace_not_a_value(name) {
            Some(refusal) => return Err(refusal),
            None => lua_ident_read(name),
        },
        // Bound by the constructor emission in `function_literal`.
        Expr::This => "self".to_string(),
        Expr::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(value(item)?);
            }
            format!("{{{}}}", parts.join(", "))
        }
        Expr::Object(props) => {
            let mut parts = Vec::with_capacity(props.len());
            for (key, prop) in props {
                parts.push(format!("[{}] = {}", lua_string(key), value(prop)?));
            }
            format!("{{{}}}", parts.join(", "))
        }
        Expr::Member { object, property } => member(object, property)?,
        Expr::Index { object, index } => index_access(object, index)?,
        Expr::Call { callee, args } => call(callee, args)?,
        // A constructor is a function that fills `this` and returns it, so
        // `new F(a)` is `F(a)`. See `function_literal` for the other half.
        Expr::New { callee, args } => {
            // `new` changes what a call means, not what may be called:
            // `new Object()` reached Lua as `Object()` once the operator
            // was dropped, so the constructor form has to ask the same
            // question the plain call does.
            if let Expr::Ident(name) = callee.as_ref() {
                if let Some(refusal) = builtins::uncallable_namespace(name) {
                    return Err(refusal);
                }
            }
            let mut parts = Vec::with_capacity(args.len());
            for arg in args {
                parts.push(value(arg)?);
            }
            format!("{}({})", operand(callee)?, parts.join(", "))
        }
        Expr::Unary { op, operand: inner } => match op {
            UnaryOp::Not => format!("(not {})", condition(inner)?),
            UnaryOp::Neg => format!("(-{})", operand(inner)?),
            UnaryOp::Pos => format!("_scxml_tonumber({})", value(inner)?),
            UnaryOp::BitNot => format!("_scxml_bitnot({})", value(inner)?),
            UnaryOp::TypeOf => format!("_typeof({})", value(inner)?),
        },
        Expr::Update { op, prefix, target } => update_expression(*op, *prefix, target)?,
        Expr::Binary { op, left, right } => binary(*op, left, right)?,
        Expr::Logical { op, left, right } => {
            // Value position: the result is one of the operands, so the
            // choice has to be made on ECMAScript truthiness while the
            // operand itself is what comes back.
            let keyword = match op {
                LogicalOp::And => "if _scxml_truthy(__l) then return {r} end return __l",
                LogicalOp::Or => "if _scxml_truthy(__l) then return __l end return {r}",
            };
            let body = keyword.replace("{r}", &value(right)?);
            format!("(function() local __l = {} {} end)()", value(left)?, body)
        }
        Expr::Conditional {
            condition: cond,
            consequent,
            alternate,
        } => format!(
            "(function() if {} then return {} end return {} end)()",
            condition(cond)?,
            value(consequent)?,
            value(alternate)?
        ),
        Expr::Function { params, body, .. } => function_literal(params, body)?,
        Expr::Assign {
            op,
            target,
            value: v,
        } => {
            let location = emit_location(target)?;
            format!(
                "(function() {} = {} return {} end)()",
                location,
                assigned_value(*op, target, v)?,
                location
            )
        }
    })
}

/// An operand that a postfix operator (`.x`, `[i]`, `(…)`) or a prefix `-`
/// attaches to. Parenthesised unless it is already a single term, because
/// `(function() … end)().x` is legal Lua and `1 + 2 .x` is not.
fn operand(expr: &Expr) -> Result<String, ExprError> {
    let emitted = value(expr)?;
    if is_term(expr) {
        return Ok(emitted);
    }
    Ok(format!("({emitted})"))
}

/// Whether the emitted form is a single Lua term — an atom, a call, an
/// index or a table constructor — and so needs no parentheses around it.
fn is_term(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Number(_)
            | Expr::Str(_)
            | Expr::Bool(_)
            | Expr::Nullish
            | Expr::Ident(_)
            | Expr::This
            | Expr::Array(_)
            | Expr::Object(_)
            | Expr::Member { .. }
            | Expr::Index { .. }
            | Expr::Call { .. }
            | Expr::New { .. }
            | Expr::Update { .. }
            | Expr::Conditional { .. }
            | Expr::Logical { .. }
            | Expr::Assign { .. }
    )
}

fn condition(expr: &Expr) -> Result<String, ExprError> {
    // A node that already yields a Lua boolean is its own condition; every
    // other value has to go through ECMAScript's truthiness, which is not
    // Lua's.
    Ok(match expr {
        Expr::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Expr::Unary {
            op: UnaryOp::Not,
            operand: inner,
        } => format!("(not {})", condition(inner)?),
        Expr::Binary { op, left, right } if yields_boolean(*op) => binary(*op, left, right)?,
        Expr::Logical { op, left, right } => {
            let keyword = match op {
                LogicalOp::And => "and",
                LogicalOp::Or => "or",
            };
            format!("({} {keyword} {})", condition(left)?, condition(right)?)
        }
        other => format!("_scxml_truthy({})", value(other)?),
    })
}

fn yields_boolean(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::StrictEq
            | BinOp::StrictNeq
            | BinOp::LooseEq
            | BinOp::LooseNeq
            | BinOp::Lt
            | BinOp::Gt
            | BinOp::LtEq
            | BinOp::GtEq
            | BinOp::InstanceOf
            | BinOp::In
    )
}

fn binary(op: BinOp, left: &Expr, right: &Expr) -> Result<String, ExprError> {
    let infix = |symbol: &str| -> Result<String, ExprError> {
        Ok(format!("({} {symbol} {})", value(left)?, value(right)?))
    };
    Ok(match op {
        // `+` needs the runtime helper only when the operand types are open.
        // A string literal on either side settles it: ECMA-262 13.15.3 makes
        // the operator concatenation as soon as one primitive is a string,
        // whatever the other side turns out to be. Deciding that here rather
        // than at runtime is what keeps a `'a: ' + x + '\n' + y` chain
        // readable in the generated source instead of nesting one call per
        // operator.
        BinOp::Add if is_string_literal(left) || is_string_literal(right) => {
            format!("({} .. {})", as_string(left)?, as_string(right)?)
        }
        // Two numeric literals cannot concatenate either.
        BinOp::Add if is_number_literal(left) && is_number_literal(right) => infix("+")?,
        BinOp::Add => format!("_scxml_add({}, {})", value(left)?, value(right)?),
        BinOp::Sub => infix("-")?,
        BinOp::Mul => infix("*")?,
        BinOp::Div => infix("/")?,
        // ECMAScript's `%` truncates toward zero; Lua's floors, so they
        // disagree on every negative operand. `math.fmod` is the truncating
        // one.
        BinOp::Mod => format!("math.fmod({}, {})", value(left)?, value(right)?),
        BinOp::StrictEq => infix("==")?,
        BinOp::StrictNeq => infix("~=")?,
        BinOp::LooseEq => format!("_scxml_eq({}, {})", value(left)?, value(right)?),
        BinOp::LooseNeq => format!("(not _scxml_eq({}, {}))", value(left)?, value(right)?),
        BinOp::Lt => infix("<")?,
        BinOp::Gt => infix(">")?,
        BinOp::LtEq => infix("<=")?,
        BinOp::GtEq => infix(">=")?,
        BinOp::BitAnd => format!("_scxml_bitand({}, {})", value(left)?, value(right)?),
        BinOp::BitOr => format!("_scxml_bitor({}, {})", value(left)?, value(right)?),
        BinOp::BitXor => format!("_scxml_bitxor({}, {})", value(left)?, value(right)?),
        BinOp::Shl => format!("_scxml_shl({}, {})", value(left)?, value(right)?),
        BinOp::Shr => format!("_scxml_shr({}, {})", value(left)?, value(right)?),
        BinOp::UShr => format!("_scxml_ushr({}, {})", value(left)?, value(right)?),
        BinOp::InstanceOf => {
            // The only constructor the datamodel can name is `Array`: SCE's
            // Lua has no prototype chain, and an author who writes
            // `x instanceof Foo` is asking a question this engine cannot
            // answer — so it is refused rather than answered `false`.
            match right {
                Expr::Ident(name) if name == "Array" => format!("_isArray({})", value(left)?),
                other => {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!(
                            "instanceof {} (only Array is representable)",
                            describe(other)
                        ),
                    })
                }
            }
        }
        BinOp::In => format!("({}[{}] ~= nil)", operand(right)?, value(left)?),
    })
}

fn is_string_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Str(_))
}

fn is_number_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Number(_))
}

/// An operand of a concatenation. A string literal is already one; anything
/// else goes through ToString, which is not Lua's `tostring` — Lua prints an
/// integral float as `1.0` where ECMAScript says `1`.
fn as_string(expr: &Expr) -> Result<String, ExprError> {
    if is_string_literal(expr) {
        return value(expr);
    }
    Ok(format!("_scxml_tostring({})", value(expr)?))
}

fn update_expression(op: UpdateOp, prefix: bool, target: &Expr) -> Result<String, ExprError> {
    let location = emit_location(target)?;
    let step = match op {
        UpdateOp::Inc => "+ 1",
        UpdateOp::Dec => "- 1",
    };
    // ECMA-262 13.4: the operand is ToNumber'd first, so `x = '1'; x++`
    // leaves 2 rather than concatenating.
    Ok(if prefix {
        format!(
            "(function() {location} = _scxml_tonumber({location}) {step} return {location} end)()"
        )
    } else {
        format!(
            "(function() local __v = _scxml_tonumber({location}) {location} = __v {step} return __v end)()"
        )
    })
}

/// The right-hand side of an assignment, folding in the compound operator.
fn assigned_value(op: Option<BinOp>, target: &Expr, source: &Expr) -> Result<String, ExprError> {
    match op {
        None => value(source),
        Some(op) => binary(op, target, source),
    }
}

fn member(object: &Expr, property: &str) -> Result<String, ExprError> {
    // `Math` is not an object in the datamodel — it is ECMAScript's
    // namespace, and Lua spells its members differently.
    if let Expr::Ident(name) = object {
        if name == "Math" {
            // ECMA-262 15.8.1, all eight. Lua's `math` table carries one
            // of them under a name of its own and computes the rest, so
            // the arms are the definition and `builtins::MATH_CONSTANTS`
            // is the membership — `every_math_constant_is_lowered` binds
            // the two so a constant cannot be listed and left unlowered.
            return match property {
                "PI" => Ok("math.pi".to_string()),
                "E" => Ok("math.exp(1)".to_string()),
                "LN2" => Ok("math.log(2)".to_string()),
                "LN10" => Ok("math.log(10)".to_string()),
                "LOG2E" => Ok("(1 / math.log(2))".to_string()),
                "LOG10E" => Ok("(1 / math.log(10))".to_string()),
                "SQRT1_2" => Ok("math.sqrt(0.5)".to_string()),
                "SQRT2" => Ok("math.sqrt(2)".to_string()),
                // A member that is a function is a member this datamodel
                // cannot hand out as a value: it has no first-class
                // functions, so the reach is a construct rejection rather
                // than a missing name.
                other if builtins::MATH_FUNCTIONS.contains(&other) => {
                    Err(ExprError::UnsupportedConstruct {
                        construct: format!("Math.{other}"),
                    })
                }
                other => Err(builtins::unknown_member(builtins::Namespace::Math, other)
                    .expect("a member in neither list is unknown")),
            };
        }
        // The other two namespaces are ordinary Lua tables, so a member
        // read is an ordinary field access — but the membership question
        // is the same one, and only `Math` was being asked it. A read of
        // `JSON.serialize` reached the engine as a field of the table
        // this repository installs itself, where it is `nil`: the
        // vocabulary is a fact here, exactly as it is for the call form
        // `unsupported_member` already answers.
        if let Some(namespace) = builtins::Namespace::from_ident(name) {
            if let Some(refusal) = builtins::unknown_member(namespace, property) {
                return Err(refusal);
            }
            return Ok(format!("{name}.{property}"));
        }
    }
    // ECMAScript's `.length` is Lua's `#` for both strings and arrays,
    // which are the two things carrying a length in this datamodel.
    if property == "length" {
        return Ok(format!("#{}", operand(object)?));
    }
    if is_lua_ident(property) {
        return Ok(format!("{}.{property}", operand(object)?));
    }
    Ok(format!("{}[{}]", operand(object)?, lua_string(property)))
}

/// `obj[k]` — where the base of the index has to be decided.
///
/// SCE stores an ECMAScript Array as a 1-based Lua sequence, so a numeric
/// index moves by one. A literal settles it here; anything else is settled
/// at runtime by `_scxml_index`, because whether `a[i]` addresses an array
/// element or an object property depends on what `a` and `i` hold.
///
/// A *string* literal never reaches here: [`super::parser`] folds it into
/// [`Expr::Member`], because ECMA-262 11.2.1 makes `a['k']` and `a.k` the
/// same operation and this datamodel's rules are stated about the property
/// being named. [`member`] emits the same `a["k"]` for a key it has no
/// rule for.
fn index_access(object: &Expr, index: &Expr) -> Result<String, ExprError> {
    if let Expr::Number(text) = index {
        if let Ok(n) = text.parse::<u64>() {
            return Ok(format!("{}[{}]", operand(object)?, n + 1));
        }
    }
    Ok(format!(
        "_scxml_index({}, {})",
        value(object)?,
        value(index)?
    ))
}

fn call(callee: &Expr, args: &[Expr]) -> Result<String, ExprError> {
    let mut emitted = Vec::with_capacity(args.len());
    for arg in args {
        emitted.push(value(arg)?);
    }

    // A call on a name that holds a value. The receiver is not consulted
    // and does not need to be: `_sessionid` is bound by the session and
    // `.length` is lowered as a property for every receiver a few lines
    // up, so neither name can be reaching an author's function here.
    if let Expr::Ident(name) = callee {
        if let Some(refusal) = builtins::uncallable_global(name, args.len()) {
            return Err(refusal);
        }
        // A namespace stands in callee position only as the receiver of
        // a member call, which the arm below answers. Written as the
        // call itself it is neither a function nor a value this
        // datamodel hands out, and passing it through emitted `Math()`
        // into Lua, where nothing binds the name.
        if let Some(refusal) = builtins::uncallable_namespace(name) {
            return Err(refusal);
        }
    }

    if let Expr::Member { object, property } = callee {
        if let Expr::Ident(name) = object.as_ref() {
            if let Some(namespace) = builtins::Namespace::from_ident(name) {
                // A namespace this repository installs, so its member set
                // is a fact and a member outside it is a mistake rather
                // than an unknown. `JSON.serialize` used to be emitted
                // verbatim and reach a nil at runtime.
                if let Some(refusal) = builtins::unsupported_member(namespace, property, args.len())
                {
                    return Err(refusal);
                }
                if namespace == builtins::Namespace::Math {
                    return math_call(property, &emitted);
                }
                // `JSON` and `Object` are ordinary Lua tables, so their
                // members are ordinary field calls.
                return Ok(format!("{name}.{property}({})", emitted.join(", ")));
            }
            // A field of a system variable, which the specification's
            // internal structure of events fills with a value, never
            // with a function. The receiver decides this one — `name` and
            // `data` are ordinary words that an author's own object may
            // hold a function under — and only the fields the clause
            // names are refused, so `_event.raw` stays whatever the I/O
            // processor made it.
            if let Some(refusal) = builtins::uncallable_system_field(name, property, args.len()) {
                return Err(refusal);
            }
        }
        // `member` lowers `.length` to Lua's `#` whatever the receiver
        // holds, so the call form cannot be an author's own method: the
        // property this datamodel provides is what the name reaches, and
        // calling it is what went wrong.
        if let Some(refusal) = builtins::uncallable_property(property, args.len()) {
            return Err(refusal);
        }
        let receiver = operand(object)?;
        match property.as_str() {
            // The engines install these under names that take the receiver
            // as their first argument, so a method call becomes a plain one.
            "indexOf" => {
                let mut all = vec![receiver];
                all.extend(emitted);
                return Ok(format!("_indexOf({})", all.join(", ")));
            }
            // `[].concat(a, b)` is a left fold: each argument is appended to
            // the accumulated array. The engines' `_concat` takes two, so
            // the fold is spelled out here rather than assumed away — the
            // rewriter this replaced passed all three through and silently
            // dropped the last.
            "concat" => {
                let mut folded = receiver;
                for arg in emitted {
                    folded = format!("_concat({folded}, {arg})");
                }
                return Ok(folded);
            }
            // `push` returns the new length, so it cannot be a bare
            // `table.insert` in value position.
            "push" => {
                let mut inserts = String::new();
                for arg in &emitted {
                    inserts.push_str(&format!("table.insert(__t, {arg}) "));
                }
                return Ok(format!(
                    "(function() local __t = {receiver} {inserts}return #__t end)()"
                ));
            }
            "join" => {
                let separator = emitted.first().cloned().unwrap_or_else(|| "\",\"".into());
                return Ok(format!("table.concat({receiver}, {separator})"));
            }
            "toString" => return Ok(format!("_scxml_tostring({receiver})")),
            // ECMA-262 15.5.4 String.prototype, in the shared
            // `ecma_semantics.lua` rather than as another native per engine.
            // Emitted receiver-first for the same reason `_indexOf` is: Lua's
            // own string library has neither these names nor these index
            // conventions, and `"abc".charAt(1)` is not valid Lua at all.
            "substring" | "charAt" | "toLowerCase" | "toUpperCase" | "split" | "replace"
            | "slice" | "sort" | "reverse" => {
                let helper = match property.as_str() {
                    "substring" => "_scxml_substring",
                    "charAt" => "_scxml_charat",
                    "toLowerCase" => "_scxml_tolowercase",
                    "toUpperCase" => "_scxml_touppercase",
                    "replace" => "_scxml_replace",
                    // One helper for both receivers: ECMA-262 gives `slice` to
                    // String (15.5.4.13) and to Array (15.4.4.10) with the same
                    // index rules, and which one a receiver is cannot be
                    // decided here — the same reason `_indexOf` takes both.
                    "slice" => "_scxml_slice",
                    "sort" => "_scxml_sort",
                    "reverse" => "_scxml_reverse",
                    _ => "_scxml_split",
                };
                let mut all = vec![receiver];
                all.extend(emitted);
                return Ok(format!("{helper}({})", all.join(", ")));
            }
            method if DOM_METHODS.contains(&method) => {
                return Ok(format!("{receiver}:{method}({})", emitted.join(", ")));
            }
            _ => {
                // A standard method none of the arms above lowers.
                // Falling through to a field call is what let
                // `words.map(...)` generate cleanly on every backend and
                // die at runtime, so the name is answered here rather
                // than by the interpreter.
                if let Some(refusal) = builtins::unsupported_method(property) {
                    return Err(refusal);
                }
                // An ordinary method on an author's object. `this` is not
                // bound: the datamodel has no prototype chain, and a
                // function stored in a field is called as a field.
                return Ok(format!("{receiver}.{property}({})", emitted.join(", ")));
            }
        }
    }

    Ok(format!("{}({})", operand(callee)?, emitted.join(", ")))
}

fn math_call(name: &str, args: &[String]) -> Result<String, ExprError> {
    // `Math.pow(a, b)` is Lua's `^`; the rest are `math.<same name>`.
    if name == "pow" {
        if args.len() != 2 {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!("Math.pow with {} argument(s)", args.len()),
            });
        }
        return Ok(format!("(({}) ^ ({}))", args[0], args[1]));
    }
    // `Math.round` is not `math.floor(x + 0.5)` inline: ECMA-262 15.8.2.15
    // sends a half toward +Infinity and hands NaN and the infinities straight
    // back, and Lua has no rounding function at all. The shared library owns
    // that clause so six engines cannot each pick a habit.
    if name == "round" {
        return Ok(format!("_scxml_round({})", args.join(", ")));
    }
    // Everything else in `Math` is `math.<same name>`. The membership
    // question was already answered by `builtins::unsupported_member`
    // before this was called, so a name arriving here is one Lua's own
    // `math` table carries under the same spelling.
    debug_assert!(
        builtins::MATH_FUNCTIONS.contains(&name),
        "Math.{name} reached the emitter without passing the namespace check"
    );
    Ok(format!("math.{name}({})", args.join(", ")))
}

fn function_literal(params: &[String], body: &[Stmt]) -> Result<String, ExprError> {
    for param in params {
        if is_lua_keyword(param) {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!("parameter named '{param}' (a Lua keyword)"),
            });
        }
    }
    let scope = Scope {
        depth: 1,
        in_function: true,
    };
    let mut rendered = String::new();
    for stmt in body {
        statement(stmt, scope, &mut rendered)?;
    }
    // A function that writes to `this` is a constructor: ECMAScript's `new`
    // hands it a fresh object and yields that object. With no `new` operator
    // in Lua, the function builds the table and returns it, which is why
    // `new F()` emits as a plain call.
    if mentions_this(body) {
        return Ok(format!(
            "function({}) local self = {{}} {} return self end",
            params.join(", "),
            rendered.trim()
        ));
    }
    Ok(format!(
        "function({}) {} end",
        params.join(", "),
        rendered.trim()
    ))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Statements
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Clone, Copy)]
struct Scope {
    depth: usize,
    /// Whether this statement list is inside a function body. At the top
    /// level of a `<script>` the SCXML datamodel *is* the global scope, so
    /// `var x` there declares a datamodel variable rather than a local one
    /// — W3C test 302 relies on it (`Var1 = 1` in a script, read by a later
    /// `cond`).
    in_function: bool,
}

fn statement(stmt: &Stmt, scope: Scope, out: &mut String) -> Result<(), ExprError> {
    let pad = "  ".repeat(scope.depth);
    match stmt {
        Stmt::Empty => {}
        Stmt::Expr(expr) => {
            out.push_str(&pad);
            out.push_str(&expression_statement(expr)?);
            out.push('\n');
        }
        Stmt::VarDecl(bindings) => {
            for (name, init) in bindings {
                if scope.in_function && is_lua_keyword(name) {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!("local variable named '{name}' (a Lua keyword)"),
                    });
                }
                let keyword = if scope.in_function { "local " } else { "" };
                let target = if scope.in_function {
                    name.clone()
                } else {
                    lua_ident_read(name)
                };
                match init {
                    Some(expr) => {
                        out.push_str(&format!("{pad}{keyword}{target} = {}\n", value(expr)?))
                    }
                    // `var x;` binds x to undefined. At the top level that
                    // is already its state, and writing `x = nil` there
                    // would erase a value the datamodel had put in it.
                    None => {
                        if scope.in_function {
                            out.push_str(&format!("{pad}local {target}\n"));
                        }
                    }
                }
            }
        }
        Stmt::If {
            condition: cond,
            consequent,
            alternate,
        } => {
            // The parser folds a bare `{ … }` block into an `if true`; emit
            // it as the `do … end` it actually is.
            if *cond == Expr::Bool(true) && alternate.is_empty() {
                out.push_str(&format!("{pad}do\n"));
                emit_block(consequent, scope, out)?;
                out.push_str(&format!("{pad}end\n"));
                return Ok(());
            }
            out.push_str(&format!("{pad}if {} then\n", condition(cond)?));
            emit_block(consequent, scope, out)?;
            if !alternate.is_empty() {
                out.push_str(&format!("{pad}else\n"));
                emit_block(alternate, scope, out)?;
            }
            out.push_str(&format!("{pad}end\n"));
        }
        Stmt::While {
            condition: cond,
            body,
        } => {
            out.push_str(&format!("{pad}while {} do\n", condition(cond)?));
            emit_loop_body(body, scope, out)?;
            out.push_str(&format!("{pad}end\n"));
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            // ECMAScript's `for` updates after the body and before the next
            // test, including when the body took a `continue`. Lua's
            // numeric `for` cannot express an arbitrary update, so this is a
            // `while` with the update at the end of the body and the
            // continue label in front of it.
            out.push_str(&format!("{pad}do\n"));
            let inner = Scope {
                depth: scope.depth + 1,
                ..scope
            };
            if let Some(init) = init {
                statement(init, inner, out)?;
            }
            let test_src = match test {
                Some(expr) => condition(expr)?,
                None => "true".to_string(),
            };
            let inner_pad = "  ".repeat(inner.depth);
            out.push_str(&format!("{inner_pad}while {test_src} do\n"));
            let body_scope = Scope {
                depth: inner.depth + 1,
                ..scope
            };
            for stmt in body {
                statement(stmt, body_scope, out)?;
            }
            if has_continue(body) {
                out.push_str(&format!(
                    "{}::__sce_continue::\n",
                    "  ".repeat(body_scope.depth)
                ));
            }
            if let Some(update) = update {
                out.push_str(&format!(
                    "{}{}\n",
                    "  ".repeat(body_scope.depth),
                    expression_statement(update)?
                ));
            }
            out.push_str(&format!("{inner_pad}end\n"));
            out.push_str(&format!("{pad}end\n"));
        }
        Stmt::ForIn {
            name,
            object,
            body,
            declares: _,
        } => {
            if is_lua_keyword(name) {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("loop variable named '{name}' (a Lua keyword)"),
                });
            }
            out.push_str(&format!(
                "{pad}for {name} in pairs({}) do\n",
                value(object)?
            ));
            emit_loop_body(body, scope, out)?;
            out.push_str(&format!("{pad}end\n"));
        }
        Stmt::Return(expr) => match expr {
            Some(expr) => out.push_str(&format!("{pad}return {}\n", value(expr)?)),
            None => out.push_str(&format!("{pad}return\n")),
        },
        Stmt::FunctionDecl { name, params, body } => {
            if is_lua_keyword(name) {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("function named '{name}' (a Lua keyword)"),
                });
            }
            let literal = function_literal(params, body)?;
            let keyword = if scope.in_function { "local " } else { "" };
            out.push_str(&format!("{pad}{keyword}{name} = {literal}\n"));
        }
        Stmt::Break => out.push_str(&format!("{pad}break\n")),
        Stmt::Continue => out.push_str(&format!("{pad}goto __sce_continue\n")),
    }
    Ok(())
}

fn emit_block(stmts: &[Stmt], scope: Scope, out: &mut String) -> Result<(), ExprError> {
    let inner = Scope {
        depth: scope.depth + 1,
        ..scope
    };
    for stmt in stmts {
        statement(stmt, inner, out)?;
    }
    Ok(())
}

/// A loop body, with the `continue` label appended when the body uses one.
fn emit_loop_body(stmts: &[Stmt], scope: Scope, out: &mut String) -> Result<(), ExprError> {
    emit_block(stmts, scope, out)?;
    if has_continue(stmts) {
        out.push_str(&format!(
            "{}::__sce_continue::\n",
            "  ".repeat(scope.depth + 1)
        ));
    }
    Ok(())
}

/// An expression in statement position. Lua accepts only calls and
/// assignments there, so the side-effecting forms are emitted directly
/// rather than through the immediately-called function `value` would use.
fn expression_statement(expr: &Expr) -> Result<String, ExprError> {
    Ok(match expr {
        Expr::Assign {
            op,
            target,
            value: v,
        } => {
            format!(
                "{} = {}",
                emit_location(target)?,
                assigned_value(*op, target, v)?
            )
        }
        Expr::Update { op, target, .. } => {
            let location = emit_location(target)?;
            let step = match op {
                UpdateOp::Inc => "+ 1",
                UpdateOp::Dec => "- 1",
            };
            format!("{location} = _scxml_tonumber({location}) {step}")
        }
        Expr::Call { .. } | Expr::New { .. } => value(expr)?,
        // Anything else has no effect, but discarding it silently would
        // hide a typo; binding it keeps the expression evaluated and the
        // chunk valid Lua.
        other => format!("local _ = {}", value(other)?),
    })
}

fn has_continue(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Stmt::Continue => true,
        Stmt::If {
            consequent,
            alternate,
            ..
        } => has_continue(consequent) || has_continue(alternate),
        // A `continue` inside a nested loop belongs to that loop.
        _ => false,
    })
}

fn mentions_this(stmts: &[Stmt]) -> bool {
    fn in_expr(expr: &Expr) -> bool {
        match expr {
            Expr::This => true,
            Expr::Array(items) => items.iter().any(in_expr),
            Expr::Object(props) => props.iter().any(|(_, v)| in_expr(v)),
            Expr::Member { object, .. } => in_expr(object),
            Expr::Index { object, index } => in_expr(object) || in_expr(index),
            Expr::Call { callee, args } | Expr::New { callee, args } => {
                in_expr(callee) || args.iter().any(in_expr)
            }
            Expr::Unary { operand, .. } => in_expr(operand),
            Expr::Update { target, .. } => in_expr(target),
            Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
                in_expr(left) || in_expr(right)
            }
            Expr::Conditional {
                condition,
                consequent,
                alternate,
            } => in_expr(condition) || in_expr(consequent) || in_expr(alternate),
            Expr::Assign { target, value, .. } => in_expr(target) || in_expr(value),
            // A nested function has its own `this`.
            Expr::Function { .. } => false,
            _ => false,
        }
    }
    stmts.iter().any(|stmt| match stmt {
        Stmt::Expr(expr) | Stmt::Return(Some(expr)) => in_expr(expr),
        Stmt::VarDecl(bindings) => bindings.iter().any(|(_, init)| match init {
            Some(expr) => in_expr(expr),
            None => false,
        }),
        Stmt::If {
            condition,
            consequent,
            alternate,
        } => in_expr(condition) || mentions_this(consequent) || mentions_this(alternate),
        Stmt::While { condition, body } => in_expr(condition) || mentions_this(body),
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            init.as_ref()
                .is_some_and(|s| mentions_this(&[(**s).clone()]))
                || test.as_ref().is_some_and(in_expr)
                || update.as_ref().is_some_and(in_expr)
                || mentions_this(body)
        }
        Stmt::ForIn { object, body, .. } => in_expr(object) || mentions_this(body),
        _ => false,
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Lexical rendering
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

fn is_lua_keyword(name: &str) -> bool {
    LUA_KEYWORDS.contains(&name)
}

fn is_lua_ident(name: &str) -> bool {
    !name.is_empty()
        && !is_lua_keyword(name)
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A free identifier. ECMAScript's identifier set is wider than Lua's — a
/// datamodel variable may be spelled `end` or `repeat` — and those names
/// live in the globals table, which `_ENV` addresses by string.
fn lua_ident_read(name: &str) -> String {
    if is_lua_ident(name) {
        return name.to_string();
    }
    format!("_ENV[{}]", lua_string(name))
}

/// Render a string as a Lua literal from its characters, escaping what Lua
/// requires and nothing else. Non-ASCII passes through as UTF-8 bytes: Lua
/// 5.2 has no `\u{}` escape, and the bytes are what every engine stores.
fn lua_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\{}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render a numeric literal. Lua reads decimal, hexadecimal and exponent
/// forms the same way ECMAScript does; ECMAScript's binary and octal
/// prefixes have no Lua spelling, so they are converted to the value they
/// denote rather than passed through as source Lua would misread.
fn lua_number(text: &str) -> Result<String, ExprError> {
    let lowered = text.to_ascii_lowercase();
    let radix = lowered
        .strip_prefix("0b")
        .map(|digits| (digits.to_string(), 2))
        .or_else(|| {
            lowered
                .strip_prefix("0o")
                .map(|digits| (digits.to_string(), 8))
        });
    let Some((digits, base)) = radix else {
        return Ok(text.to_string());
    };
    match u64::from_str_radix(&digits, base) {
        Ok(v) => Ok(v.to_string()),
        Err(_) => Err(ExprError::UnsupportedConstruct {
            construct: format!("numeric literal '{text}'"),
        }),
    }
}

fn describe(expr: &Expr) -> &'static str {
    match expr {
        Expr::Number(_) => "number literal",
        Expr::Str(_) => "string literal",
        Expr::Bool(_) => "boolean literal",
        Expr::Nullish => "null",
        Expr::Ident(_) => "identifier",
        Expr::This => "this",
        Expr::Array(_) => "array literal",
        Expr::Object(_) => "object literal",
        Expr::Member { .. } => "member access",
        Expr::Index { .. } => "index access",
        Expr::Call { .. } => "call",
        Expr::New { .. } => "new expression",
        Expr::Unary { .. } => "unary expression",
        Expr::Update { .. } => "update expression",
        Expr::Binary { .. } => "binary expression",
        Expr::Logical { .. } => "logical expression",
        Expr::Conditional { .. } => "conditional expression",
        Expr::Function { .. } => "function expression",
        Expr::Assign { .. } => "assignment",
    }
}
