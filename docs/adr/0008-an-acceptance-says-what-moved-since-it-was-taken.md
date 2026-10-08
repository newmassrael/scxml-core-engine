# ADR 0008 — An acceptance says what moved since it was taken

- Status: Accepted for step 3 (implemented and tested, see "What step 3 measured")
- Date: 2026-10-08
- Scope: `sce-build/src/acceptance_record.rs` (`evidence_delta`), the `acceptance-delta` command
  in `sce-build/src/bin/sce_codegen.rs`; the authoring MCP's `scxml_acceptance_delta`. Step 3 of
  the chain begun in ADR 0006
- Related: `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`,
  `docs/adr/0007-an-acceptance-keeps-what-each-requirement-rests-on.md`,
  `SCE_WIRE_CONTRACTS.md` (acceptance-record row)

## Context

ADR 0006 gave a requirement an id that means the same requirement in two revisions of its
specification. ADR 0007 made an acceptance record keep, per requirement, the digests of the rows
the report showed the owner for it. Nothing yet COMPARES them. When an acceptance lapses the
product still says only that bytes moved (`Lapse::Moved`), and an owner asked to accept again has
to read every requirement's block again.

### Why two records are not enough

The obvious comparison is record against record. It cannot say WHERE anything moved: a record
holds digests, and a digest of a row carries no position the owner can open. What can say where
is the design as it is now, because the current rows are in hand with their `node_path`. So the
comparison is the old record's digests against the current design's rows:

- a current row whose digest the record does not have is **moved**, and it has a location;
- a recorded digest no current row has is **gone**, and is only a count: the row that held it no
  longer exists to point at.

### Constraints that still bind

- **It states and does not enforce.** The acceptance still lapses by bytes. A delta says what a
  second look should be about; it does not say the design is fine.
- **It is a report.** Like `acceptance-impact`, it exits 0 whenever it ran. What it cannot answer
  (a record that does not read, a design that does not parse) is a refusal, not a finding.
- **No sentence is committed or printed.** Locations are the node paths the report already prints.

## Decision

**`sce-codegen acceptance-delta <record> --root <root> [--design <scxml>]` compares a record's
evidence with the evidence of a design now, per requirement.**

1. **The design compared is the record's own document by default** (`root` + the record's
   `document`): the common question is "I edited the design that was accepted; what moved?".
   `--design` names another draft, so a re-drafted document is compared with the accepted one.
2. **Per requirement, one of four, over the union of the ids in the record and in the design:**
   - `unchanged`: the same multiset of digests;
   - `changed`: some digest differs. The line carries `moved` (the locations of the current rows
     the record lacks) and `gone` (how many recorded rows no current row matches);
   - `new`: cited now, no evidence in the record. Carries `at`, the locations;
   - `dropped`: evidence in the record, cited by no node now. Carries `gone`.
   A requirement the manifest lists that neither side cites has no line (the existing
   `missing` outcome of `requirements` already says that).
3. **The rows that claim nothing** get one line: `added` (locations of current unclaimed rows the
   record lacks) and `gone` (a count).
4. **Multisets, not sets.** Digests include position, so two equal digests are two rows. A row
   moved by an insertion changes digest, so it reads as one `moved` and one `gone`: the honest
   report of a thing that is not at the same place.
5. **A record from before evidence cannot be compared and is refused**, not read as "everything is
   new". A record with no `evidence` and no `unclaimed` against a design that has rows predates the
   field (a design with nodes always has unclaimed rows or cited ones); one against a design with
   no rows either (a kind with no annotation site) compares as equal.
6. **Wire.** Lines of JSON on stdout, `v: 1`, in the manner of `acceptance-impact`:
   `{"v":1,"kind":"acceptance-delta","requirement":"R3","evidence":"changed","moved":[...],"gone":1}`,
   `{"v":1,"kind":"acceptance-delta-unclaimed","added":[...],"gone":0}` and
   `{"v":1,"kind":"acceptance-delta-summary","record":P,"requirements":N,"unchanged":a,
   "changed":b,"new":c,"dropped":d}`. They carry the status of the acceptance-record row of
   `SCE_WIRE_CONTRACTS.md`, as the `acceptance-impact` lines do.

## What this does not claim

- **Not "this requirement is still satisfied".** `unchanged` means the product's closure of what
  the requirement depends on reads the same. A dependency the closure does not follow can move
  without it (ADR 0007 says so for the digests, and this inherits it).
- **Not a statement about the words.** Whether the requirement's sentence changed is the
  lineage's (`delta` of `scxml_requirement_set`). Joining the two columns is step 5.
- **Not the reached design elements of a changed SENTENCE.** That needs the product's reachability
  and guard analysis from the requirement's cited nodes outward, and is what step 4 gets from the
  same `fragment`.

## What step 3 measured

Criteria set before the code, and what `sce-build/tests/an_acceptance_says_what_moved_since_it_was_taken.rs`
found, on the alarm of ADR 0007 (a timer, three cited requirements, one nobody cites) and a lookup:

1. **Met.** A design identical to the accepted one reports every requirement `unchanged`, no
   unclaimed change, and no line for the requirement nobody cites.
2. **Met, and wider than the sentence said.** Retiming the delay reports `changed` for R2 and R3
   (R2 enters the alarm that arms the timer, so the closure holds it), each with `moved` naming
   the `<send>`'s place (`states.alarm.on_entry_blocks…`) and `gone` 1. R1 is `unchanged`. The send
   claims nothing itself, so it is also the one unclaimed row added and the one gone.
3. **Met.** A transition inserted ahead of a cited one reports its requirement `changed` and names
   `states.armed.transitions[1]`, the place the row has now.
4. **Met.** A citation removed reports `dropped`; one added reports `new`, with the places of the
   closure (the cited transition and the entry that arms the timer), not the citing node alone.
5. **Met.** A state no requirement reaches reports its rows as unclaimed additions and moves no
   requirement.
6. **Met.** A lookup's edited entry reports its requirement `changed` with the entry's place.
7. **Met.** A record from before evidence, and a design that does not parse, are refused; through
   the binary both exit with `cli/closure-input-unusable`.
8. **Met.** Through the binary the lines are the ones the library computes, a changed design still
   exits 0, and `--design` compares another draft.

Added after the first run: a row taken out of the design is `changed` with nothing moved and one
gone. Without that case a comparison that looked only at what is new would have passed every other
test. Three ways of breaking the code were put to the tests (counting nothing as gone, calling a
requirement unchanged when only rows are gone, and not refusing a record without evidence) and each
was caught. Clippy with warnings denied passes.

Not done in this step: nothing joins this with the words of a requirement (the lineage's `delta`),
and nothing yet finds the elements a changed requirement reaches in a NEW draft. Those are steps 5
and 4.
