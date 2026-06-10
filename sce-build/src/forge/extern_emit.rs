// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `<sce:extern>` per-language emit — watching-zenoh RFC §5.I.
//
// 3-language scope (Rust + C11 + Cpp); Kotlin/Go/Python
// reject `<sce:extern>` via the existing
// `codegen/mcu-class-kind-on-non-mcu-language` family (the
// rejection lives in lib.rs's compile_forge_* gate, not here).
//
// Emit shapes:
//
// | Backend | Shape |
// |---|---|
// | Rust | `extern "C" { fn name(p0: T1, p1: T2) -> R; }` block |
// | C11  | `extern R name(C(T1) p0, C(T2) p1);` forward decl |
// | Cpp  | `extern "C" R name(C(T1) p0, C(T2) p1);` forward decl |
//
// `<sce:extern sig="...">` carries the canonical Rust-style signature
// (registry source of truth). Rust emit is verbatim with positional
// parameter names (`p0, p1, ...`) since the registry stores type-only
// signatures; C11/Cpp emit translates Rust types to C types via the
// closed map below — every type form present in the §5.I 101-symbol
// baseline is enumerated, so an unknown form is a hard error
// (`UnknownType`) rather than a silent passthrough.
//
// The translator is closed-set on purpose: the spec defines the
// whitelist (and target plugins extending it via the plugin loader
// MAY introduce vendor types like `irq_state_t`, which the closed set
// passes through as-is so plugin authors do not have to register every
// vendor type with SCE).

use crate::forge::model::ExternDeclaration;

/// Parsed view of one `<sce:extern sig="...">` triple. Canonicalises
/// the Rust-style sig string into ordered parameter types and an
/// optional return type so per-language emitters can format without
/// re-parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSignature {
    /// Parameter types in order (Rust-style spelling, e.g. `*const u32`,
    /// `usize`, `irq_state_t`). Empty for `()` or `() -> T`.
    pub params: Vec<String>,
    /// Return type, `None` for `()` (no return) and
    /// `Some("usize")` / `Some("u32")` / etc. otherwise.
    pub ret: Option<String>,
}

/// Sig-parse failure modes. Surfaces are bounded by the closed
/// signature shape: parens-wrapped param list + optional `-> T` return
/// suffix. Any deviation is a hard error so the registry/plugin entries
/// stay byte-byte verifiable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SigParseError {
    /// Sig does not start with `(`.
    #[error("expected `(` at start of signature, got `{got}`")]
    MissingOpenParen { got: String },
    /// No matching `)` after the opening `(`.
    #[error("expected `)` to close parameter list in `{sig}`")]
    MissingCloseParen { sig: String },
    /// Trailing tokens after the close paren that are not the
    /// `-> T` return-type suffix.
    #[error("trailing garbage after parameter list in `{sig}`: `{trailing}`")]
    TrailingGarbage { sig: String, trailing: String },
    /// `->` without a return type after it.
    #[error("`->` without return type in `{sig}`")]
    EmptyReturn { sig: String },
}

