# ADR 0108 — A statechart holds a table by key as a bounded map value, and a full table answers `full`

- Status: Proposed (2026-11-19). Written under the owner's standing rule (the most long-term correct structure,
  textbook in its foundations, cost ignored; decided, not asked) at the request of a downstream consumer that is
  moving a protocol stack's hand-written logic into SCE documents. The consumer reviews it against its own tables
  before the first landing, and the record is accepted or amended from that review. Nothing of it has landed;
  "Order of work" lists the steps.
- Date: 2026-11-19
- Amends, once accepted (each amended document gains an "Amended by: ADR 0108" line in the landing that moves it):
  - `sce-build/src/forge/value_type.rs`'s grammar (docs/adr/0086, section 1): two constructors, `map<K, V>[B]` and
    `set<K>[B]` (D1);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15: a paragraph for each of the map variable, its reads, its statements, its
    loop, its saved form and its algorithm slot, and the paragraph that refuses an algorithm's loop over a
    bounded-collection import, whose "reopened by the change that lets a statechart hold a bounded-collection" no
    longer names the change that reopens it (D8);
  - `docs/SCE_ACCEPTED_SUBSET.md`'s appendix: the diagnostic codes D10 adds.
- Scope: the type grammar and its judgment, the `sce-static` statechart parser and checker, the standard library
  (`stdlib/collection`), the Interpreter's lowering, the Rust and C11 runtimes and machine lowerings first, and the
  other five lanes after them (D9); the algorithm kind's slots, locals, returns and `<sce:foreach>`.

## Context

A downstream consumer that implements the zenoh protocol, moving its logic into SCE documents, found the largest
part of that logic blocked on one missing construct: a table the machine owns, read and written by a key known only
at run time. The tables are a router's face and resource tables, the declaration and interest stores, the tables of
pending queries and of matches, the peer table and the reassembly slots. Today they are hash maps and bounded vectors
written by hand in Rust.

What SCE has today does not hold them:

- A `sce-static` statechart holds `list<T>[B]` with `T` a number, a bool, an enum or a record. A list is written by
  `<sce:append>` and `<sce:clear>` and read by index, length and `<foreach>`. Nothing finds an element by a key,
  replaces it or removes it, and an append to a full list raises `error.execution`, which ends the block and leaves
  no value the document can test or count.
- The `bounded-collection` kind is a slot table with `<sce:index-by>`, `reject` and a deploy-sourced capacity. It is
  addressed by handles (slot and generation packed in 32 bits), its element is a codec or procedure document, and
  only a host or a forge kind holds one. `docs/SCE_ACCEPTED_SUBSET.md` §2.15 refuses an algorithm's loop over a
  bounded-collection import in the Interpreter's lowering because no statechart can hold one, and says the refusal
  is reopened by the change that lets a statechart hold one.
- `[name=N]` with `sce-codegen --limit name=N` already gives a bound that a build chooses, written once and read as
  the buffer, the host's constant and `_x.limits.<name>` (docs/adr/0050, docs/adr/0086).

The consumer answered two questions on 2026-11-19 that shape the decision:

1. **Capacity is chosen in two ways.** The microcontroller and session-core tables (the subscription table, the
   reassembly slots) take theirs from build constants of the deployment. The access-point tables (the face table,
   the pending queries of each face, the multicast member table) take theirs from zenoh's configuration read at
   start (`transport/unicast/max_sessions`, `transport/multicast/max_sessions`), and a face is shed when that limit
   is reached. Those tables are on the heap today, with the configured value as their limit, and hold thousands of
   faces.
