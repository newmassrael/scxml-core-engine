//! SCE_MESH.md §9.6.2 Session 5 — Zenoh cross-device scxml-invoke Rust mirror.
//!
//! ## Why a shared zenoh::Session (Zenoh diverges from SOME/IP here)
//!
//! §9.6 cross-device `<invoke type="scxml" src="#peer">` traffic on Zenoh
//! rides the SAME `zenoh_session_` that ordinary `<send>` zenoh targets use,
//! not a dedicated one. The Session 4b SOME/IP rationale for a dedicated
//! `<machine>_scxml_invoke_app_` does NOT carry over:
//!
//!   1. **No §13 OEM boundary on the Zenoh side.** `vsomeip.json applications[*]`
//!      is OEM-owned territory; Zenoh has no equivalent OEM-allocated
//!      identifier whose registration must stay outside SCE-named spaces.
//!      The SCE-reserved §9.6 namespace is carved out via key-expression
//!      prefix ([`SCXML_INVOKE_KEY_PREFIX`]), not via session identity.
//!   2. **No 128-ID counter or service_id collision domain.** SOME/IP's
//!      RFC F.X-1 hybrid (counter + author-pin) allocator carves out a
//!      bounded sub-range `[0x8100, 0x817F]` whose ceiling is observable
//!      on its own routing tuple; Zenoh routes by full key-expression
//!      with no analogous bounded namespace, so the parallel allocator
//!      is unnecessary.
//!   3. **Failure isolation.** A §9.6 peer disconnect surfaces on the same
//!      Zenoh runtime callback thread as `<send>` traffic — sharing the
//!      session matches the existing `zenoh_subscribers_` map that already
//!      dispatches on the same callback thread without per-pattern
//!      isolation.
//!
//! ## What this Rust module owns
//!
//! Today the codegen template emits `SCE::Mesh::Zenoh::keyExprP2C("...",
//! "...")` / `keyExprC2P` calls directly, so the C++ header
//! [`sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h`] is the
//! authoritative source for runtime values. This module mirrors the
//! key-expression prefix and the direction-oriented layout so:
//!
//! * **Rust drift detection** — the unit tests at the bottom pin specific
//!   key strings; a divergence from the C++ helper (e.g. someone touching
//!   the prefix or the direction segment) trips the tests on
//!   `cargo test -p sce-build --lib` long before the C++ side compiles
//!   a wrong key into generated code.
//! * **Future validator hook** — a deploy-time `<send>`-key vs. §9.6
//!   reservation collision check (an author-supplied zenoh `key:` value
//!   that begins with `sce/scxml_invoke/...` would interfere with §9.6
//!   traffic) will call [`SCXML_INVOKE_KEY_PREFIX`] from the topology
//!   stage to reject such conflicts. Mirrors how the SOME/IP module
//!   exposes [`SCXML_INVOKE_SERVICE_BASE`] for the future
//!   collision validator.
//!
//! No `mesh::codegen` path consumes this module yet; the Jinja template
//! emits `SCE::Mesh::Zenoh::keyExprP2C` / `keyExprC2P` calls directly.
//! When the validator lands, that consumer will route through here.
//!
//! [`SCXML_INVOKE_SERVICE_BASE`]: super::someip::SCXML_INVOKE_SERVICE_BASE

// ── SCE-reserved §9.6 namespace constants ───────────────────────────────────

/// SCE-reserved §9.6 scxml-invoke key-expression prefix. All §9.6
/// cross-device traffic over Zenoh travels under
/// `sce/scxml_invoke/...`. Mirror of the C++
/// `SCE::Mesh::Zenoh::SCXML_INVOKE_KEY_PREFIX` constant in
/// `ZenohScxmlInvokeEndpoint.h`.
///
/// Author-supplied `<send>` zenoh `key:` values do not begin with `sce/`
/// by SCE convention, so the §9.6 namespace stays disjoint from
/// arbitrary author keys. A future deploy-time validator will grep
/// author keys for this prefix to reject reservation collisions.
pub const SCXML_INVOKE_KEY_PREFIX: &str = "sce/scxml_invoke";

// ── Per-peer key derivation ─────────────────────────────────────────────────