/// Parse a Rust-style signature into [`ParsedSignature`]. The grammar
/// recognised here is a strict subset matching the §5.I baseline +
/// target plugin entries:
///
///   sig         = "(" param_list? ")" return_clause?
///   param_list  = type ( "," type )*
///   return_clause = "->" type
///   type        = (any token sequence not containing `,` or `)` or `->`)
///
/// Whitespace around params/return is normalized; the type body is
/// preserved verbatim so the per-language emitter can map known forms
/// (`*const u32` → `const uint32_t*`, etc.) without re-tokenising.
pub fn parse_signature(sig: &str) -> Result<ParsedSignature, SigParseError> {
    let trimmed = sig.trim();
    let after_open = trimmed
        .strip_prefix('(')
        .ok_or_else(|| SigParseError::MissingOpenParen {
            got: trimmed
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default(),
        })?;
    // Find the matching close-paren. Sigs in v1 do not nest parens
    // (no function-pointer args), so the first `)` closes the list.
    let close_idx = after_open
        .find(')')
        .ok_or_else(|| SigParseError::MissingCloseParen {
            sig: sig.to_string(),
        })?;
    let params_body = &after_open[..close_idx];
    let after_close = after_open[close_idx + 1..].trim_start();

    let params = if params_body.trim().is_empty() {
        Vec::new()
    } else {
        params_body
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    let ret = if let Some(rest) = after_close.strip_prefix("->") {
        let r = rest.trim();
        if r.is_empty() {
            return Err(SigParseError::EmptyReturn {
                sig: sig.to_string(),
            });
        }
        Some(r.to_string())
    } else if !after_close.is_empty() {
        return Err(SigParseError::TrailingGarbage {
            sig: sig.to_string(),
            trailing: after_close.to_string(),
        });
    } else {
        None
    };

    Ok(ParsedSignature { params, ret })
}

/// Translate one Rust-style type to its C/Cpp equivalent. Closed set
/// covers every type present in the §5.I 101-symbol baseline:
///
/// - Pointer forms: `*const T` → `const C(T)*` ; `*mut T` → `C(T)*`
/// - Integer widths: `u8`/`u16`/`u32`/`u64` → `uint8_t`/`uint16_t`/
///   `uint32_t`/`uint64_t`
/// - `usize` → `size_t`
/// - `c_void` → `void`
/// - `bool` → `bool`
/// - Vendor-passthrough (e.g. `irq_state_t`): identifier preserved
///   verbatim. Plugin entries that introduce new vendor types ride this
///   passthrough — SCE does not register every vendor typedef.
///
/// Returns `None` for malformed input the closed set does not cover,
/// so the emitter can surface a `GenerateError::UnsupportedFeature`
/// rather than silently emit `???`.
pub fn rust_type_to_c(rust_ty: &str) -> Option<String> {
    let t = rust_ty.trim();
    if let Some(inner) = t.strip_prefix("*const ") {
        let c_inner = rust_type_to_c(inner)?;
        return Some(format!("const {c_inner}*"));
    }
    if let Some(inner) = t.strip_prefix("*mut ") {
        let c_inner = rust_type_to_c(inner)?;
        return Some(format!("{c_inner}*"));
    }
    Some(match t {
        "u8" => "uint8_t".to_string(),
        "u16" => "uint16_t".to_string(),
        "u32" => "uint32_t".to_string(),
        "u64" => "uint64_t".to_string(),
        "i8" => "int8_t".to_string(),
        "i16" => "int16_t".to_string(),
        "i32" => "int32_t".to_string(),
        "i64" => "int64_t".to_string(),
        "usize" => "size_t".to_string(),
        "isize" => "ptrdiff_t".to_string(),
        "c_void" => "void".to_string(),
        "bool" => "bool".to_string(),
        // Vendor typedef passthrough — `irq_state_t`, future
        // `cortex_m_pri_t`, etc. The plugin author is responsible for
        // making the typedef visible at the include site.
        ident
            if ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !ident.is_empty()
                && !ident.chars().next().unwrap().is_ascii_digit() =>
        {
            ident.to_string()
        }
        _ => return None,
    })
}

/// One emit-ready entry per `<sce:extern>` declaration. Carries the
/// pre-computed Rust signature (verbatim) + parameter triples for
/// C11/Cpp template iteration. Templates do not call back into
/// type-translation logic; the translator runs once at codegen time
/// and the result rides the template context as a flat shape.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExternEmit {
    /// Symbol name (lookup key against registry).
    pub name: String,
    /// ABI as authored (`c` / `rust`). Today every baseline
    /// entry is `c`; carried verbatim so future ABI extensions ride
    /// without a template change.
    pub abi: String,
    /// Crate that hosts the implementation. Defaults to the registry
    /// entry's canonical crate (today, `sce_intrinsics_runtime`);
    /// plugin entries may override per-symbol.
    pub crate_name: String,
    /// Rust-style signature verbatim from the registry. Used by the
    /// Rust template's `extern "C" { fn name SIG; }` emit so authors
    /// reading the generated file see exactly what they wrote in
    /// `<sce:extern sig="...">`.
    pub rust_sig: String,
    /// Rust-formatted parameter list (`p0: *const u32, p1: usize`).
    /// Pre-computed so the template does not need a Jinja2 zip.
    pub rust_params: String,
    /// Rust return-type clause (`" -> u32"` with leading space, or
    /// empty string for `()`). Pre-computed for direct template
    /// concatenation.
    pub rust_ret: String,
    /// C-formatted return type (`uint32_t`, `void`). Plain identifier
    /// suitable for `extern <ret> name(...)`.
    pub c_ret: String,
    /// C-formatted parameter list (`const uint32_t* p0, size_t p1`).
    /// `void` when there are no parameters (matches C11 convention for
    /// "explicitly no args" in extern decls).
    pub c_params: String,
}

/// Errors raised when translating an [`ExternDeclaration`] into an
/// emit-ready [`ExternEmit`]. Distinct from [`SigParseError`] so the
/// caller can attach the offending symbol name to the diagnostic
/// surface.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExternEmitError {
    /// Sig text malformed — wraps the underlying [`SigParseError`].
    #[error("`<sce:extern name=\"{name}\">` has malformed sig: {source}")]
    Sig {
        name: String,
        #[source]
        source: SigParseError,
    },
    /// One of the parameter or return types is outside the closed
    /// translator set. Usually means a plugin author used a Rust type
    /// SCE has not learned to translate — register the type as a
    /// vendor passthrough by giving it a plain identifier name.
    #[error(
        "`<sce:extern name=\"{name}\">` carries unsupported type `{ty}`; \
         the §5.I translator covers `*const/*mut T`, `u8`-`u64`, `i8`-`i64`, \
         `usize`, `isize`, `c_void`, `bool`, and plain identifier vendor types"
    )]
    UnsupportedType { name: String, ty: String },
}

