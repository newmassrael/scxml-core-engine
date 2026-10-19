# ADR 0041 — A deploy declares the queries a node serves, and the matching and the replies are generated from it

- Status: Accepted and implemented for the Rust backend (the owner asked on 2026-10-19 for the
  upstream form to be designed for the long term, without regard to the downstream whose request
  raised the question)
- Date: 2026-10-19
- Scope: `machines.<m>.queryables` in `deploy.yaml` (`SCE_MESH.md` deploy schema), RFC §5.R,
  `sce-build/src/mesh/queryable.rs`, `sce-build/src/forge/queryables.rs`,
  `tools/codegen/templates/forge/rust/queryables.rs.jinja2`, `sce_link_runtime::{keyexpr, json}`
- Related: ADR 0005 (SCE closes gaps by bounding and declaring), the session runtime of RFC §5.Q

## Context

A node that answers zenoh queries (an admin space, a configuration endpoint, a status page) knows
three things before it runs and recomputes them for every query:

- **which keys it answers.** The keys are in the code as strings. A query arrives, the key is split
  on `/`, and each answering leg is intersected with it;
- **the shape of each answer.** `status/connect` is `{"endpoints":[...],"last_write":...,
  "permissions":{"write":...}}` and its fields are fixed. It is assembled into a `String` per query;
- **the size of the largest answer.** It follows from the shape and from the bounds on its arrays
  and strings. A general frame buffer is taken per query instead.

A downstream measured this on an Infineon CYT4BF8CDS (Cortex-M7, 350 MHz) against zenoh-pico on the
same MAC, driver and stack: 175 to 181 us of application time per query against 54, with about a
fifth of it spent in the heap, and 20 allocations per query against 8. That measurement is cited as
one witness. It is not what fixes the shape: the shape follows from what is known when.

What is **not** known when the deploy is written is the node's own key prefix (`@/<zid>/peer`; the
zenoh id comes from the MAC or is drawn at boot) and the values in a reply (the endpoint list, the
session list, a write permission). A design that treated the whole key as static would be wrong.

## Decision

**1. The deploy declares the queryables, and only what is static.** `machines.<m>.queryables` is a
list; an entry has a `name`, a `key` (the part after the node prefix) and a `reply` made of an
`encoding` and exactly one of a `body` (text that never changes) and a `document` (a JSON object
whose fields, types and bounds are declared). The prefix and the values are the host's, at run time.

**2. The Rust backend generates three things from it, in `{machine}_queryables.rs`.**

- `answering(query) -> LegSet`: the set of declared queryables a query key expression reaches,
  without allocating. An exact query is a comparison with the declared literal keys; one with `*`,
  `**` or `$*` is intersected with each declared key by `sce_link_runtime::keyexpr::intersects`.
- `write_<name>(doc, out)`: the reply written into a buffer the caller owns. The document is
  measured first, so a buffer too small is an error and nothing is written, and a value outside a
  declared bound is refused and not written.
- `<NAME>_MAX_BODY_BYTES` and `MAX_BODY_BYTES`: the largest a reply body can be, derived from the
  declared bounds, which the host sizes its response buffers from.

**3. The declared side is constrained so that the matching is bounded.** A declared key is literal
chunks and whole-chunk `*` and `**`, never `$*`, with at most 32 chunks; the matcher keeps the
positions in a declared key as the bits of one `u64` and moves them once per query chunk, so a query
of any length costs time linear in its length and a `$*` chunk costs at most the product of its
length and the declared literal's. The query side is whatever arrived, so it may carry all three
wildcards. `**/**` and `**/*` are refused because the canonical spellings are `**` and `*/**`.

**4. Overlap is allowed and a duplicate is not.** Two queryables may declare `peer/**` and
`peer/x`: a query reaches every queryable it overlaps, and the host answers each. Two queryables
declaring the same key would give one query two answers under one name and is refused.

**5. Refusals are errors at the deploy, with their own codes.** `deploy/queryable-key-invalid`,
`-key-duplicate`, `-name-invalid`, `-reply-invalid` and `-document-invalid`. The last names the path
of the field (`outer.inner`, `list[]`). A deploy that states no queryables emits nothing and every
existing output stays byte-identical, which a test holds.

**6. The writer is JSON, written as the declaration says and no more.** Compact, fields in the
declared order, a string escaped as RFC 8259 requires (so a string of `n` bytes is at most `6n + 2`
bytes), an integer in decimal, an absent `nullable` value as `null`. The size bound is exact for the
worst case: a test writes that worst case and compares its length with the generated constant.

## What this does not decide

- **The other five backends.** The matcher and the writers are Rust the host links; the other
  backends emit nothing for a `queryables:` block, as the session runtime does. A backend that serves
  queries would need its own runtime pair and the same declared constraints.
- **Borrowed views of the request codec, and a machine with no dynamic registration.** The request
  asked whether the generated `Request` codec's borrowed view is enough for dispatch, and whether a
  machine can be said not to register queryables at run time. Neither is answered here; neither
  changes this decision.
- **Kconfig.** The size constants are Rust constants a host can read. Emitting `CONFIG_*` fragments
  from the same declaration is a separate decision.
- **Generating the host's dispatch.** `answering` says which queryables a query reaches and the
  writers write their replies. What the host does between them (the transport's reply call) stays
  the host's.

## Consequences

- A downstream replaces its per-query key splitting and `String` assembly with `answering` and
  `write_<name>`, and derives its response buffer from `MAX_BODY_BYTES`. Whether that saves the time
  it was measured to cost is the downstream's measurement; this decision is the shape, not a
  performance claim.
- The declared constraints are the price of a bounded matcher. A key that needs `$*` on the declared
  side is not expressible, and the refusal says so.
- `sce_link_runtime` gains two `no_std`, allocation-free modules with their own unit tests, and the
  deploy-conformance crate compiles and runs the generated module on a declared admin machine.
