# ADR 0007 — An acceptance keeps what each requirement rests on

- Status: Accepted for step 2 (implemented and tested, see "What step 2 measured")
- Date: 2026-10-08
- Scope: `sce-build/src/acceptance_record.rs` and the `accept` command; the authoring MCP's
  `scxml_accept`. Step 2 of the chain begun in ADR 0006
- Related: `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`,
  `sce-build/src/acceptance_report.rs` (`fragment`), `sce-build/src/transition_table.rs`,
  `sce-build/src/forge/review_table.rs`, `SCE_WIRE_CONTRACTS.md` (acceptance-record row)

## Context

ADR 0006 gave a requirement an id that means the same requirement in both revisions of its
specification. To say which requirements a revision touched, and later to show an owner only
what moved, the other half is needed: **what an acceptance rested on, per requirement, in a form
a later draft can be compared with.** Today an acceptance pins the manifest, the design's files,
the specification, the decision record, the profile and the scenario set by sha256, and states
what it was accepted WITH (`applied_rules`, `open_at_acceptance`). It says nothing per
requirement, so when it lapses, the only thing it can say is that bytes moved.

### What the product already computes

The acceptance report (`acceptance_report.rs`, RFC §7a) does not show an owner "the nodes
carrying the id". It shows the **dependency closure** of each cited node, because choosing a
block by `sce:req` alone leaves it byte-identical when a delay is retimed: of 72
(requirement, dependency) pairs measured on the real fixture pair, 39 lie on nodes that do not
carry the id. `fragment(model, id)` returns that closure: the cited nodes, the `<send>` that
arms a cited transition, the `<cancel>` of it, the target's entry actions, the source's exit
actions and the transition's own actions. The report prints the rows of the statechart's
transition table that those node paths name, and never renders a node itself ("one renderer, not
two").

So what a person was shown for requirement `R3` is a set of table rows. A forge document has the
same object in its own columns (`forge::review_table`, `source | node | type | detail`).

### Two constraints from ADR 0006 that still bind

- **No sentence is committed.** A record is committed. Anything new in it is a digest or a
  number.
- **A record lapses by bytes, on purpose** (`acceptance_record.rs`, "Bytes, not a model"). This
  ADR adds no way for an acceptance to keep standing through an edit. What it adds is a
  statement, in the record, of what the owner was shown for each requirement, so that the NEXT
  acceptance can be offered as a difference.

## Decision

**A record states, for each requirement the design cites, the digests of the rows the report put
in front of the owner for it. It states and does not enforce, like `open_at_acceptance`.**

1. **`evidence`**: an object from requirement id to a sorted list of sha256 digests, one per row
   of that requirement's fragment (a statechart) or per row that claims it (a forge document).
   An id appears if some node of the design cites it. A requirement the design does not cite has
   no key: "no evidence" is not an empty fragment (`fragment`'s own rule).
2. **`unclaimed`**: the digests of the rows that carry no requirement id themselves, which are
   the `(none)` block of the table. ⚠ That is "carries no claim", not "nothing asked for it": an
   unannotated row a cited node depends on (the `<send>` that arms a timer, say) is in
   `unclaimed` AND in the fragments that reach it, so retiming that delay moves both. A later
   draft that adds an unclaimed row has changed something no requirement id names.
3. **What a digest covers.** The row with its `source` column left out (the claim is the key,
   not the content) and with everything else in, **its position included**: `node_path` for a
   statechart row, `node` for a forge row. Transition order inside a state is behaviour in SCXML
   (the first enabled transition wins), so a digest that left position out would call two
   differently ordered machines equal. The cost is the direction the product chooses: a
   transition inserted before others in the same state moves the digests of the ones after it,
   and the owner is asked for a second look at requirements that did not need one. A false
   "unchanged" costs the acceptance its meaning; a false "changed" costs a look.
4. **Same reading as the report.** The record takes its rows from the functions the report calls
   (`fragment`, `transition_table`, `review_table`), so what the owner was shown and what the
   record keeps are one list. There is no second walk, and no second definition of what depends
   on what.
5. **`succeeds`**: optional `{path, sha256}` of the record this one replaces, set by
   `accept --succeeds <record>`. The previous file must be a record the library reads, for the
   same `doc_id`; the path is relative to the record's root like every other pin. It states and
   cannot lapse: it says how the acceptance stood when it was taken. It makes a chain of
   acceptances auditable and lets a later step find the predecessor without being told.
6. **Wire.** Every field is optional and omitted when empty, so a record taken without them keeps
   its bytes and an old record reads as before (`SCE_WIRE_CONTRACTS.md`: an optional field is
   added without a `v` bump; `deny_unknown_fields` stays, so an unknown field is still
   refused). `from_json` refuses what `take` would not write: a digest that is not 64 lowercase
   hex characters, an empty id, an unsorted list.

## What this does not claim

- **The fragment is the product's closure, not a proof of behaviour.** Behaviour a requirement
  depends on outside it (a variable's initial value read by a guard, say) is not in its evidence.
  The record says what the owner was shown; the byte pin is what makes the acceptance lapse.
  Step 5 must say "carries over on the evidence of equal fragments" and must not say "unchanged".
- **A multi-document design is read through its entry document.** The companions are pinned in
  `inputs` as before; a claim made by a companion's own nodes is not in `evidence`.
- **Nothing here compares two records.** That is step 3.

## Consequences

- A record taken after this change is larger by 64 hex characters per row. A design with 40
  requirements and a few rows each adds a few kilobytes.
- `acceptance-impact` and `acceptance-check` are unchanged: the new fields do not lapse.
- The Python tool `scxml_accept` gains an optional `succeeds`; nothing else on the authoring side
  changes.

## What step 2 measured

Criteria set before the code was written, and what the tests in
`sce-build/tests/an_acceptance_keeps_what_each_requirement_rests_on.rs` and
`cli_acceptance_record_wire.rs` found, on a small alarm with one timer and on a lookup:

1. **Met.** A record taken twice over the same files is byte-identical.
2. **Met, and wider than the sentence said.** Retiming the delay moves the evidence of R2 and R3
   and not R1's. R2 moves too because the alarm it enters arms the timer on entry, which the
   product's own closure counts as R2's evidence (`TargetEntry`): the closure, not the id, decides.
3. **Met.** A state no fragment reaches adds `unclaimed` rows and moves no requirement's evidence.
4. **Met.** A record from before this change reads and writes back as it was (compared as JSON, so
   key order is not what is asserted), and one that has evidence round-trips.
5. **Met.** A short or upper-case digest, a list out of order, an empty id, an unknown field, a
   predecessor that climbs out of the root or whose digest is short or which carries another field,
   a file that is not a record, and a record of another specification are refused.
6. **Met.** `accept --succeeds` writes the pin through the binary, the next record still holds,
   and a file that is not a record is refused with the unusable-input code and leaves no record.
   A lookup's review rows are its evidence, and editing one entry moves only its requirement.

Also pinned: a transition inserted ahead of a cited one in its state moves that requirement's
evidence (position is in the digest), and deleting the predecessor does not lapse the acceptance.

Four ways of breaking the code were put to the new tests and each was caught: leaving the row's
position out of the digest, taking the citing nodes instead of the closure, accepting a record of
another specification as the one replaced, and switching off the order check on a list.
Clippy with warnings denied and the rest of the acceptance tests (2225 library tests and seven
integration files) pass.

Not done in this step: nothing compares two records, and no report uses the digests yet. That is
step 3 and step 5.

## A row that omitted an attribute omitted it from the evidence (2026-10-08)

The evidence is a digest of the report's rows, so it can only see what a row shows. The review
table's header said in so many words that a transition's own attributes were not yet in it, and
that was carried into this ADR's scope without being named as a hole: a transition written
`external` and the same transition written `internal` had byte-identical rows, so
`acceptance-delta` called the requirement `unchanged` for a change that decides whether the source
state's `<onentry>` runs again (§scxml-3.13). A review of the revision tools found it.

- **The cell.** The `to` cell of a transition now ends in `type=internal` (or whatever the
  attribute says) when it is not the default `external`. The default is left out, so no row of a
  document that never wrote the attribute changes; spelling the default out moves nothing.
- **⚠ A record already taken IS affected, in one direction, and the first version of this
  section said otherwise.** The digests an earlier build wrote for a design with an `internal`
  transition are the ones `external` has now, so such a record compared with the same design
  changed to `external` reads `unchanged`: the defect again, through the record. A review found
  it the same day. The record now names the rule its digests were made under
  (`evidence_rule`, 2 from this change; absent means 1), and `acceptance-delta` refuses a record
  of any other rule and says to take the acceptance again. `acceptance-check`, which lapses by
  bytes, is untouched, and a record that states no evidence stays byte-identical (it has no
  rule to name). The rule moves whenever what a digest covers moves, as a column added to a row
  would.
- **The proof is a sweep, not a list.** `an_attribute_that_changes_a_transition_changes_its_row`
  moves every attribute of every transition of a fixture, one at a time, and requires the row to
  move whenever the parsed model of the transition does. Before the cell changed it found exactly
  this: 13 moves changed a transition's model and the 3 that were `type` hid. It also holds the
  states (`initial`, a history's `type`) and found nothing hidden there. What it does not hold: a
  state's `id` (renaming moves every reference to it), and the transition inside an `<initial>`
  or a `<history>`, which is an action row.
