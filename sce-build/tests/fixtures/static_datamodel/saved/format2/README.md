# Format-2 reading fixtures

These are saved states in format 2 (`schemas/sce-saved-state.v2.schema.json`), kept for the readers of format 3.
docs/adr/0090, section 7, says how a format-2 origin is read. Every backend restores each text here with a machine
of its document, at the wall-clock moment `1700000000000`, as for the shared instances in `../`. It saves at once
and must write, byte for byte, the format-3 text of the same name in `read_as/`. Each `read_as/` text is also the
pair and the sender that each record below must be read as. `sce-build/tests/saved_state_schema.rs` holds each
`read_as/` text to the table of section 7, applied to its fixture.

A backend picks the machine that restores a fixture by the fixture's name. Every name here therefore starts with
`static_invoke_tree_working` or `static_timers`, and its `read_as/` twin has the same name.

| Fixture | What it is |
|---|---|
| `static_invoke_tree_working.json` | The shared tree as the backends other than C11 wrote it before ADR 0090. The sender is stored as its bare id with the SCXML type. A host-posted event (`swap`) is stored as an empty origin with the SCXML type beside it. |
| `static_invoke_tree_working_c11.json` | The same tree with the queued `answer` stored as the C11 writer before ADR 0090 step 5 (cb4ca22e06^) stored a queued event's origin: the sender's location `sce://scxml/<id>`, unencoded. From step 5 on (7b6b026129 among them), C11 writes the bare id, as the other backends do, so this text stands for cb4ca22e06^. It is the shared tree with that one member respelled. |
| `static_invoke_tree_working_origins.json` | The shared tree with the root's queue holding one event for each row of the table, and for each kind of type saved beside it, the prefix `sce://scxml/` alone among them. Its two session-to-session acts are stored with an empty `origin`. It is written by hand on the shared text. |
| `static_timers.json` | The shared timers as every backend wrote them. Its `internal` act stored the sender. |

`static_invoke_tree_working.json` and `static_invoke_tree_working_c11.json` are both read as the shared format-3
tree `../static_invoke_tree_working.json`. `static_timers.json` is read as `../static_timers.json`.

## How each record is read

`SCXML` stands for `http://www.w3.org/TR/scxml/#SCXMLEventProcessor`, and `W` for the child's id
`fixture-root.working.2.worker`. "Row" is the row of the table in section 7.

### Queued events (`external`)

| Fixture | Record | Stored `origin` | Stored `origintype` | Row | Read as `origin` | Read as `sender` |
|---|---|---|---|---|---|---|
| tree, c11, origins | child's `external[0]` `swap` | empty | `SCXML` (tree), empty (c11, origins) | 1 | `null` | `null` |
| tree, origins | root's `external[0]` `answer` | `W` | `SCXML` | 2 | `{SCXML, sce://scxml/W}` | `W` |
| c11 | root's `external[0]` `answer` | `sce://scxml/W` | `SCXML` | 3 | `{SCXML, sce://scxml/W}` | `W` |
| origins | root's `external[1]` `report` | `W` | empty | 2 | `{SCXML, sce://scxml/W}` | `W` |
| origins | root's `external[2]` `go` | empty | `SCXML` | 1 | `null` | `null` |
| origins | root's `external[3]` `swap` | empty | `x-sce-host` | 1 | `null` | `null` |
| origins | root's `external[4]` `stamped` | `sce://scxml/peer:1` | empty | 3 | `{SCXML, sce://scxml/peer%3A1}` | `peer:1` |
| origins | root's `external[5]` `ask` | `sce://scxml/peer%3A1` | `x-sce-host` | 3 | `{SCXML, sce://scxml/peer%3A1}` | `peer:1` |
| origins | root's `external[6]` `arm` | `mesh://ecu` | empty | 4 | `{SCXML, mesh://ecu}` | `null` |
| origins | root's `external[7]` `rest` | `motor` | `x-sce-host` | 4 | `{x-sce-host, motor}` | `null` |
| origins | root's `external[8]` `back` | `mesh://ecu` | `SCXML` | 4 | `{SCXML, mesh://ecu}` | `null` |
| origins | root's `external[9]` `question` | `sce://scxml/` | `SCXML` | 4 | `{SCXML, sce://scxml/}` | `null` |

Three rows pin points that a reader can get wrong:

- `stamped` is an unencoded location. It is read by the id it names and written again percent-encoded, as the
  session publishes it.
- `rest` is an address with no scheme beside a type that is not the SCXML type. It is the host's own pair, not a
  session's id.
- `question` is the prefix alone. Row 3 needs an id after the prefix, and row 2 a text with no `://`, so it is
  row 4: the pair as saved, with no sender. It is never a sender of the text `sce://scxml/`.

Every root event of `static_invoke_tree_working_origins.json` has a name of its own. A test that keys the queue by
name reads all ten rows. `question` is an event the root document sends to its child, so every backend's root
machine knows the name.

### Waiting sends (`pending`)

| Fixture | Record | Stored `origin` | Read as `origin` | Read as `sender` |
|---|---|---|---|---|
| tree, c11 | root's `pending[0]`, `child` act `nudge` | `fixture-root` | `{SCXML, sce://scxml/fixture-root}` | `fixture-root` |
| origins | root's `pending[0]`, `child` act `nudge` | empty | `{SCXML, sce://scxml/fixture-root}` | `fixture-root`, the root's `sessionid` |
| tree, c11 | child's `pending[0]`, `parent` act `report` | `W` | `{SCXML, sce://scxml/W}` | `W` |
| origins | child's `pending[0]`, `parent` act `report` | empty | `{SCXML, sce://scxml/W}` | `W`, the child's `sessionid` |
| timers | `pending[0]`, `internal` act `inner` | `fixture-root` | `null` | `null` |
| timers | `pending[1]` to `pending[3]`, `raise` acts | `fixture-root` | `{SCXML, sce://scxml/fixture-root}` | `fixture-root` |

- The `internal` act delivers an internal event, which has no origin (section 2), so its stored sender is not read.
- A `raise`, `parent` or `child` act stored with an empty origin is the own send of the session that holds it. It is
  read by the `sessionid` of the session record it sits in.
- A session's act stored as the prefix `sce://scxml/` alone names no session and is refused. No fixture here holds
  one, because a backend cannot restore it.

## C11 only

C11 derives a sender from an SCXML location, and it cannot hold a sender beside another address. It runs no Mesh
remote child, so none of its machines holds such an event. A format-3 text with a `sender` beside an address other
than `scxmlLocation(sender)` is refused by C11, and the reason names that. Every other backend reads it. None of
these fixtures holds such an event, so C11 restores them all.
