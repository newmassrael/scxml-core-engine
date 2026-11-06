# ADR 0090 — An event's origin is the reply address its processor writes, and one value holds both fields

- Status: Accepted (2026-11-06). On 2026-11-06 the owner chose this structure over keeping the boundary conversion.
  The owner had asked for the most correct long-term structure, with cost ignored, and was given three options:
  keep `publishedOrigin` and patch its cases, convert to the published spelling where the event is raised, or carry
  one typed value that the delivering processor writes. Nothing has landed yet; Order of work lists the steps.
- Date: 2026-11-06
- Amends:
  - ADR 0084 (the processor door). A host ending or a host-posted event names its origin as the pair defined here,
    not as two free strings.
  - `schemas/sce-saved-state.v2.schema.json`, by a format 3 (section 7).
  - `SCE_WIRE_CONTRACTS.md`, the saved-state row.
- Scope:
  - the C++ Interpreter (`sce/src/runtime`, `sce/include/core/EventMetadata.h`);
  - the C++ generated engine (`sce/include/static`, `tools/codegen/templates/*.jinja2`, the Mesh templates);
  - the Rust, Go, Python, Kotlin, Swift and C11 runtimes and their templates;
  - the host surfaces `HostInvokeEnding`, `HostSendResponse` and each runtime's host-posted external event;
  - saved state.
- Out of scope: the BasicHTTP sending side. Its message format is fixed by §scxml-C-2-2 and is unchanged here.
- Related: ADR 0045 (saved state holds the tree), ADR 0084 (one inbox and one processor door per engine).

## Context

§scxml-5.10.1 defines the two fields together:

- `origin` is "a URI, equivalent to the 'target' attribute on the `<send>` element".
- For external events the processor SHOULD set `origintype` "to a value which, when used as the value of 'type',
  will allow the receiver of the event to `<send>` a response back to the originating entity at the URI specified
  by 'origin'". For internal and platform events it MUST leave `origintype` blank.

§scxml-C-1 adds a constraint for the SCXML processor. The origin of an event raised in a receiving session MUST
match the `location` that the sending session's `_ioprocessors` entry publishes. W3C test 336 asks exactly this: it
answers with `<send targetexpr="_event.origin" typeexpr="_event.origintype"/>`.

So the two fields form one reply address, and the address means something only relative to its processor. The
engines do not carry them that way.