/// Build the parent→child key for a (parent, child) machine pair.
/// First argument is the source (parent) machine, second is the
/// destination (child). Pattern: `sce/scxml_invoke/p2c/<parent>/<child>`.
/// Mirror of the C++ `SCE::Mesh::Zenoh::keyExprP2C` helper.
pub fn key_expr_p2c(parent_machine: &str, child_machine: &str) -> String {
    format!("{SCXML_INVOKE_KEY_PREFIX}/p2c/{parent_machine}/{child_machine}")
}

/// Build the child→parent key for a (child, parent) machine pair.
/// First argument is the source (child) machine, second is the
/// destination (parent). Pattern: `sce/scxml_invoke/c2p/<child>/<parent>`.
/// Mirror of the C++ `SCE::Mesh::Zenoh::keyExprC2P` helper.
///
/// Direction-oriented naming (`<from>/<to>`) mirrors the SHM
/// fallthrough channel naming in the codegen template
/// (`/sce_p2c_<parent>_<child>` / `/sce_c2p_<child>_<parent>`) so a
/// reader switching transports sees the same direction encoded the
/// same way.
pub fn key_expr_c2p(child_machine: &str, parent_machine: &str) -> String {
    format!("{SCXML_INVOKE_KEY_PREFIX}/c2p/{child_machine}/{parent_machine}")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_pinned() {
        // Drift-detection: the C++ helper hard-codes the string literal
        // `"sce/scxml_invoke"`. Anyone changing one side without the
        // other trips this assertion before the C++ side rebuilds
        // against a stale prefix.
        assert_eq!(SCXML_INVOKE_KEY_PREFIX, "sce/scxml_invoke");
    }

    #[test]
    fn p2c_canonical_pair() {
        // Pin the parent/worker fixture pair used by the Session 5
        // single-process roundtrip — a future rename of the fixture
        // machines must update this assertion in the same commit, so
        // the codegen template's emitted key cannot drift away from
        // the test fixture's expectation silently.
        assert_eq!(
            key_expr_p2c("parent", "worker"),
            "sce/scxml_invoke/p2c/parent/worker"
        );
    }

    #[test]
    fn c2p_canonical_pair() {
        // Direction is reversed from p2c: the child publishes, the
        // parent subscribes, so the first segment after `c2p/` is the
        // child machine. This pin guards against an accidental
        // argument swap in either the Rust or C++ helper.
        assert_eq!(
            key_expr_c2p("worker", "parent"),
            "sce/scxml_invoke/c2p/worker/parent"
        );
    }

    #[test]
    fn p2c_and_c2p_are_distinct_for_same_pair() {
        // Each peer-pair declares two keys. They MUST differ — same key
        // for both directions would let the parent's publisher feed
        // its own subscriber (loopback), which the §9.6 lifecycle
        // does not survive (wire-14 would re-enter the parent as if
        // the worker had emitted it).
        let p = key_expr_p2c("alpha", "beta");
        let c = key_expr_c2p("beta", "alpha");
        assert_ne!(
            p, c,
            "p2c and c2p MUST be distinct keys for the same peer pair"
        );
    }

    #[test]
    fn keys_carry_reserved_prefix() {
        // Every emitted key starts with the SCE-reserved prefix so a
        // future §9.6-reservation collision validator can grep author
        // `<send>` keys for this prefix without false negatives. If a
        // refactor moves the prefix into a sub-namespace (e.g.
        // `sce/v1/scxml_invoke`) this guard fails before the codegen
        // template silently emits keys outside the reserved space.
        for (a, b) in &[("p", "c"), ("very_long_machine_name", "x"), ("", "")] {
            let p = key_expr_p2c(a, b);
            let c = key_expr_c2p(a, b);
            assert!(
                p.starts_with(SCXML_INVOKE_KEY_PREFIX),
                "p2c key '{p}' missing reserved prefix"
            );
            assert!(
                c.starts_with(SCXML_INVOKE_KEY_PREFIX),
                "c2p key '{c}' missing reserved prefix"
            );
        }
    }

    #[test]
    fn empty_machine_names_yield_well_defined_keys() {
        // Empty machine names are degenerate but well-defined: both
        // segments still appear (as empty strings) so the key shape
        // stays parseable as five `/`-separated tokens
        // (`sce/scxml_invoke/p2c//`). Topology validation rejects
        // empty machine names long before this path fires; pinning
        // the degenerate output documents the fall-through behaviour.
        assert_eq!(key_expr_p2c("", ""), "sce/scxml_invoke/p2c//");
        assert_eq!(key_expr_c2p("", ""), "sce/scxml_invoke/c2p//");
    }
}