/// Promote one [`ExternDeclaration`] to an emit-ready [`ExternEmit`].
/// Runs sig parsing once and pre-formats the per-language fragments
/// so templates remain logic-free.
pub fn build_extern_emit(decl: &ExternDeclaration) -> Result<ExternEmit, ExternEmitError> {
    let parsed = parse_signature(&decl.sig).map_err(|source| ExternEmitError::Sig {
        name: decl.name.clone(),
        source,
    })?;

    // Rust formatting: `p0: T0, p1: T1` for params; ` -> R` for return.
    let rust_params = parsed
        .params
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("p{i}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    let rust_ret = match &parsed.ret {
        Some(r) => format!(" -> {r}"),
        None => String::new(),
    };

    // C/Cpp formatting: `const uint32_t* p0, size_t p1` for params;
    // bare type for return. Parameterless extern decls in C11 use
    // `void` to match the strict-prototype convention (otherwise C
    // treats `()` as "unspecified parameters").
    let c_params = if parsed.params.is_empty() {
        "void".to_string()
    } else {
        let mut parts = Vec::with_capacity(parsed.params.len());
        for (i, ty) in parsed.params.iter().enumerate() {
            let c_ty = rust_type_to_c(ty).ok_or_else(|| ExternEmitError::UnsupportedType {
                name: decl.name.clone(),
                ty: ty.clone(),
            })?;
            parts.push(format!("{c_ty} p{i}"));
        }
        parts.join(", ")
    };
    let c_ret = match &parsed.ret {
        Some(r) => rust_type_to_c(r).ok_or_else(|| ExternEmitError::UnsupportedType {
            name: decl.name.clone(),
            ty: r.clone(),
        })?,
        None => "void".to_string(),
    };

    Ok(ExternEmit {
        name: decl.name.clone(),
        abi: decl.abi.clone(),
        crate_name: decl.crate_name.clone(),
        rust_sig: decl.sig.clone(),
        rust_params,
        rust_ret,
        c_ret,
        c_params,
    })
}