- **Every engine except C11 carries the sender's bare session id as `origin`.** The value is converted only when
  the document reads it: `IOProcessorHelper::publishedOrigin` (`sce/include/common/IOProcessorHelper.h`) and its
  five ports leave a string containing `://` as it is and wrap anything else as `sce://scxml/<percent-encoded id>`.
  The guess by string shape is wrong in three ways:
  - A host-supplied origin that has no scheme comes out as an SCXML address that no session published. Examples:
    a `HostInvokeEnding.origin` of `motor`, or `raiseExternal(..., origin)` in C++, Rust, Go and Swift.
  - The C++ Mesh remote child's session id is `source:invoke` (`mesh_transport.h.jinja2`). It has no `://`, so the
    rule wraps it as well. The comment that justifies the pass-through ("a remote invoke's child is stamped with a
    URI") is false for that path.
  - The origintype is set separately from the origin. A free origin therefore gets the SCXML processor's type by
    default, which contradicts §scxml-5.10.1.
- **C11 carries the published location instead**, unencoded (`snprintf "sce://scxml/%s"`), and has no conversion
  step. Its saved state writes that location for queued events and the bare id for scheduled sends. Other backends
  write the bare id in both places, so a saved state does not mean the same thing across backends, although
  schema format 2 promises that "what one backend saved, another restores".
- **Kotlin and C11 invent an origin.** An external event that arrives with none is given the receiving session's
  own address and the SCXML type. This affects BasicHTTP inbound events, host-posted events, host send replies and
  host endings that name no party. The C++, Rust, Go, Python and Swift engines leave these empty.
- **Origin and origintype disagree in several places:**
  - The C++ Mesh-RPC reply sets `origintype="sce:mesh-rpc"` with an empty origin. The other six set
    `mesh://<source>`.
  - A host send reply has origintype SCXML and origin empty in the C++ generated engine, but both empty in the
    Interpreter.
  - The Interpreter and the C++ generated engine copy the `<send type>` spelling into origintype as written, so the
    alias `scxml` reaches the document. C11 always writes the canonical URI.

The bare id travels on for a reason. Three consumers that are not the document match on the session id:
`<finalize>` dispatch, cancelled-invoke filtering, and the Interpreter's choice of queue. The comment above
`publishedOrigin` records that converting earlier once broke those lookups (W3C 233 and 234). That breakage came from
asking one string to serve two readers. It does not show that the conversion belongs at the read.

## Decision

### 1. One value: `EventOrigin { processor, address }`

Each engine gains one value type with two fields:

- `processor` is the canonical type URI of an Event I/O Processor;
- `address` is a URI that the processor accepts as a `<send>` target.

An event holds either an `EventOrigin` or none. There is no state in which one field is set and the other is not:
the type is constructed whole, and `none` is the only way to express absence. The document sees
`_event.origin = address` and `_event.origintype = processor`. With no origin, both fields read as the empty string,
which is what §scxml-5.10.1 asks of internal and platform events and what test 330 requires to be present.

The session id of the sender is a separate field, `sender: SessionId?`. `origin` is the reply address and `sender`
is the identity of the sending session: two independent fields, and no rule ties one to the other. The processor
that delivered the event from that session writes `sender`:

- the SCXML processor, for a local session;
- the Mesh transport, for a remote child or parent (§mesh-9.6.3). Its address beside the sender is the session's
  endpoint `mesh://<device>/<machine>/<session>`.

`sender` is null for every other sender: a host, BasicHTTP, or a Mesh peer that is not a session of this tree.
`<finalize>`, cancelled-invoke filtering and the Interpreter's queue choice read `sender`. Nothing reads `address`
to find a session. §scxml-6.4 is about the invoked child, not about which processor carried its event.

(Amended 2026-11-06. The first text said "set only by the SCXML processor". The C++ Mesh transport writes the remote
child's id as `sender` beside its mesh endpoint, so that `<finalize>` and the cancelled-invoke filter find the child.
Under the first text, a machine that holds such an event could not be saved.)

### 2. The processor that delivers an event writes its origin, once, when it builds the event

The origin is computed where the event is made, by the processor that delivers it. No step converts it later.

- **SCXML processor (§scxml-C-1).** It covers sends between sessions (`#_parent`, `#_<invokeid>`, `#_scxml_<id>`),
  self-sends and delayed sends. The pair is `{SCXML type URI, scxmlLocation(sender)}`, and `sender` is set.
  `scxmlLocation` is the same function that writes the session's `_ioprocessors` entry. It stays the one spelling
  of an SCXML address on every runtime, so C11 percent-encodes as the others do.
- **Autoforward** copies the origin and `sender` of the event it forwards, unchanged.
- **BasicHTTP processor (§scxml-C-2-1).** The receiving rules fix the event's name and data and say nothing about
  origin. Every request parameter other than `_scxmleventname` is data, so an HTTP request carries no reply address
  that is not part of the data. The pair is none.
- **Host processors (ADR 0084).** A host that delivers an event for an invocation, a send or its own reasons writes
  the pair in its own terms:
  - `processor` is the host processor's declared type, never the SCXML type by default;
  - `address` is the URI by which a `<send>` of that type reaches the party.

  A host that names no party gives none. The host surfaces take the pair, not two strings:
  - `HostInvokeEnding` holds `origin: EventOrigin?` in place of `origin` and `originType`;
  - `HostSendResponse` gains `origin: EventOrigin?`;
  - each runtime's host-posted external event takes `EventOrigin?`.
- **Mesh transport.** The Mesh transport writes the pair for every event it delivers: peer events, RPC replies and
  events from a remote child. Its values are those SCE_MESH.md gives for reaching that peer. The C++ RPC reply gains
  the address the other six runtimes already give. A remote child's events carry the transport's pair, never a
  wrapping of `source:invoke`. Their `sender` is the child's session id (section 1).
- **Platform and internal events** (`done.*`, `error.*`, `<raise>`) have none, including `done.invoke` from an SCXML
  child (§scxml-5.10.1).

### 3. No engine invents an origin

An event that no processor stamped has none on every runtime. Kotlin's `meta.origin.ifEmpty { scriptSessionId }`
and C11's `_pending_event_origin_carry or _scxml_location` are removed, together with their SCXML-type defaults.

### 4. The processor type is canonical

`processor` holds the canonical URI. The `<send type>` alias `scxml` is read as
`http://www.w3.org/TR/scxml/#SCXMLEventProcessor` and `basichttp` as the BasicHTTP URI. The registry that resolves a
`<send type>` (§scxml-6.2.4) does this, so origintype never carries an author's spelling. `_ioprocessors` keeps
publishing both spellings, and a reply sent with the canonical type reaches the same processor.

### 5. The boundary conversion is removed

`IOProcessorHelper::publishedOrigin` and its ports (`published_origin` in Rust and Python, `publishedOrigin` in Go,
Kotlin and Swift) are deleted, along with the `://` test. The no-script `targetexpr="_event.origin"` path in
`actions/send.jinja2` reads `address`. `scxmlLocation` and `sessionIdFromScxmlLocation` remain as the SCXML
processor's address codec.

### 6. The two C++ engines and the six generated runtimes give the same answers

After this ADR, every event kind in section 2 has one origin, and the Interpreter and all seven generated runtimes
give it. The tests in the Tests section ask each event kind on every lane.

### 7. Saved state format 3

A saved state stores each queued event and each scheduled act as:

- `origin`: `{ "processor": string, "address": string }` or `null`;
- `sender`: a session id or `null`.

Format 3 is a version step. A saved state is read by a later build than the one that wrote it (the reason
`SCE_WIRE_CONTRACTS.md` gives for format 2), and the meaning of `origin` changes.

A reader of format 3 also reads formats 1 and 2. A format-2 `origin` becomes a pair as follows:

| Format-2 `origin` | Saved `origintype` | Read as |
|---|---|---|
| empty | any | none |
| a bare id (no scheme) | SCXML type, or empty | the SCXML pair of that id, `sender` = the id |
| a value starting `sce://scxml/` (written by C11) | any | the SCXML pair, `sender` = the decoded id |
| anything else | its saved type, or the SCXML type when it is empty | the pair as saved, `sender` none |

Two rows close the gap a half pair would fall into. An empty `origin` is none whatever type was saved beside it:
several format-2 writers saved the SCXML type beside an event no processor stamped, and an empty address answers
nothing. An empty `origintype` beside an address is the SCXML processor's, which is what format 2 meant by it
(`HostInvokeEnding.originType`: "empty keeps the SCXML processor's").

"No scheme" in the second row means that the text holds no `://`, which is how every reader tests it. A session id
is an XML Nmtoken and may hold a `:` (`peer:1`), so a colon alone does not mark a scheme. The third row comes before
the fourth whatever type was saved: a value that starts `sce://scxml/` and names a session is that session's pair. A
format-1 text holds `origin` and `origintype` in the same members, with the same meaning, so the table reads it too.

A waiting send of a format-1 or format-2 text stores only `origin`, which is the sending session's id, or its
location as C11 stored it. A reader takes the id, decoding a location, and reads the act as that session's SCXML pair
with the id as its `sender`. An `internal` act is the one exception, because the event it delivers is an internal
event, and an internal event has no origin (§scxml-5.10.1, section 2). Its stored `origin` is therefore not read,
and the act is read as having no origin and no sender.

A `raise`, `parent` or `child` act of a format-2 text whose stored `origin` is empty is read as sent by the session
that holds it: the `sessionid` of the session record the act sits in. The act is that session's own send whatever
was stored, and this one rule replaces the different choices the backends made. It also makes a format-2 text that
a runtime reads and writes back readable again. A format-1 act with an empty `origin` has no session id in its text,
so the machine that restores it arms it as its own send, under the id that machine runs as.

The prefix `sce://scxml/` stored alone, with no id after it, names no session, and no writer stored it:

- **In a queued event** it is the table's fourth row. The third row needs an id after the prefix, and the second a
  text without `://`. It is read as the pair it stores, `{its saved type or the SCXML type, "sce://scxml/"}`, sent
  by no session, which is how the stored-origin readers already read it. A reply sent there reaches no session, as
  it would have before the text was saved.
- **In a `raise`, `parent` or `child` act** it is refused. Such an act is a session's own send and its stored
  `origin` must name that session, by an id, by a location with an id, or by nothing (the holder, above). The
  prefix alone is none of these, and it is not an Nmtoken. Reading it as a sender would give the act a sender that
  is no session id.

This reading is one function, shared by every backend: the stored-origin reader each runtime already holds. It is
`IOProcessorHelper::readStoredOrigin` and `readStoredSender` in C++, with their ports in the other runtimes and
`sce_saved_origin_read` in C11. Two builds therefore read one format-2 text as the same pairs. A build that reads
only formats 1 and 2 refuses `"format":3` by its `format` member before it reads anything else. It never reads part
of the text.

#### Text form

Format 3 is format 2 with the origin held as the pair. Every member that is not named here is written as format 2
writes it, and the text stays compact. The bytes below are what every backend writes after the same run, and what
the shared instances hold.

- **`format`** is the JSON number `3`, the first member of the root. Format 3 applies to the whole text: a format-3
  writer writes `3` for every text it writes, including one that holds no event and no waiting send. A child
  session does not state the format again. One format per text keeps one rule for the reader (judge `format`
  first) and one for the writer.
- **`shape`** is the format-2 shape (docs/adr/0045, section 2). The pair changes no document, so each machine's
  format-2 shape constants serve format 3 as they are, and no generated tree changes because of this step.
- **Which records carry the pair.** These records carry it:
  - every element of a session's `external`, in every session of the tree, whatever delivered the event: a session's
    send, a host invocation's ending queued with its `hostinvoketoken`, a host-posted event, a host send's reply, a
    Mesh peer event or RPC reply, a BasicHTTP event;
  - every waiting send in `pending` whose `act` is `raise`, `internal`, `parent` or `child`.

  These carry no pair: a `host` act, a `hostinvokes` entry (the Mesh request `sce:mesh-rpc` among them) and an
  `invokes` entry. Each of them is a request a processor performs or a session that runs, not an event a session
  will take. When the reply to one of them arrives, it is an event, and it is queued in `external` with its own
  pair. These are the record kinds that held `origin` in format 2. `origintype` was held only by the elements of
  `external`.
- **Where the two members sit.** `origin` then `sender` take the place that format 2's `origin` (and in `external`,
  `origintype`) held:
  - an element of `external` is `name`, `data`, `type`, `sendid`, `origin`, `sender`, `invokeid`,
    `hostinvoketoken`;
  - a waiting send is `due`, `act`, then `invokeid` for a `child` act only, then `event`, `data`, `sendid`, `origin`,
    `sender`.