2. **Both key shapes occur.** The key is a field of the element in the subscription table (a subscriber's id), the
   reassembly slots (a chain's key) and the routing graph (a node's zid, with a second index by zid). It stands
   apart from the element in the alias tables (an expression id to a key expression, and a set of ids), the
   observer table, the table from a query id to its fan-out, and the declarations sent to each face. The
   reassembly key is composite (part of a zid and two enums), and one table is a set with no values.

## Decision

### D1. Two constructors of the one type grammar: `map<K, V>[B]` and `set<K>[B]`

```text
T ::= … | map<K, T>[B] | set<K>[B]
K ::= uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64 | bool
    | enum:<alias> | string[B] | bytes[B]
    | record:<alias>            -- every field of the record is itself a K
```

`map<K, V>[B]` holds at most `B` entries, each a key of type `K` and a value of type `V`, with no two keys equal.
`set<K>[B]` holds at most `B` keys, and is the map whose value carries nothing (a set of ids). `B` is written as every bound
is, a number or `name=N` (docs/adr/0086, section 1), and takes no lower count (`validation/lower-count-misplaced`, as
for a variable today).

A key is a type whose equality is exact and whose every value is bounded: the fixed-width integers, `bool`, an enum,
a bounded text or byte string compared byte by byte, and a record whose every field is a key, compared field by
field. That record is the composite key the reassembly slots need, declared as an event-schema record as every
statechart record is. A zid is `bytes[16]`. A float is not a key, because `NaN` is not equal to itself and `-0.0`
equals `0.0`. An optional, a union, a list, a map and a set are not keys. A key of any other type is refused at the
type as `validation/map-key-type`, naming the type and the field of a record key that is not a key.

A string key is in the grammar from the start because its equality is the byte string's. What goes beyond
equality, such as matching a key expression against a pattern, is not a map's and is decided apart.

### D2. A map is a value, and its order is the order its keys were put

A map is a value of the datamodel as a list is. It is copied whole where a list is copied whole, handed to an
algorithm by name and taken back whole by an `<assign>` (D5), saved and restored, published to the host through
the snapshot, and handed to an invoked child as a top-level `<data>` (docs/adr/0098). No handle, slot or position
ever leaves it. Two maps are not compared, as two lists are not (`expression/unsupported-construct`).

Its entries are ordered by when their key was first put. Putting a key that is present replaces its value in place.
Removing a key and putting it again puts it last. Iteration, the saved form and the snapshot all use this order, so
every engine walks a map in the same order and a scenario compares one trace across all of them. Sorting by key
was rejected (Alternatives rejected, 3).

### D3. What a statechart reads and writes

Reads, in any expression:

- `len(m)` is the number of entries, a `uint32` as a list's length is.
- `has(m, k)` is `true` when `k` is a key of `m`.
- `m[k]` is the value at `k`, and `m[k].field` is a field of a record value, read as a list element is read by its
  index (`docs/SCE_ACCEPTED_SUBSET.md` §2.15, "Reading an element" and "An element taken whole"). A key that is
  absent fails the read as an index outside a list does: `error.execution` is raised and the block ends
  (§scxml-4.9). A guard asks `has(m, k)` first. The key is read once, so it may not name the record being written
  (as `last = days[last.n]` may not).

Statements, each naming the map by `target` as `<sce:append>` names a list:

```xml
<sce:put    target="faces" key="_event.data.zid" expr="face" outcome="putResult"/>
<sce:remove target="faces" key="_event.data.zid" outcome="removeResult"/>
<sce:clear  target="faces"/>
<sce:put    target="aliases" key="_event.data.id"/>          <!-- a set: no expr -->
```

`<sce:put>` sets the value at `key` to `expr`, putting the key if it is absent. Its `expr` is a scalar expression
for a scalar value, and the name of a record variable or loop item for a record value, as `<sce:append>` takes a
record. On a set it takes no `expr`, and on a map it requires one.

### D4. A full map answers `full` as a value; a limit set at run time is a value of the machine

**The outcome.** `outcome`, when written, names a variable of the standard enum `sce:std/collection/put_outcome.scxml`
(`inserted`, `replaced`, `full`) for a put, or `sce:std/collection/remove_outcome.scxml` (`removed`, `absent`) for a
remove. The statement writes the outcome there, and a `full` put leaves the map as it was and raises nothing, so the
document branches on it and counts it in a variable of its own, which the snapshot and the saved state carry as
they carry any variable. A put with no `outcome` that finds the map full raises `error.execution` and ends the block,
as an append to a full list does. A remove of an absent key with no `outcome` does nothing. The outcome enums are
standard documents, so they are imported, generated, saved and shown as every enum is, and nothing new is built for
them.

**A limit set at run time.** The access-point tables are limited by a value the process reads
at start. That value is not the map's bound. The bound is part of the map's type (docs/adr/0086): it is how much the
map can ever hold on every engine, and one saved state means one thing on every build of the machine. The limit
read at start is a decision the machine makes on each put. So the document holds it as data:

```xml
<data id="maxSessions" sce:type="uint32"/>                      <!-- delivered at start: an event or an invoke param -->
<data id="faces" sce:type="map&lt;bytes[16], record:Face&gt;[ap_max_faces=4096]"/>
…
<transition event="face.open" cond="len(faces) &lt; maxSessions">
  <sce:put target="faces" key="_event.data.zid" expr="incoming" outcome="putResult"/>
</transition>
<transition event="face.open" target="shedding"/>               <!-- the limit was reached: shed -->
```

The host delivers the configured value as the payload of an event, or as an invoke `<param>` (docs/adr/0098). The
document refuses a value above the bound at the point it takes it in (`maxSessions <= _x.limits.ap_max_faces`), and
reaching the limit is a transition the document writes, so a statechart and its trace both show it. The outcome `full`
then means only that storage is exhausted, which a document that checks its limit against the bound never meets.
This is the form the consumer suggested ("build bound N, run-time limit L ≤ N"), with L held in the machine rather than in the
type. Alternatives rejected, 1 says why it is not held in the type.

**Storage.** A lane with a heap (Rust with `std`, C++, Kotlin, Go, Python, Swift, the Interpreter) grows a map's
storage as it is filled, up to its bound, and does not reserve the bound at construction. So an access point built
with a large bound pays for the entries it holds. A lane without a heap (Rust without `std`, C11) reserves the bound
in the machine, as it does for a list (docs/adr/0082 for C11's storage).

### D5. A statechart walks a map with `<foreach>`, and an algorithm takes one by name

`<foreach array="m" item="v" sce:key="k" index="i">` walks a map in its order: `item` is the value, `sce:key` the
key, and `index`, which may be left out, the position from 0 (§scxml-4.6). On a set, `item` is the key and
`sce:key` is refused. Each name follows the rules a list's loop has, and the walk is of the map as it was when the
loop began, as a list's is.

An algorithm takes a `map<K, V>` or `set<K>` slot, filled by the name of a variable that holds one, under the
section "State an algorithm takes and gives back". It walks the slot with `<sce:foreach in="slot" item="v" key="k">`,
reads it with `len`, `has` and `m[k]`, and may declare a map local and a map return. It writes them with the same
`<sce:put>`, `<sce:remove>` and `<sce:clear>`, with the same outcomes. A map return declares `may-fail`, and the
variable it is assigned to must hold its bound, as a list return must. It is also the
loop the Interpreter's algorithm lowering refuses today for a bounded-collection import (D8), now over a slot a
statechart fills.

### D6. Where a map may stand

A map and a set may be a `sce-static` variable, an algorithm's parameter, local or return, and the value of a map
or the element of a list. An event schema's field, and therefore a codec's, a lift's answer and a payload, is
refused for now as `validation/map-in-wire-position`, naming this record. The wire form of a map (a CBOR map in
the order D2 gives, a JSON array of pairs because a JSON object's key is only a text) is its own decision (Open
questions, 1).

### D7. Each lane holds a map in one runtime type with a hash index; the hash is never observable

Each lane holds a map in one runtime type: Rust `sce_rust_runtime::BoundedMap<K, V, N>` and `BoundedSet<K, N>`,
and C11 a pair of macros in the runtime header that declare a map type for a key, a value and a bound, used by
the generated machine. Each type is a dense array of entries in D2's order with an open-addressing index over a
hash of the key's canonical bytes, sized to the next power of two at or above twice the bound. A lookup takes
expected constant time, a remove shifts the entries after it to keep the order, and the hash appears in no output,
order or saved form. A table of thousands of faces cannot be scanned linearly on every packet, which rules out the
`bounded-collection` kind's `find_by_index`.

**The saved form** (`docs/SCE_ACCEPTED_SUBSET.md` §2.15, "Saving and restoring") is an array of `[key, value]`
pairs in D2's order, and a set's is an array of keys. Each key and value is written in the form its type already
has there. A restored map holds its entries in the saved order. A saved map with more entries than its bound, or
with a key twice, is refused as a saved list with too many elements is.

**The snapshot** gives the host a read-only view in D2's order: Rust an iterator of `(&K, &V)` with `get(&K)`, C11
a pointer to the entry array and its length.

### D8. The bounded-collection kind is unchanged, and §2.15's refusal names a different reopening

`bounded-collection` stays what it is: a slot table a host or a forge kind holds by handle. A statechart does not
hold one, because a handle is an identity, not a value (Alternatives rejected, 2). §2.15's paragraph that refuses an
algorithm's `<sce:foreach in="<bc-alias>">` in the Interpreter's lowering keeps the refusal and its reason, that
nothing can reach such an algorithm under the Interpreter, and changes its last sentence to say that the table a
statechart holds is a map (this record) and that an algorithm walks it through a map slot (D5).

### D9. Lanes

The Interpreter (whose lowered ECMAScript holds a map as a JavaScript `Map` of canonical key text, which keeps
insertion order as D2 does), Rust and C11 land first, in that order, because the consumer's targets are Rust and C11 and the
Interpreter is the reference every scenario is compared with. C++, Kotlin, Go, Python and Swift follow. Until a lane
lands, a document that declares a map refuses to generate for that lane as `generate/unsupported-feature`, naming
the type and this record, as ADR 0092's union did on the Swift lane (ADR 0099).

### D10. Diagnostic codes

- `validation/map-key-type`: a key type D1 does not admit.
- `validation/map-in-wire-position`: a map or set in an event schema's field (D6).
- `scxml/static-datamodel-rule` (existing): a `target` that names no map, an `expr` on a set's put or none on a
  map's, an `outcome` whose variable is not of the standard outcome enum, `sce:key` on a set's loop.
- `expression/unsupported-construct` (existing): a map as an operand, two maps compared, a whole-map assignment
  from anything other than a map variable of a type it fits or an algorithm's map return.

Each new code lands in the acceptance-doc appendix with its first emitter (`acceptance_doc_covers_every_code`).

### Not in scope

- A second index kept by the map itself (a routing graph that finds a node by its zid beside its id). The document keeps a second
  `map<bytes[16], uint32>` and writes both maps in the same transition. A declared secondary index is a later
  decision once the rows show how many need one (Open questions, 2).
- Ordered range queries (the first key at or above a value).
- Matching a string key beyond equality.

## Consequences

- The consumer's keyed tables have a construct to move to. The session-core and MCU rows take a bound from `--limit`, and the
  access-point rows take a large bound and a limit the document holds.
- A map costs a hash index beside its entries, about two index slots per bound, on every lane.
- Every lane gains a runtime type, a lowering, a saved form and a snapshot view. That is five lanes after the first
  three, each with the scenarios the first three established.
- An algorithm can now take and give back keyed state, so a rule over a table is written once and runs on every
  lane, as a rule over a list is today.

## Alternatives rejected

1. **A capacity chosen at run time in the type** (`map<K, V>[runtime]`, or a limit attribute on the map). The bound
   is part of the type so that one value means one thing on every build and engine (docs/adr/0086). A run-time
   capacity would make two copies of one machine disagree on what a put does with nothing in the saved state to say
   so, and a no-heap lane would still have to reserve a build-time maximum. A limit attribute on the map would also
   hide the decision from the document: "the configured limit was reached" (shed a session) and "storage is
   exhausted" would both be `full`, and the trace would not show which.
2. **Let a statechart hold the `bounded-collection` kind.** That kind is addressed by handles. A handle is an
   identity, not a value: it means nothing after save and restore or in a child session, it cannot be compared, and
   it would bring a handle type into every expression lowering. Its element is a codec or procedure document, not
   the event-schema record a statechart holds. Its lookup is a linear scan. Keyed operations on top of it would hide
   the handle and keep none of what it is for.
3. **Order by key.** A sorted map needs an order on every key type (an enum's declared order, a record's field
   order), costs a logarithmic insert, and gives the rows nothing: none of them reads a table in key order, and
   several read it in arrival order (the declarations sent to a face).
4. **A list of records with algorithms for lookup** (`Upsert(entries, k, v)` assigned back). It works today, but
   every put copies the whole list through the algorithm's slot and return, every lookup is a linear scan, and a
   full list is still the algorithm's failure rather than a value.
5. **Only `map<K, record:S>` keyed by a field of the record.** Half the rows keep the key apart from a scalar value
   (the alias, observer and declaration tables), and a field key can be written as `key="rec.id"`. A field-keyed form can be added as
   a check later without changing the type.

## Order of work

1. This record, reviewed against the consumer's tables, accepted.
2. The grammar: `map` and `set` in `value_type.rs`, the key judgment, the positions of D6. Tests:
   `a_map_key_is_a_type_with_exact_equality`, `a_map_is_refused_in_a_wire_position`.
3. The `sce-static` checker: the statements, reads and loop of D3–D5, the standard outcome enums in
   `stdlib/collection`, the codes of D10 in the appendix.
4. The Interpreter's lowering, with `scenarios/static_map.json`, `static_map_full.json` (outcome and
   `error.execution`), `static_map_record.json`, `static_map_composite_key.json`, `static_set.json` and
   `static_map_order.json` (replace in place, remove and put again goes last).
5. Rust: `BoundedMap` and `BoundedSet` in the runtime, the machine lowering, the saved form and the snapshot. The
   scenarios of step 4 run on the Rust lane.
6. C11: the runtime macros, storage in the machine, the lowering, the saved form and the snapshot. The same scenarios
   run, and `no_c11_frame_grows_with_a_list_bound` is extended to a map.
7. Algorithms: map slots, locals and returns, `<sce:foreach in>` with `key`, on the Interpreter, Rust and C11, held by
   the numerical reference (`tests/forge/conformance/numerical_reference.json`). §2.15's bounded-collection paragraph
   is rewritten (D8).
8. C++, Kotlin, Go, Python, Swift, one landing each, each removing its `generate/unsupported-feature`.

## Open questions

1. The wire form of a map in an event schema's field (D6): a CBOR map in D2's order and a JSON array of pairs is the
   likely answer, and it waits for a row that sends a table whole.
2. A declared secondary index, decided once it is known how many tables are read by two keys.
