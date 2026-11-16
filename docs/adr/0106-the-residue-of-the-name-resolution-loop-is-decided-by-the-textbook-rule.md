# ADR 0106 — The residue of the name-resolution loop is decided by the textbook rule

- Status: Accepted (2026-11-16). Written and accepted under the owner's standing rule (the most long-term correct
  structure, textbook in its foundations; decided, not asked), which the owner extended on 2026-11-16 to the items
  the enum and forge name-resolution loop had left as "owner decisions" when it stopped on 2026-09-25. Written as
  0105 and renumbered 0106 the same day, 0105 being promised to the tlv-chain record that keeps ids.
- Date: 2026-11-16
- Scope: the decisions only, item by item, each with the evidence of the tree as it stood on 2026-11-16, what was
  implemented with this ADR, and what is left and whose it is. The items keep the numbers of the loop's defect
  ledger (G3 is the NL to IR closure ledger's row).

## Context

The loop stopped with six items marked for the owner and seven residual diagnostic items, plus the G3 row's
follow-up. Many had moved since: other rounds fixed some, and some decisions had been taken in passing without a
record. Each item below was checked against the tree first. "Fixed" cites the commit that fixed it.

## Decisions

### #155b — a misplaced SCE element is a move, not a rename

Implemented. `validation/misplaced-element`: when the read ledger shows another element of the document was asked
for the unread element's name, the refusal says where it is read, with `fix` `move_element` and a `destination`
related site; otherwise `validation/unexpected-child-element` as before. The destination is derived from the
parse's own reads, never from a list of where elements belong.

### `sce:*` attributes nothing reads (the forge parser's lax `xs:anyAttribute`)

Fixed before this ADR: `8f837f9933` (2026-09-28) records every `sce:` read through `sce_attr` on the parse's ledger
and refuses the rest as `validation/sce-attribute-unread`. No new code was needed.

### G3 — the root of every forge document is a claim site

Decided, not yet implemented. The `NoAnnotationSite` wording already says what SCE reads, not what the grammar
refuses (`ReviewTableUnavailable`'s message, "SCE reads sce:req on no node of this kind"); the stale comments
that still said a root `sce:req` was dropped were corrected with this ADR.

The decision: every forge document is one artifact, and its root is the one node every kind has, so `sce:req` on the
root is read for every forge kind and `forge::requirement_nodes` answers every kind with at least the root's row
(lookup keeps its per-entry rows besides). `ReviewScope::NoAnnotationSite`, `CliError::ReviewTableUnavailable` and
`cli/review-table-unavailable` then become unreachable and are removed in the same change (the schema is
`pre-release`, SCE_ERROR_CONTRACT.md section 8.1). The change is wider than the parser: the root claims are a
fact of `ParsedForge` (as `kind_basis` is), so `requirement_nodes` and its four callers (`review_table`,
`requirement_manifest`, `requirements_report`, `acceptance_report`) take it; the pseudocode page writes the root's
`req` lines and `unpseudo` reads them back (the page is total by contract); the forge AST export and its schema gain
the field.

Implemented (2026-11-16): the forge parse reads the root's `sce:req` through `collect_sce_req` into
`ParsedForge::requirements` (empty for an inline kind, whose claims are its statechart's, and for a statechart, whose
claims stay in its model); `requirement_nodes` takes the `ParsedForge` and answers every kind with the root's row
(`document`, the detail `<kind> '<name>'`) before a lookup's entries; `review_table`, `classify_document`,
`citations_of_document`, `emit_forge_requirements_ndjson` and the acceptance record's evidence take the
`ParsedForge` too; the pseudocode page writes the root's `req` lines under its head line
(`pseudo::with_root_claims`, beside the kind basis); the forge AST schema gains `requirements`.
`ReviewScope`, `NoTable`, `CliError::ReviewTableUnavailable` and `cli/review-table-unavailable` are removed (538
codes).

### #142 — a link carries the events it declares

Decided: implement the RFC's codegen contract, `driver.poll() -> bytes -> framer.decode() -> event inject` and
`event extract -> framer.encode() -> pool slot -> driver.send()`, in every link backend. The RFC's own invariant
(section 2.4, "kinds are additive": added when wired, never a built-but-unconsumed parser surface) rules out the
alternative of rewriting the comment and the RFC to say a link is a byte pipe while `<sce:events>` stays parsed and
unread. Left to the owners of the link templates (`tools/codegen/templates/forge/{rust,c}/link*.jinja2`), with the
framer codec resolved across imports; the template header comment is to be made true by that change, not edited
ahead of it.

Implemented (2026-11-16), on the two backends that generate a link (Rust and C11; the kind is MCU-class elsewhere):
a link that declares `<sce:events>` rows is generated against the codecs they name, read through the link's own
`<sce:import kind="codec">` (`forge::generator::link_events`). The receive path (`pump`) gives the driver its tick,
decodes every frame with the framer and injects, through a callback, the event of every `<sce:inbound>` row whose
`when` holds, in document order; each row is a rule of its own, so a frame may raise several events and a row
without `when` raises its event for every frame. `when` is an expression over `decoded`, a closed record of the
framer's fields, lowered by the forge expression pipeline and refused where it was written. A frame the framer
refuses raises no event and is handed to a second callback. The transmit path (`send`) encodes an `<sce:outbound>`
event's value with its codec into a slot of the `<sce:tx-pool>` when the link names one (acquired, written, handed
to the driver, returned: `Link::tx` copies the bytes before it returns, so the slot is free whichever way the send
went) and into a buffer of the codec's largest encoding otherwise. The listener's established-session half carries
the same paths. Refused at generation, naming the link: a codec a row names that the link does not import (its
decode and encode are generated from its document, which only an import reads), a framer whose decode takes caller
inputs (flag inputs, a caller-supplied tag), an outbound codec whose encode does, an outbound event declared twice,
and two event names one identifier would spell. A link with no event row stays the byte endpoint it was.

Held end to end (2026-11-16): the five documents of `tests/forge/conformance/link_events/` (a framer, a second codec,
a transmit pool, a link through the pool and one without) are generated and compiled by the Rust forge arm
(`forge_link_events.rs`, the modules as siblings) and the C11 arm (`link_events_c`, under `-Werror`), and each drives
`pump` and `send` over a driver whose wire loops back, so a frame sent is the frame the next pump decodes.

### #127 — a deployment's refusal names the deployment's path

Implemented. `DeployConfig` keeps the path it was read from (`source`, set by `parse_deploy` and by the CLI's
loader), and every set-level refusal and artifact label of `compile_scxml_with_imports` uses `DeployConfig::label`,
the literal `deploy.yaml` only for a deployment given as text.

### #76 — a deployment's refusal carries the row of the key it names

Decided: yes. A diagnostic locates the author's input (SCE_ERROR_CONTRACT.md section 2.2); a key path is a name, not
a coordinate a consumer can open. `serde_yaml_ng` keeps no marks, so the deploy reader gains a marked parse of the
same text (a YAML event reader that reports line and column), consulted only on the failure path to map the dotted
key path a refusal already names (`machines.x.bindings.y`) to the row of its last key. Left: it adds a dependency,
which needs a lockfile change and a build this ADR could not make.

Implemented (2026-11-16): `yaml-rust2` 0.11 (MSRV 1.65, without its input-encoding feature) is the marked reader
(`mesh::deploy_marks`). `DeployError::key_path` gives every refusal the key it is about, by an exhaustive match, so a
new refusal cannot be added without deciding where it stands; the refusals about no key (an unreadable file, a YAML
syntax error that serde places in its message, an identifier count past its ceiling, a link no deployment declares)
answer `None`. A path names the machine's device as any key (a refusal names the machine, not its device), a
queryable by its `name`, a machine of a partition's list by its value, and an unknown binding key by the dotted
location the refusal already spelled. The lookup stands on the deepest key of the path that was written (a missing
field stands on the entry it is missing from) and, when the record carries `actual`, on the key or value of that
entry or a parent of it that is `actual`. `DeployConfig` keeps the text it was parsed from; the CLI's deploy reader,
the mesh transport compile and the orchestrator's deploy-side checks place their refusals through it.

Implemented (2026-11-16): the YAML refusal is placed too. `DeployError::Yaml` carries serde's words and, apart from
them, the row and column `serde_yaml_ng::Error::location` gives (`Yaml { message, at }`); `mark_in` answers that
place, so the record's `location` holds it, and the ` at line L column C` clause serde writes into its words is
removed by the numbers it was built from. The record's id is made of the words alone, so it no longer moves when a
line is added above the refusal.

### #112 — an arriving event name is matched by its token prefix

Fixed before this ADR: `a6806700fb` (2026-10-04) resolves an arriving name to the longest token prefix the document
writes, on all six backends, held by one shared scenario.

### #102 — an inline kind is an artifact, not a value of the datamodel

Decided: keep the explicit host binding SCE_FORGE.md already states (section 4.4 and "Kind Reference Resolution",
corrected in `c8d25c2f92`); do not bind a guard's name to an inline kind by matching its inputs to the statechart's
data by name, which would make the meaning of a guard depend on a coincidence of names.

Implemented (2026-11-16): the statechart's identifier resolver knows the inline kinds a document declares
(`DocumentScope::inline_kind`), and a read of one's id is `expression/inline-kind-not-a-value`, naming the kind,
with no fix, where it was `expression/unknown-identifier` telling the author to declare a name that already was.

### #184 — the `code:` vocabulary is removed from the C++ parser

Decided: remove `<code:action>`, `<code:external-action>`, `code:guard`, `code:scope` and
`GuardParser::parseExternalImplementation` from the C++ interpreter's parser. They are recognised by prefix string,
not by namespace; nothing in the tree uses them; no AOT backend has a counterpart, so keeping them is a partial
feature and an engine parity breach; `<sce:action>` replaces them.

Implemented (2026-11-16): `isActionNode` takes `<sce:action>` by its namespace and nothing else named `action`;
`isExternalActionNode`, `parseExternalActionNode` and both `parseExternalImplementation`s are gone, with the guard
node's external class and factory that only the latter set; `code:guard` and `code:scope` are no longer read. The
one live effect was `<code:external-action name="op">`, which became a script action `op`; it is skipped now, as
every element of another vocabulary is (`ForeignExecutableContentTest.TheEarlierCodeVocabularyIsNotReadAsActions`).
The public headers (`parsing/ActionParser.h`, `parsing/GuardParser.h`, `model/IGuardNode.h`, `model/GuardNode.h`)
land with the embed manifest emitted again.

### #178 — whitespace between comments in a body

Fixed before this ADR: `16f527c9e4` (2026-11-05) parses with `parse_ws_pcdata`, and `getTextContent` concatenates
every PCDATA and CDATA child, so `x<!--a--> <!--b-->y` reads `x y` in both engines.

### #74 — an undeclared enum value is refused with the values to pick from

Implemented. `validation/enum-value-not-declared`, `fix` `replace_one_of` over the declared values in declaration
order. A value past the enum's carrier stays `validation/cross-kind-type-mismatch`: no variant can be meant by it.
This supersedes the earlier design lock that reused the mismatch code: SCE_ERROR_CONTRACT.md section 3.2 puts a
closed set in `fix`, and a set carried as prose in a message is not one a consumer can read.

### #79 — an import cycle is named by its documents

Implemented. The compiled document is on the walk's stack when it is a file, and the cycle's start is found by
canonical path, not by the `src` spelling. (The record's `actual` was already dropped by `94e56bc16c`.)

### #84 — a manifest cycle is one cycle at its imports

Implemented. The documents Kahn's sort leaves are walked to one cycle, begun at its least path; the record stands on
the cycle's first import with the others as `also-refused` sites.

### #83 — a value placed at its attribute is reported as written

Implemented in `located()`, where every node-placed refusal passes: when the record names an attribute's decoded
value and the document escapes it, `actual` is the value as written; the id keeps the decoded value.

### #81 — what `col` means

Decided: `col` is the column at which `actual` begins on `line` when the record carries `actual`, and the column at
which the element begins otherwise; the guard of SCE_ERROR_CONTRACT.md section 3.1.1 is then `line[col-1..]` starts
with `actual`.

Implemented in part (2026-11-16): SCE_ERROR_CONTRACT.md section 2.2 defines `col` so, and `forge::parser::located`,
where every node-placed forge refusal passes, places it so (the value's start for a value, the name's for a name, the
tag's for a tag). Left: the statechart parser's own placements (some 38 sites in `parser.rs` and a handful in
`scxml_identifier.rs`, `test_component/rules.rs`, `product_line.rs`, `declared_names.rs`) are moved by measuring them
on the corpus, which takes a run this change could not make; the §3.1.1 guard (`wire_site::locate`) is tightened to
`line[col-1..]` starting with `actual` in the change that moves the last of them, not before, since a guard tightened
ahead of its producers fails on records that are right by the rule they were written to.

Implemented (2026-11-16): every node-placed statechart refusal (the null and `sce-static` data model rules, the
`sce:candidates` / `sce:targets` / `sce:types` / `sce:req` / `sce:provenance` / `sce:unhandled` readers, executable
content, `<invoke>` ids and mesh-rpc targets, drivers, session roles, `<assign>` and `<param>`, the root, the
product line and the test component's elements) is placed by `forge::parser::located`, so there is one placement for
both parsers; an unresolved marker stands at its id's value and an unknown standard document at its `src` value. The
sites that already stood at the value they report (`scxml_identifier`, `declared_names`, the typed invoke
attributes) keep their own placement. The guard (`wire_site::locate_at`, read by `record_misplaced` and the corpus
sweep) holds a record's own `actual` to its column. A `related` entry is held to its row only: its producers were not
measured in this change.

Implemented (2026-11-16): the `related` producers, measured by reading each. A site on an element is placed by
`forge::parser::related_at`, which shares `located`'s rule (`token_place`): the misplaced element's `destination`
at its tag's name, a feature model imported twice at the first import's `src` value, the accept-side naming rule's
record and `also-refused` sites at each state's id (both stood at the `<`). A feature-gate `referent` stands at the
referent's id value and a native action's `conflicting-use` (and its record) at the `name` value, from the reader's
recorded positions. The sites with no column (an import cycle's other imports, a generation error's positioned rows,
an artifact collision's first writer, an import route's step without `actual`) claim none, which the rule allows;
an `<invoke>` route step already stood at its token. The corpus sweep now hands each entry's `col` to
`wire_site::locate_at`, so a related site is held to its column as the record's own `actual` is.

### #53g — `actual` is an input token

Partly fixed before this ADR (`94e56bc16c`: counts and cycle paths are no longer `actual`).

Implemented (2026-11-16): the golden scan, over the 527 goldens of 2026-11-16, found twelve whose `actual` read as
prose, a relation or a computed value. Two are input tokens as written (an extern's `sig`, a guard's text) and stay.
The other ten now carry the token as written or none: a transform's output cycle and a per-peer-quota relation, a
queue's list of storage elements, five `cli` refusals whose `actual` was SCE's sentence (`acceptance-lapsed`,
`requirement-closure-broken`, `pseudo-unavailable`, `diagram-unavailable`, `diagram-does-not-fit`), carry none; a
worker's forbidden import carries its alias and a foreign attribute its value, where both carried an element SCE
re-spelled; a deployment's `discovery:` block carries the key. Every id is unchanged: the key fragments did not
move.

### #111 — `_event.data.reason` is structured

Decided: the reason an `error.*` event carries is a record (`code`, `message`), not a sentence with a code inside
it, on every engine. Left to the engine owners.

## Consequences

The implemented items land with the commits that cite this ADR's items. The items left above are open debt with a
decision, not open questions: none of them is to be put to the owner again.
