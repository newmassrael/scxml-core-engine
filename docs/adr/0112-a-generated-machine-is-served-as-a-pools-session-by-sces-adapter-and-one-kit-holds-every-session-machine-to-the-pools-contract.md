# ADR 0112 — A generated machine is served as a pool's session by SCE's adapter, and one kit holds every session machine to the pool's contract

- Status: Accepted (2026-11-22, by the owner, relayed by a session of the downstream consumer: "the adapter is a
  library facility, not test code ... the structure that is right in the long run, cost aside"). Written under the
  owner's standing rule (the most long-term correct structure, textbook in its foundations, cost ignored; decided, not
  asked), while building ADR 0109's in-place upgrade under load ("Conformance"; D11).
- Landed: D1 to D5, in the landing that adds this record: `sce_rust_runtime::saved_state::PersistentPolicy` and its
  generated implementation (D1), `sce_rust_mesh::served` (D2, D3), `sce_rust_mesh::session_conformance` (D4), and
  the shape and record lineage of every record at every depth with the Rust and C++ restores that grow it (D5); the
  server suite's upgrade under load (ADR 0109, "Conformance") runs on them.
- Date: 2026-11-22
- Amends (each amended document gains an "Amended by: ADR 0112" line in the landing that moves it):
  - ADR 0107, "a server program is each deployment's": it stays each deployment's, as the composition root
    (configuration, TLS, store, authenticator, clock loop); the adapter that serves a generated machine as a session is
    a part SCE publishes and the program composes (D1, D2);
  - ADR 0109, "Conformance": the server suite's upgrade under load runs the adapter, and the kit of D4 holds every
    session machine the suite serves;
  - `docs/SCE_SERVER_DEPLOY.md` section 1, the row of how a session's machine is started and restored, and section
    0's list of what is not built;
  - ADR 0044 and `docs/SCE_ACCEPTED_SUBSET.md` §2.15, "A record schema that grew": a record is named by its schema
    and its first revision at every depth a state holds it, and grows there (D5);
  - `docs/SCE_CODEC_CONSUMER_MIGRATION.md`, a section for what a consumer changes.
- Scope: the Rust host core (`sce-rust-mesh`), the Rust runtime (`sce-rust-runtime`) and the Rust template of the
  generator for D1 to D4; C++ serves no keyed pool (ADR 0109, D0), so it has no adapter to take. D5 is the build's
  shape and record lineage (`forge::static_lowering::saved_shape`, `forge::import_source`), read by the Rust and
  C++ restores (`saved_state::upgrade`, `upgradeSavedState`) and refused by name on every other lane.
- Related: ADR 0045 (a saved state is a tree of sessions), ADR 0044 (a record schema grows by revisions), ADR 0107 D3
  and D10 (the keyed pool; the server suite), ADR 0109 D10 and D11 (the outbox; the declared migration).

## Context

A keyed pool (`sce_rust_mesh::pool::Pool`) runs each session through a `SessionMachine` a `SessionFactory` builds.
Its documentation said a host implements both for its generated machine "in a few lines": hand the endpoint's calls to
the engine, tick it, save it with the generated `save_at`. No code anywhere did. The server suite's sessions ran a
machine written by hand (`tests/server/src/ledger.rs`), whose save and restore never ran a generated restore, so an
upgrade under load over it would have held nothing of ADR 0109 D11: the migration runs only in the generated restore.

The few lines are not a host's to write. What makes a session's checkpoint (SCE_MESH.md section 10.11) and its
machine's restore (D11) meet is an order nothing in the types enforces: the engine's clock owned by the host and moved
to the pool's wall clock, the endpoint registered before the machine starts so that what its initial configuration
sends reaches it, the host's operations registered before the first macrostep so that an invocation a saved state held
running starts again, the migration applied by the restore before anything is registered. A consumer that writes them
writes SCE's knowledge into its program, and every consumer writes it again. And a generic adapter could not be
written: the generated save and restore were methods of a trait of each machine (`<Machine>Persist` on `Engine<P>`),
which code generic over the document cannot name.

## Decisions

### D1. The runtime names a machine's save and restore once, and the generator implements it for every policy

