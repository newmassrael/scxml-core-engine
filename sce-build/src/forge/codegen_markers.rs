//! Structured codegen-emit markers for reverse-linkage.
//!
//! Templates declare structural invariants they emit via inline
//! comments shaped `SCE-EMIT: kind=<kind>`. Validators read these
//! declarations rather than scanning for the literal emitted code —
//! a template refactor (whitespace, type-param rename, line break)
//! does not silently break the validator as long as the marker
//! comment is preserved with the same kind.
//!
//! Prior to this helper, five validators in `generator.rs`
//! (`check_listener_sibling_emitted_rust`, `check_listener_sibling_emitted_c`,
//! `check_reassembly_peer_id_zid_invariant_rust`,
//! `check_reassembly_peer_id_zid_invariant_c11`,
//! `check_inter_pool_padding_invariant`) reached into the rendered
//! template output by literal-substring scan — pure ownership-inversion:
//! the Rust producer of template context was asserting the
//! Jinja template's exact emission shape, and a template change with
//! incidentally-preserved substring would pass the check while breaking
//! the structural invariant.
//!
//! Marker grammar (case-sensitive, ASCII):
//! ```text
//!   SCE-EMIT: kind=<kind-identifier>
//! ```
//! `<kind-identifier>` is a dotted slug naming the structural invariant
//! the marker attests to (e.g. `link.listener.established-session`,
//! `reassembly.peer-id-zid`, `mem.inter-pool-padding`). The kind name
//! is the documentation of what the template promises to emit — adding
//! a new kind requires a matching template change and a matching
//! validator check.
//!
//! Templates emit the marker inside any comment form valid for the
//! target language (Rust `//`, C `/* */`, jinja `{# #}` etc.). The
//! helper does not require a specific comment style — only that the
//! literal `SCE-EMIT: kind=<kind>` substring is present in the
//! rendered output.

/// Returns `true` iff `rendered` contains the structured emit marker
/// for the given `kind`. Used by validators to check that a template
/// promised to emit a particular structural invariant.
pub(crate) fn contains_emit_marker(rendered: &str, kind: &str) -> bool {
    let needle = format!("SCE-EMIT: kind={}", kind);
    rendered.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_rust_line_comment() {
        let r = "// SCE-EMIT: kind=link.listener.established-session\npub struct X;";
        assert!(contains_emit_marker(r, "link.listener.established-session"));
    }

    #[test]
    fn matches_c_block_comment() {
        let r = "/* SCE-EMIT: kind=reassembly.peer-id-zid */\ntypedef uint8_t pid_t[16];";
        assert!(contains_emit_marker(r, "reassembly.peer-id-zid"));
    }

    #[test]
    fn matches_linker_block_comment() {
        let r = "/* SCE-EMIT: kind=mem.inter-pool-padding pool=pkt */\n. = ALIGN(64);";
        assert!(contains_emit_marker(r, "mem.inter-pool-padding"));
    }

    #[test]
    fn rejects_missing_kind() {
        let r = "// SCE-EMIT: kind=other.invariant\n";
        assert!(!contains_emit_marker(
            r,
            "link.listener.established-session"
        ));
    }

    #[test]
    fn rejects_partial_match() {
        // `link.listener` is a prefix but not the full kind — a partial
        // match must not pass.
        let r = "// SCE-EMIT: kind=link.listener\n";
        assert!(!contains_emit_marker(
            r,
            "link.listener.established-session"
        ));
    }

    #[test]
    fn rejects_absent_marker() {
        let r = "pub struct XEstablishedSession;";
        assert!(!contains_emit_marker(
            r,
            "link.listener.established-session"
        ));
    }
}
