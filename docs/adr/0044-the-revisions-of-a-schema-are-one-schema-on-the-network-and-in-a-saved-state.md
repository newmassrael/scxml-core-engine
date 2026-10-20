# ADR 0044 — The revisions of a schema are one schema, on the network and in a saved state

- Status: Accepted and implemented: the Mesh rule in the build-time check, the saved state on the
  Rust and Kotlin backends (the two that save)
- Date: 2026-10-20
- Scope: `forge::schema_revision` (the rule), `mesh::deploy::validate_event_schemas_cross_machine`
  and `mesh/event-schema-mismatch`, `forge::static_lowering::saved_shape`, the Rust and Kotlin
  templates' save and restore, `sce_rust_runtime::saved_state`, `com.sce.runtime.SavedState`,
  `schemas/sce-saved-state.v1.schema.json`
- Related: `docs/SCE_ACCEPTED_SUBSET.md`, "An event schema that grows" (`sce:revision`, `sce:since`,
  `sce:default`, and the lift that reads a default for a field a delivery lacks), and §2.15 "Saving
  and restoring"; ADR 0005 (`sce-static` closes its gaps by bounding and declaring)

## Context

A schema says which revision it is, and a field a later revision added says which revision added it
and what a delivery that lacks it takes. Every generated lift reads that default, so a delivery
written before the field existed is read rather than refused. That round left two places where a
schema that grew was still refused, and documented both as not covered:

- **The network.** `mesh/event-schema-mismatch` refused a sender and a receiver whose schemas for
  one event differed in any way, a field's `since` and default included. The devices of one
  deployment do not all run one build: during an upgrade some run the old one and some the new, and
  every one of them had to be refused although each lane reads what the other sends (an older
  delivery takes the receiver's defaults; a newer one carries fields an older receiver walks past).
- **A saved state.** A saved state is bound to the document's shape, a digest that names each field
  of each record variable. Adding a field to the schema a `record:` variable is of changed the
  shape, so a state saved by the earlier build was refused as one whose variable was re-typed, and
  the users of an application lost their state the day a field was added.

What is to be decided is the same question in both places: when are two schemas of one event two
revisions of ONE schema, and what may be read across them.

## Decision

**1. The rule, stated once.** A schema's lineage is named by the event it declares. Two schemas of
one event are revisions of one schema when both of these hold, and not otherwise:

- (a) every field both declare is declared alike: the same type, bound, quantity, `since` and
  default (the whole field as the forge model serialises it, the comparison the structural hash
  made before);
- (b) every field only one of them declares was added after the other's revision: its `since` is
  past the other schema's `sce:revision` (a field of the first revision has no `since` and counts
  as revision 1).

So a later revision keeps every field an earlier one declares, alike, and adds fields only with a
`since` past the earlier revision and a default. What (b) refuses is every other difference in the
field set: a field removed or renamed (the old name is in one, at a revision the other has), two
schemas of the same revision that declare different fields, and two builds that disagree about
which revision added a field. What (a) refuses is a field re-typed or re-bounded, and two builds
that give one added field different defaults. The rule is symmetric, so it does not matter which
side is older. It is `forge::schema_revision::divergence`, which answers the first field that
breaks it and how; every place below calls it or derives from it.

**2. The network.** `validate_event_schemas_cross_machine` compares the sender's and receiver's
schemas of each cross-machine `<send>` by the rule instead of by a hash. A refusal stays
`mesh/event-schema-mismatch`, with a reason that says which field, how, and the two revisions
(`NotOneSchema`), in place of two digests no document holds. A schema on one side only is refused
as before.

The run-time half needs no change, and no backend has a run-time schema check to change: a Mesh
envelope carries `_event.data` as JSON with no schema identity, and no SCE engine produces the codec
that would bind the bytes to a schema (`typed`; the Rust router refuses an envelope carrying it).
What the run time must do is what every
lane's lift already does, held by `static_schema_revision` on all seven: read the default of a
field an older sender omitted, and walk past a field a newer sender added. The world simulation
holds it for two builds on one network (`backends/rust/world`, two machines generated from two
revisions of one schema exchanging one event each way).

**3. A saved state names the revision it was saved at.**