/// Promote a slice of declarations to emit-ready entries. Short-
/// circuits on the first error so the caller surfaces a single
/// diagnostic per build (plugin authors typically fix one entry at a
/// time).
pub fn build_extern_emit_list(
    decls: &[ExternDeclaration],
) -> Result<Vec<ExternEmit>, ExternEmitError> {
    decls.iter().map(build_extern_emit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(name: &str, sig: &str) -> ExternDeclaration {
        ExternDeclaration {
            name: name.to_string(),
            sig: sig.to_string(),
            abi: "c".to_string(),
            crate_name: "sce_intrinsics_runtime".to_string(),
            line: None,
        }
    }

    #[test]
    fn parse_no_args_no_return() {
        let p = parse_signature("()").unwrap();
        assert!(p.params.is_empty());
        assert!(p.ret.is_none());
    }

    #[test]
    fn parse_no_args_with_return() {
        let p = parse_signature("() -> irq_state_t").unwrap();
        assert!(p.params.is_empty());
        assert_eq!(p.ret.as_deref(), Some("irq_state_t"));
    }

    #[test]
    fn parse_atomic_load_signature() {
        let p = parse_signature("(*const u32) -> u32").unwrap();
        assert_eq!(p.params, vec!["*const u32".to_string()]);
        assert_eq!(p.ret.as_deref(), Some("u32"));
    }

    #[test]
    fn parse_cas_signature() {
        let p = parse_signature("(*mut u32, u32, u32) -> u32").unwrap();
        assert_eq!(
            p.params,
            vec!["*mut u32".to_string(), "u32".to_string(), "u32".to_string()]
        );
    }

    #[test]
    fn parse_cache_signature() {
        let p = parse_signature("(*const c_void, usize)").unwrap();
        assert_eq!(p.params.len(), 2);
        assert_eq!(p.ret, None);
    }

    #[test]
    fn parse_irq_save_signature() {
        let p = parse_signature("(irq_state_t)").unwrap();
        assert_eq!(p.params, vec!["irq_state_t".to_string()]);
    }

    #[test]
    fn parse_rejects_missing_open_paren() {
        let err = parse_signature("u32").unwrap_err();
        assert!(matches!(err, SigParseError::MissingOpenParen { .. }));
    }

    #[test]
    fn parse_rejects_missing_close_paren() {
        let err = parse_signature("(u32, u32").unwrap_err();
        assert!(matches!(err, SigParseError::MissingCloseParen { .. }));
    }

    #[test]
    fn parse_rejects_empty_return() {
        let err = parse_signature("(u32) ->").unwrap_err();
        assert!(matches!(err, SigParseError::EmptyReturn { .. }));
    }

    #[test]
    fn parse_rejects_trailing_garbage() {
        let err = parse_signature("(u32) garbage").unwrap_err();
        assert!(matches!(err, SigParseError::TrailingGarbage { .. }));
    }

    #[test]
    fn translate_pointer_const() {
        assert_eq!(rust_type_to_c("*const u32").unwrap(), "const uint32_t*");
        assert_eq!(rust_type_to_c("*const c_void").unwrap(), "const void*");
    }

    #[test]
    fn translate_pointer_mut() {
        assert_eq!(rust_type_to_c("*mut u32").unwrap(), "uint32_t*");
        assert_eq!(rust_type_to_c("*mut c_void").unwrap(), "void*");
    }

    #[test]
    fn translate_widths() {
        assert_eq!(rust_type_to_c("u8").unwrap(), "uint8_t");
        assert_eq!(rust_type_to_c("u16").unwrap(), "uint16_t");
        assert_eq!(rust_type_to_c("u32").unwrap(), "uint32_t");
        assert_eq!(rust_type_to_c("u64").unwrap(), "uint64_t");
        assert_eq!(rust_type_to_c("usize").unwrap(), "size_t");
    }

    #[test]
    fn translate_vendor_passthrough() {
        // Plain identifiers (no leading digit, no special chars) ride
        // through unchanged so plugin authors can add vendor typedefs
        // like `irq_state_t` without registering with SCE.
        assert_eq!(rust_type_to_c("irq_state_t").unwrap(), "irq_state_t");
        assert_eq!(rust_type_to_c("cortex_m_pri_t").unwrap(), "cortex_m_pri_t");
    }

    #[test]
    fn translate_rejects_unknown_form() {
        // Empty string
        assert!(rust_type_to_c("").is_none());
        // Leading digit
        assert!(rust_type_to_c("3foo").is_none());
        // Special chars (would surface as malformed registry entry)
        assert!(rust_type_to_c("foo-bar").is_none());
    }

    #[test]
    fn build_emit_atomic_load_acquire() {
        let d = decl("sce_atomic_load_acquire_u32", "(*const u32) -> u32");
        let e = build_extern_emit(&d).unwrap();
        assert_eq!(e.name, "sce_atomic_load_acquire_u32");
        assert_eq!(e.rust_params, "p0: *const u32");
        assert_eq!(e.rust_ret, " -> u32");
        assert_eq!(e.c_ret, "uint32_t");
        assert_eq!(e.c_params, "const uint32_t* p0");
    }

    #[test]
    fn build_emit_cache_clean() {
        let d = decl("sce_dcache_clean_by_addr", "(*const c_void, usize)");
        let e = build_extern_emit(&d).unwrap();
        assert_eq!(e.c_ret, "void");
        assert_eq!(e.c_params, "const void* p0, size_t p1");
        assert_eq!(e.rust_params, "p0: *const c_void, p1: usize");
        assert_eq!(e.rust_ret, "");
    }

    #[test]
    fn build_emit_irq_save() {
        let d = decl("sce_irq_save", "() -> irq_state_t");
        let e = build_extern_emit(&d).unwrap();
        assert_eq!(e.c_ret, "irq_state_t");
        // Empty parameter list emits `void` per C11 strict-prototype
        // convention.
        assert_eq!(e.c_params, "void");
        assert_eq!(e.rust_params, "");
        assert_eq!(e.rust_ret, " -> irq_state_t");
    }

    #[test]
    fn build_emit_fence() {
        let d = decl("sce_atomic_fence_acquire", "()");
        let e = build_extern_emit(&d).unwrap();
        assert_eq!(e.c_ret, "void");
        assert_eq!(e.c_params, "void");
        assert_eq!(e.rust_params, "");
        assert_eq!(e.rust_ret, "");
    }

    #[test]
    fn build_emit_rejects_malformed_sig() {
        let d = decl("sce_broken", "no parens");
        let err = build_extern_emit(&d).unwrap_err();
        assert!(matches!(err, ExternEmitError::Sig { .. }));
    }

    #[test]
    fn build_emit_list_propagates_error() {
        let decls = vec![
            decl("sce_atomic_load_acquire_u32", "(*const u32) -> u32"),
            decl("sce_broken", "garbage"),
        ];
        let err = build_extern_emit_list(&decls).unwrap_err();
        assert!(matches!(err, ExternEmitError::Sig { name, .. } if name == "sce_broken"));
    }
}