`sce_rust_runtime::saved_state::PersistentPolicy`, a supertrait of `StatePolicy`: `SHAPE`, `save_at(engine, wall)`,
`judge(saved)` and `restore_with(policy, saved, clock, wall)`, the same four the machine's `<Machine>Persist` has.
The Rust template implements it for each generated policy whose machine can be saved, by those functions
(`tools/codegen/templates/rust/state_machine.rs.jinja2`). It is on the policy, a type of the generated crate, so the
orphan rule admits it where an implementation on `Engine<P>` would not be.

### D2. `sce_rust_mesh::served`: the factory and the machine of a generated document, in one order

`Served<P, H>` is the `SessionFactory` and `ServedMachine<P>` the `SessionMachine` over any `P: PersistentPolicy`:

1. The engine measures time on `SceClock::Manual`, read as milliseconds since the Unix epoch and moved to the pool's
   wall clock at every run, so a delayed send is saved as the wall-clock moment it comes due and restored against the
   same clock, whatever clock the pool is given (a scripted one in a test).
2. A start builds the engine, sets the clock, registers the session's endpoint (`endpoint::register`) and then the
   host's operations, and only then initializes.
3. A restore is `PersistentPolicy::restore_with`, which applies the document's migration (D11) and refuses a state no
   step carries with its reason, which the pool reports as `PoolError::NotBuilt` and answers `InternalError`; then the
   endpoint and the host's operations are registered, before the first macrostep.
4. A run applies the endpoint's calls (`endpoint::apply_to`), advances the clock to the wall clock and ticks while
   something is due at that instant, at most `TICKS_PER_RUN` ticks; a document that keeps arming sends with no delay is
   left to its next run, which the pool makes at the earliest due its checkpoint names.
5. A save is the generated `save_at` at the engine's own instant, which its last run moved to the wall clock, so a
   pending send is saved as the moment it comes due; a session is finished when its engine reached a top-level
   final. Amended on landing: a save at the wall clock the pool hands `save` wrote every pending send as due later
   by the time since the session last ran, and moved it again at each stop; the kit found it (D4).

### D3. What a deployment varies enters as a `SessionHost`

`SessionHost<P>`: `policy(key)`, the policy of each session (`<Machine>Policy::new()`, or one given the values its
deployment supplies, ADR 0109 D12), and `register(key, engine)`, the host operations the deployment serves it
(`sce_rust_host_features::serve`, or invokers of its own). Both are called for a start and a restore alike. Nothing
in the adapter varies by document.

### D4. One kit holds every session machine to the pool's contract

`sce_rust_mesh::session_conformance`, under the crate's `conformance` feature: functions over a `Subject` (the
deployment's `Machine`, its factory, the requests that move a session and end it, what it sends as it starts, a state
of another document, a state an earlier version saved), each run through a `Pool` on a `MemoryStore` as a server
runs one, so the contract is held as the pool reads it:

- a session saved at an orderly stop between two requests, and restored by the next process, answers the rest as a
  session that never left memory does, and is saved as that one is (its session id aside);
- what the machine sends as its initial configuration is entered is sent once, at the start, and not at a restore;
- a session is kept while it runs and removed once it reaches a top-level final;
- a request for a key whose checkpoint is of another document is answered `InternalError` and the checkpoint is left;
- a state an earlier version saved is restored through the declared migration and saved under this version's shape.

The ledger and both versions of the suite's upgraded document are held to it (`tests/server/tests/the_session_contract_holds.rs`).
Found by it on landing: the ledger restored a state of any shape whose variables it could read, and now refuses
another shape; the adapter saved at the pool's wall clock rather than the engine's instant (D2, step 5). The kit runs
in the server suite and not in `sce-rust-mesh`'s own tests, which build no generated machine.

### D5. A record is named by its schema and its first revision, and grows at every depth a state holds it

Found by the upgrade under load before any of it landed, and reported to the consumer first; the owner had it
widened to one rule for every record a state holds. The shape writer named a record of entries by the fields of its
FIRST revision, its later revisions the saved state's `revisions` (ADR 0044), but a record that held a list or a
record at any revision, and a union's case, by a line naming the record's document and every field of every
revision, with no lineage. A schema whose next revision added a list (as `schema_cose_note_revised` does) moved the
shape of every variable that held it: a state saved before the revision was refused by shape, and a declared
migration was refused as a change of type, though D11 counts a record schema grown by revision as a widening that
needs no element. A record below a variable's value (a field of a record, a list a record holds, a union's case) was
never grown at all.