- The shape names the fields of each record schema's FIRST revision only. For a schema that never
  grew that is every field, so the shape of every document whose schemas have one revision is the
  shape it had, every state already saved restores, and every such machine saves the same text.
- A machine any of whose record schemas is past its first revision writes `revisions`: for each such
  schema, by its event name, the revision it was saved at and `fields`, a SHA-256 of what that
  revision declares (each field whose `since` is at most the revision, with its type and its enum's
  variant names; not the default and not `since`, which a saved state does not name). It also writes
  the REVISED shape, the digest of the shape's text with one more line. A reader that predates
  revisions knows only the first shape, so it refuses such a state, as it refused any change of shape
  before, instead of reading it with the newer fields dropped.
- A restore takes the first shape with no `revisions`, or the revised shape with `revisions`, and
  nothing else. Each revision a state names must be of a schema a record variable of the machine is
  of, at most the machine's own revision of it, and its `fields` must be the machine's own digest of
  that revision; a state saved at revision r of a schema now at revision R ≥ r is then read with each
  record of it (a record variable, each element of a list of records) taking the default of every
  field added after r. A record that already holds such a field is refused: revision r does not
  declare it. A state saved at a later revision than the machine's is refused (a downgrade would
  drop what it does not know).
- An event in the saved external queue or in a waiting send holds its data as the wire carried it,
  and the lift reads it with the defaults when it is delivered, so those need nothing more.
- On the network a field's default and `since` are compared, and in a saved state they are not.
  On the network a receiver reads a default for every delivery an older sender wrote, and two
  receivers that disagree on it read one delivery two ways. A saved state is read by one build, the
  restoring one, and the default is read only for a field the state was saved without.

**4. Where it is implemented.** The generator computes the shapes and the per-revision digests
(`static_lowering::saved_shape`, which returns a `SavedShape`), the templates hand them to the
runtime as one constant, and each runtime judges and grows a saved state in one function
(`saved_state::upgrade` in Rust, `SavedState.upgrade` in Kotlin) before the generated restore reads
a variable, so the generated readers are unchanged. The Go, Python, C11 and C++ backends and the
Interpreter have no saved state, so nothing changes there.

**5. The saved-state format changes additively.** `revisions` is an optional member of
`sce-saved-state.v1`, absent when no record schema is past its first revision; `format` stays 1.

## Alternatives not taken

- **One shape over every combination of revisions** (a table of digests for each vector of saved
  revisions). It grows as the product of the schemas' revisions, and it answers nothing the
  per-schema digests do not.
- **`revisions` with the shape left as it is.** A reader that predates revisions ignores members it
  does not know, so it would restore a newer state with the newer fields dropped, which no build
  did before. The revised shape is what makes that reader refuse.
- **`format: 2` for a state that names revisions.** The format is the document's grammar; a state
  of a grown schema is the same grammar with one more member. And a pre-release surface moves
  additively where it can (SCE_WIRE_CONTRACTS.md).
- **The lineage alone on the network** (first-revision fields compared, added fields trusted). Two
  builds that disagree about what revision 2 added would exchange events silently and read
  different values; the rule refuses that disagreement where it can be seen, at the build.
- **Removing a field a later revision added.** A delivery would still read on both sides, but
  "revision N" would no longer name one set of fields, which the saved state's digests and the rule
  both rely on. A field that is no longer wanted stays, and its default is what an older sender's
  delivery reads.
- **Restoring a newer state into an older build, dropping what it does not know.** Refused, as every
  shape change was before: the dropped values cannot be recovered by the next upgrade.

## Consequences

- A deployment upgraded device by device builds when one machine is pinned to an older revision of a
  schema; a deployment whose machines disagree about a field is refused with that field's name.
- An application that adds a field to a record schema keeps its users' saved states.
- What is still refused is unchanged in kind: a renamed, re-typed or removed field, a state, history
  or variable changed, a variant of an enum a record field is of renamed or added (the shape names
  the variants, as before).
- `mesh/event-schema-mismatch`'s `SenderOnly` and `ReceiverOnly` reasons still put a tag in `actual`
  that no document holds (SCE_ERROR_CONTRACT.md §2.1); the new reason carries none. That older gap is
  not changed here.
