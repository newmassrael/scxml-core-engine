# SCE Axis 6 Patterns — Third-Party Library Surface Absorber

**Scope**: this document defines the axis-6 ownership-inversion mechanism,
catalogs the absorbers SCE ships, and declares the reactivation protocol for
future audit findings.

It complements `memory/next_ownership_inversion_5axis_program.md` (the audit
trail) and `claudedocs/rfc-axis6-third-party-surface-absorber.md` (the design
RFC). When a new third-party-library surface assertion is discovered, this
catalog is updated to seat the new instance.

## Mechanism

Axis 6 covers ownership inversions where the downstream component asserts an
invariant about an **upstream third-party library's** surface (error-message
format, callback lifecycle, ABI lifetime, version-conditional behaviour) that
SCE cannot rewrite in the library's terms. Axes 1–5 do not apply: SCE owns
neither the library's API nor its declarations, so neither parameter-binding
(axis 1) nor declared-consumption (axis 2) nor reverse-linkage (axis 5) closes
the loop.

The axis-6 mechanism has two paired techniques. Each absorber uses one or
both:

### Technique A — Dynamic verification gate

A ctest fixture exercises the failure mode end-to-end with the **same
third-party binary the production build links** and asserts the SCE-side
classification still fires correctly. An upstream rephrasing of an error
string, or a behavioural shift in a callback contract, surfaces at CI time
rather than at customer time. The fixture's success defines the contract; the
fixture's failure on a library upgrade names the contract drift.

### Technique B — Defensive idempotent absorption

Where the third-party library's contract is "I will tell you when X changes
via callback", SCE wraps the call with an immediate post-registration probe
of the current state. The probe is **idempotent with the callback** — both
paths converge on the same handler invocation. The SCE-side caller sees
identical behaviour regardless of whether the library fires the initial-edge
callback, debounces it, or omits it entirely.

## Catalog (seeded 2026-05-20)

### A6-001 — `AuthClassifier` (zenoh-cpp `ZException::what()` keyword scan)

- **Header**: `sce/include/mesh/third_party/AuthClassifier.h`
- **Asserted upstream surface**: zenoh-cpp's `ZException::what()` message
  contains one of the ASCII tokens `certificate`, `tls`, `auth`, or
  `handshake` (case-insensitive) on any auth-class failure.
- **Drift consequence**: a zenoh-cpp upgrade rephrasing those tokens silently
  flips every UNAUTHORIZED to TRANSPORT_UNAVAILABLE. SCXML author's
  `<transition cond="reason == 'UNAUTHORIZED'">` never fires.
- **Absorber technique**: A (dynamic verification gate).
- **Absorber fixture**: `tests/mesh/AuthClassifierCIFixture.cpp` generates
  fresh CA-mismatched cert material at ctest run time (openssl shell-out
  inside the fixture's test-suite setup), spins two in-process zenoh
  Sessions configured for mTLS, captures the client-side
  `ZException::what()`, and asserts both (a) the throw arrived (Stage 1)
  and (b) the current limitation status (Stage 2 — see Limitation below).
- **Keyword manifest**: `kZenohAuthFailKeywords` `constexpr` array in
  `AuthClassifier.h` — single source consumed by both the runtime classifier
  and the fixture. Conservative widening (per row-10 RFC Q3 lock-in) is
  structurally enforced by the `EveryManifestKeywordFires` unit test:
  adding a keyword requires adding a fixture line; removing a keyword
  fails the unit test.
- **Origin**: §16.7 row 10 UNAUTHORIZED closure (`73087043`); axis-6
  formalisation (this RFC).

#### Limitation — production-deferred (2026-05-20)

The axis-6 fixture surfaced that current zenoh-cpp versions wrap every
connection error (including mTLS handshake failure) in a generic
`Z_ENETWORK = -4` `ZException` whose `what()` payload is `"Failed to open
session(Error code: -4 )"`. None of the manifest keywords match that
string, so the runtime `isZenohAuthFailMessage` returns false and SCE's
§16.7 row-10 production codepath does not fire — the spec contract is
shipped (closure `73087043`) but live production emission of
`error.communication{reason: UNAUTHORIZED}` is deferred until zenoh-cpp
upstream exposes a typed auth-failure discriminator (e.g.
`ZAuthException`) that SCE can catch directly.

The runtime classifier is retained as **future-proofing**: the moment a
zenoh-cpp release ships with auth-fail messages containing a manifest
keyword in `what()`, the production path activates without any SCE-side
edit. The CI fixture's `RowTenLimitationIsLockedIn` assertion catches that
transition — when it ever fails, the remediation steps are inline at the
failing assertion (flip to `EXPECT_TRUE`, remove the limitation notes
here and at `CommunicationError.h::transport_status`, add a release-note
entry naming the activating zenoh-cpp version).

The SOMEIP arm of row 10 (binding-declared SD-denial classification via
`sd_denied_classifies_as_unauthorized: true`) is unaffected by this
limitation — no text inspection is involved.

## Reactivation protocol

When a future audit, consumer-project signal, or runtime incident surfaces a
new third-party-library surface assertion:

1. **Classify**. Confirm the inversion is axis-6 (not axes 1–5). Axis-6
   requires: (a) the asserted surface lives in upstream code SCE does not
   own; (b) the surface is not exposed as a typed discriminator by the
   upstream (otherwise the resolution is "use the typed API", not axis-6
   absorber); (c) drift would silently break an author-facing SCE contract.
2. **Choose technique**. Dynamic verification gate (A), defensive idempotent
   absorption (B), or both paired. Mock-based absorbers (no real third-party
   binary in fixture) violate axis-6 by definition — they prove nothing
   about real-binary drift.
3. **Land as atomic**. Add the absorber header under
   `sce/include/mesh/third_party/` (or analogous backend-specific subdirectory
   if the instance is not mesh-scoped). Add the ctest fixture under
   `tests/mesh/` (or analogous). Update this catalog with a new entry.
4. **Diagnostic codes**. Axis-6 work strengthens existing author-facing
   contracts; it does not introduce new ones. New codes are reserved for
   internal misconfiguration scenarios that the author cannot guard on.
5. **No carve-outs**. Per `feedback_no_carveouts`, each new instance lands as
   a full textbook atomic — full absorber + full fixture + full catalog
   entry. Incremental landing across multiple atomics is permitted only for
   genuine atomic-size constraints (e.g. cross-platform fixture work
   requiring per-OS gates).

## Out of scope

- Replacement of the substring-scan classifier with a typed zenoh-cpp
  discriminator: gated on upstream capability. When zenoh-cpp 1.x exposes
  `ZAuthException` (or equivalent), a follow-up atomic replaces the keyword
  manifest with the typed catch. The axis-6 mechanism is retained as the
  upgrade gate.
- Generalized `ThirdPartyAbsorber<L>` framework: rejected at RFC Q-1 as
  premature abstraction. Each absorber stays instance-specific until the
  third axis-6 instance shows a clear unification pattern.
- AOT-side mocking of third-party APIs for `--no-network` test variants:
  separate concern; mock fixtures do not satisfy axis-6 (see protocol step
  2).
