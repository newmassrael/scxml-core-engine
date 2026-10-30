# ADR 0073 — A session id proven while the program is compiled

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left, and a consumer's request (watching-zenoh).
- Amends: ADR 0045 ("Questions the owner answered" 2: the host gives a root's session id, and it is
  held to the Nmtoken rule where the machine is built).
- Evidence: watching-zenoh's fixed client image (thumbv7em, `opt-level = "s"`), where the run-time
  check is `helpers::session_id::checked` 254 bytes and `character_at` 122 bytes; and
  `backends/rust/probes/nostd-footprint`.

## Context

ADR 0045 makes a root's `_sessionid` an XML Nmtoken that the host gives, and the generated
`new_with_session_id` refuses an id that is not one, with the reason. On a target without an
allocator the id is a `&'static str` and the check runs where the machine is built.

A firmware states its ids as literals: watching-zenoh builds its unicast session and each peer
connection under a constant, and wraps the `Result` in a helper that says the refusal can never be
met. The check still runs, and its 376 bytes are in every image, for a judgement the compiler could
have made. `checked` is a constant function already (ADR 0056's `new_const` uses it), but
`new_const` builds the whole machine in a constant, which a machine whose host binding is made at
run time cannot use.

`saved_state`, the other caller, is `std`-only and is not in such an image.

## Decision

### 1. A type for an id that is already an Nmtoken

`helpers::session_id::SessionId` holds a `&'static str` that has been shown to be an Nmtoken. Its
only constructor is `SessionId::new`, a constant function that panics on an id that is not one,
with the reason `checked` gives. The field is private, so a value of the type is a proof.

`sce_rust_runtime::session_id!("...")` evaluates `SessionId::new` in a constant, so a bad literal is
a compile error and the program holds no check of it. A `const` item that calls `SessionId::new`
does the same. Called outside a constant, `SessionId::new` checks at run time and panics; a host
whose id arrives at run time uses the fallible constructor, which tells it why.

### 2. A constructor that takes the proof

Every generated machine gains `new_with_checked_session_id(SessionId, ..)`, and with a host of
native duties `new_simulated_with_checked_session_id`. It returns the machine, not a `Result`,
because there is nothing to refuse. It holds the text on a target without an allocator and a copy
of it with one, as `new_with_session_id` does.

`new_with_session_id` stays: it is the constructor for an id the program does not know when it is
compiled. `new_const` checks through `SessionId::new` instead of a helper of its own.

## Consequences

- A program whose machines are all built from a `SessionId` does not link `checked`.
- The footprint probe builds its machines this way, as the consumer shape it weighs.
- The constructors are additive; no existing caller changes.