- **Always written.** Both members are in every record that carries the pair, and `null` stands where there is
  none. Format 2 already follows this rule: a member of a record that may hold nothing is written as `null`
  (`hostinvoketoken`, a host invocation's `due`, an absent optional value). The only members it leaves out are
  `revisions` and `features`, which a session holds or lacks as a whole, and which texts written before they
  existed lack. The pair is a member of every event, not such a member. One rule with no exception means that no
  writer decides when to leave a member out.
- **The pair** is the object `{"processor":"<processor>","address":"<address>"}`. `processor` comes first and
  `address` second, the object has no other member, and neither member is empty. Every record of the format is
  written in the order its definition declares, never sorted (`name` before `data`, `due` before `act`). Section 1
  declares `EventOrigin { processor, address }`. Format 2's order, the address `origin` before `origintype`, was
  the order of two members of the event, and it does not carry into the new object. `processor` is the canonical
  type URI (section 4). For the SCXML processor this is `http://www.w3.org/TR/scxml/#SCXMLEventProcessor`.
  `address` is the address as the delivering processor wrote it.
- **The SCXML pair's address** is `scxmlLocation(sender)`: the prefix `sce://scxml/`, then the UTF-8 bytes of the
  session id, with each byte outside `A-Z a-z 0-9 - . _ ~` written as `%` and two uppercase hexadecimal digits.
  This is the same function that writes the session's `_ioprocessors` entry. `fixture-root` is written
  `sce://scxml/fixture-root`, and `peer:1` is written `sce://scxml/peer%3A1`.
- **`sender`** is the session id as a JSON string, or `null` (section 1). It is the id of the session that sent the
  event, written by the processor that delivered it from that session. That processor is the SCXML processor for a
  local session, and the Mesh transport for a remote child or parent. No rule ties `sender` to the address beside
  it:
  - a local session's event carries `scxmlLocation(sender)`;
  - a remote child's event carries the SCXML type and the child's endpoint, `mesh://<device>/<machine>/<session>`
    with each segment percent-encoded. The C++ child id is `<parent machine>:<invokeid>`, so its `:` is written
    `%3A` there.

  A session that sent an event always has a reply address, so a `sender` never stands beside a `null` origin. An
  SCXML pair with no session behind it has `sender` `null`: the `mesh://ecu` row of a format-2 text is read that
  way.
- **A waiting send.**
  - A `raise`, `parent` or `child` act is always this session's own send, and the SCXML processor delivers it. It
    carries this session's pair and this session's id.
  - An `internal` act carries `"origin":null,"sender":null`, because the event it delivers is internal. A backend
    whose scheduled entry keeps the sender for such an act does not write it.
  - Each act carries the pair and the sender that its event will carry when it is delivered, which is why it holds
    both even where one could be derived from the other.
- **`null`** is the four letters `null`, in lower case. As in format 2, the text has no whitespace outside strings.
  A string is escaped by the one rule of `tests/json_text/string_escape.json`.

A queued event and a waiting send in format 2 (the shared tree text before this step):

```json
{"name":"answer","data":"","type":"external","sendid":"","origin":"fixture-root.working.2.worker","origintype":"http://www.w3.org/TR/scxml/#SCXMLEventProcessor","invokeid":"worker","hostinvoketoken":null}
{"due":"1700000005000","act":"child","invokeid":"worker","event":"nudge","data":"","sendid":"poke","origin":"fixture-root"}
```

The same two records in format 3, with an event that no processor stamped and an internal act:

```json
{"name":"answer","data":"","type":"external","sendid":"","origin":{"processor":"http://www.w3.org/TR/scxml/#SCXMLEventProcessor","address":"sce://scxml/fixture-root.working.2.worker"},"sender":"fixture-root.working.2.worker","invokeid":"worker","hostinvoketoken":null}
{"due":"1700000005000","act":"child","invokeid":"worker","event":"nudge","data":"","sendid":"poke","origin":{"processor":"http://www.w3.org/TR/scxml/#SCXMLEventProcessor","address":"sce://scxml/fixture-root"},"sender":"fixture-root"}
{"name":"swap","data":"","type":"external","sendid":"","origin":null,"sender":null,"invokeid":"","hostinvoketoken":null}
{"due":"1700000001000","act":"internal","event":"inner","data":"","sendid":"__send_2","origin":null,"sender":null}
```

A format-3 reader takes both members as they are written and derives neither one from the other. It refuses, and
names the record, in these cases:

- an `origin` that is neither `null` nor a pair;
- a pair with a member missing, empty or added;
- a `sender` that is not a session id;
- a queued event with a `sender` beside a `null` origin;
- a `raise`, `parent` or `child` act with no `sender`, or with an `origin` other than the SCXML pair of that
  sender, `{SCXML type, scxmlLocation(sender)}`, byte for byte;
- a `raise`, `parent` or `child` act whose `sender` is not the `sessionid` of the session record it sits in, in
  the root or in any child session. Such an act is that session's own send. A reader that took it would save it
  back under its own id, so one text would mean two things. The reason names the member, the two ids and the act,
  for example `'pending[0].sender' is 'y', and this session is 'x': a raise act is this session's own send`;
- an `internal` act that holds either member.

The reader refuses these texts and does not repair them, as it refuses every other text that it cannot read whole.
A queued event whose `sender` stands beside any pair is read as written, the pair as the address and the sender as
the identity. The writer never refuses a queued event for the combination of its `sender` and its `origin`.

One backend narrows this by what it can hold. C11 derives a sender from an SCXML location and cannot hold a sender
beside another address. C11 runs no Mesh remote child, so no C11 machine holds such an event. It refuses a text
with a `sender` beside an address other than `scxmlLocation(sender)`, and names that as the reason. Every other
backend reads such a text.

A build that reads format 3 refuses any other `format` with one reason on every backend: `the saved state is
format <the member as written>, and this runtime reads formats 1, 2 and 3`. This is the format-2 reason, with the
list of formats grown.

**What a format-3 build writes.**

- It writes format 3 for every state that it saves from a machine.
- A state read from a format-2 text holds the pairs that the table gives, and it is written back as format 3.
- A state read from a format-1 text and written back without a machine (a runtime's own read-and-write test) stays
  format 1. Format 1 has no place for a child's session. Its `origin` and `origintype` are written by the rule that
  format 2 uses today: the sender's id for a session's send and the address otherwise, then the pair's processor;
  both are empty for none.

**The fixtures.** Each backend's writer moves to format 3 in the saved-state step, all at once.

- The shared instances in `sce-build/tests/fixtures/static_datamodel/saved/` and
  `sce-build/tests/fixtures/host_processor/saved/` move with it.
  - The instances that hold an origin are moved through the table, so an SCXML pair keeps its sender.
  - Every other instance changes only its `format`.
- `sce-build/tests/fixtures/static_datamodel/saved/format2/` holds the format-2 reading fixtures:
  - the shared tree as the backends other than C11 wrote it before this ADR;
  - the same tree as the C11 writer before step 5 (cb4ca22e06^) stored a queued event's origin, the location
    `sce://scxml/<id>`. From step 5 on, C11 wrote the bare id, as the other backends did;
  - a tree whose root queue holds one event for each row of the table and each type beside it, the bare prefix
    `sce://scxml/` among them. Its two session-to-session acts are stored with an empty `origin`, and each is read
    by the `sessionid` of the session that holds it;
  - the shared timers, whose `internal` act stored the sender.
- `saved/format2/read_as/` holds, under the same name, the format-3 text that each fixture is read as. Every
  backend restores each fixture, saves at once, and gets that text. A backend picks the machine that restores a
  fixture by its name, so every name starts with `static_invoke_tree_working` or `static_timers`.
  - The C11 tree and the tree of the other backends are both read as the shared format-3 tree.
  - The timers are read as the shared format-3 timers.
- `saved/format2/README.md` lists the pair and the sender that each record is read as.
- `host_processor` has no `format2/`. Its instances hold no event and no session's send, and a host act and a host
  invocation have the grammar that format 2 gave them. Their format-2 text differs from their format-3 text only
  in `format`, and the static data model fixtures already reach every path that reads a stored origin.

## Tests

Each new integration stem is wired on all eight lanes (the Interpreter and the seven generated runtimes), in the way
`inline_content_is_read_by_its_data_model` is wired:

1. `an_origin_is_the_address_its_sender_published`. A self-send, a child-to-parent send and a parent-to-child send.
   Each time, `_event.origin` equals the sender's `_ioprocessors` SCXML location, `_event.origintype` equals the
   canonical URI, and a reply sent with `targetexpr="_event.origin" typeexpr="_event.origintype"` arrives. This is
   test 336 asked of every route. A send written with `type="scxml"` yields the canonical URI.
2. `an_event_no_processor_delivered_has_no_origin`. A host-posted event and a host send reply that name no origin
   read empty for both fields. Kotlin and C11 fail this test today.
3. `a_host_names_its_origin_in_its_own_terms`. A host ending whose origin has no scheme (`{host type, "motor"}`)
   reads back `motor` and the host type. Six runtimes fail this test today, because they wrap the address as
   `sce://scxml/motor` and default the type to SCXML.
4. On the Mesh lanes, the RPC reply's origin is the address the transport gives that peer. The C++ lane fails this
   test today.
5. Saved state. `sce-build/tests/saved_state_schema.rs` validates format 3. It also holds each format-2 reading
   fixture to the format-3 text it is read as, by the table of section 7. Each backend restores every reading
   fixture in `saved/format2/`, saves it at once, and gets the text `saved/format2/read_as/` holds for it, byte for
   byte.
6. A unit test of the SCXML address codec (`scxmlLocation`, `sessionIdFromScxmlLocation`, round trip with a
   reserved character) on every runtime. Today C++, Python and Kotlin have none.

The W3C tests that read origin (198, 230, 253, 336, 337, 349, 352) and the finalize tests (233, 234) keep passing
on every lane.

## Order of work

1. This ADR.
2. The C++ core: `EventOrigin` and `sender` in `core/EventMetadata.h` and the queue, then the Interpreter, the
   generated-engine templates, the host surfaces in `core/HostProcessor.h`, and the C++ Mesh transport. Stems 1 to
   4 land with their C++ and Interpreter drivers.
3. Rust and Go, with their drivers of the stems.
4. Python, Kotlin and Swift, with their drivers. Kotlin's invented origin is removed here.
5. C11: the encoded location, the removed fallback, and the bare-id-versus-location split replaced by the pair.
6. Saved state format 3 on every backend at once: the schema, the fixtures and `SCE_WIRE_CONTRACTS.md`.
7. Delete `publishedOrigin` and every port, and the comments that cite them.

Until step 6, each runtime keeps writing format 2: `origin` holds `sender` for an SCXML pair and `address` otherwise,
which is what each runtime writes today.

## Consequences

- One reply address means one meaning everywhere. A document that answers with `_event.origin` and
  `_event.origintype` reaches the sender on every processor, not only on SCXML.
- A host has to name the processor of the party it reports. Today the SCXML type is filled in silently. That default
  was wrong whenever the host was not an SCXML session.
- Saved states written before format 3 stay readable. Builds older than format 3 refuse a format-3 text by its
  `format`, and do not misread it.
- An event that comes in over BasicHTTP has no origin. A document that needs to answer an HTTP sender must take the
  address from the data the sender chose to send, because no rule in §scxml-C-2 gives one.

## Alternatives rejected

- **Keep the conversion at the read and fix its cases.** A conversion that must decide from a string's shape which
  processor the string belongs to cannot be right for a processor added later. Each new processor would add a case,
  and C11 would remain a separate implementation.
- **Convert to the published spelling where the event is raised, keeping one string.** That is the change that broke
  W3C 233 and 234. One string cannot serve both the document and the session-keyed lookups.
- **Let BasicHTTP read a reply address from a reserved request parameter.** §scxml-C-2-1 makes every parameter other
  than `_scxmleventname` data. A second reserved name would change what a conforming sender's data means.