- **One spelling.** A record is named by the event its schema declares, its identity whichever file it is read from,
  and by its first revision's fields: field by field where that revision holds one entry per field, by one `record`
  line otherwise; a union's `case` line names its record the same way, and a `union` line names the variable and
  its tag, not the union's document. A file renamed, or a revision that adds a field of any type, moves no shape.
- **One lineage.** Every record a state holds, at any depth, has its lineage: the digest of each revision past the
  first (a field of one entry by its present type and its enum's first variants, exactly as before, so no digest a
  state already names moves; a composition by its type), each added field's default (a list the empty list, an
  optional value absent; a record a later revision adds is optional and reads absent, as the language already
  decides), and each field a revision made optional. Its holders name the steps from a variable's value to the
  record (`RecordHolder::path`: a field, the elements of a list or set, a part of a map's entry, a union's case),
  and the Rust and C++ restores grow every record they reach. The data is read where a record's scope is built
  (`ScopedRecord::lineage`).
- **Other lanes.** Kotlin, Go, Python and Swift grow what a variable or its elements hold, as before; a machine that
  holds a record whose schema grew below that is refused for them by name (`StaticTarget::walks_held_paths`,
  docs/adr/0109, D0), so no restore reads it without the fields it lacks.
- **States saved before.** The spelling of a composed record and of a union moved, so the build writes the shape
  an earlier build named the document by too (`SavedShape::legacy`), and the Rust and C++ restores take a state of
  that shape as one of this document, each record the earlier build did not name in `revisions` at this build's
  revision, the revision that build saved it at. A state the earlier build could not restore across a revision
  stays refused. The other lanes refuse such a state by its shape. A shape file of the old spelling is not a step's
  `from`: a release built before this landing is first rebuilt as it is, and its next version's step starts from
  the file this build writes (`docs/SCE_CODEC_CONSUMER_MIGRATION.md`).
- Amends ADR 0044: its promise, a record saved at an earlier revision read at the later revision's defaults, now
  holds for every record a state holds, composed or not, at every depth, on the Rust and C++ restores.

## Tested by

- `sce-build/tests/a_declared_migration_is_planned_from_its_shape_files.rs`,
  `a_record_grown_at_any_depth_is_carried_with_no_element` (D5): one shape for both versions of
  `static_grow`, the step planned for Rust and C++ and the document refused by name for Kotlin and Go.
- `backends/rust/tests/tests/a_record_is_grown_at_any_depth_it_is_held.rs` and
  `tests/integration/ARecordIsGrownAtAnyDepthItIsHeldTest.cpp` (D5): a state of `static_grow_1` restored into
  `static_grow_2`, a number with a default, a list, an optional record and an optional text added, read at a record
  variable, a record inside it, a list a record holds, a list variable, an optional variable and a union's case.
- `tests/server/tests/the_session_contract_holds.rs` (D2 to D4).
- `tests/server/tests/a_server_upgraded_in_place_under_load_loses_nothing.rs` (ADR 0109, D11 and "Conformance"), over
  the adapter.

## Alternatives rejected

- **The adapter in the server suite.** It would hold the suite and not the code a server program runs; every program
  would write it again, and the order of D2 would be each program's to get right.
- **The adapter generated beside each machine.** Nothing in it varies by document, and a generated copy per machine is
  the duplication the generator exists to avoid.
- **A trait on `Engine<P>` in the generated crate.** The orphan rule refuses it; the policy is the generated type.
- **A record line keyed by the schema's document name across revisions.** A shape that moves when a schema file is
  renamed, or when a revision adds a composition, refuses states ADR 0044 promises to read.
- **A default instance for a record a later revision adds.** No literal spells a record; the language declares such
  a field optional, and absence does not claim a value the sender never sent.
- **Refusing every state an earlier build saved under the old spelling.** The values are the same and only the
  spelling moved; the earlier build's digest names the same document, so it is read as one.
