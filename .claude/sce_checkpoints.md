# Admissible next checkpoints for this repository's AI loop

`.claude/sce_loop.scxml` names `.claude/sce_admits.sh` as its `successor_check`,
and that script reads this file. A checkpoint the loop proposes for itself is
admitted only when the proposal names one of the keys below.

## Why this file exists rather than a sentence in a prompt

The loop plugin states the reason in its own words: *"reasoning written as prose
is measured by nobody; make it a predicate a document or an artifact can be
asked."* Before this register existed the slot held `claude -p`, and on run 341
(2026-09-12) that instrument answered nothing three times in a row, so a run that
had finished its milestone closed `unadmitted` instead of continuing. Every other
answer is a refusal by design, so an instrument that cannot answer stops the loop.

## ⚠ The loop must not edit this file

What may be worked on next is a decision about this repository's direction, and
the register is the denominator the loop is measured against. A loop that edits
its own register is the defect the register exists to prevent — the same shape
`claudedocs/rfc-nl-to-ir-requirement-closure.md` §4 names for coverage.

Adding, removing or rewording a key is a person's edit.

## ⚠ What this register does NOT answer

It answers *is this proposal in the population*, and nothing else. It does not
know which keys below are already finished, so a proposal re-naming completed
work is admitted. That is still true of the keys in this file.

## ⭐ The population has a SECOND half, and this file is not it

`sce_admits.sh` also admits the **open rows of `docs/SCE_NL_IR_CLOSURE.md`**,
read at call time. Those fourteen rows were measured off the tree rather than
taken from the RFC, so most of them never had a key here — and a loop that
proposed one was refused. Measured 2026-09-15: run 378 closed `no_successor` and
run 383 spent 193 iterations holding a checkpoint nothing would admit, because
five of the nine rows then open had no key in this file.

⛔ **The rows were NOT copied into the key list below, and must not be.** Two
definitions of one population plus a mapping to keep between them is the drift
generator this repository keeps meeting; the ledger stays the single list and
this script reads it.

⭐ That half also pays the hole stated above, for its own rows. This file said
the repair would need *"a per-key predicate against the tree, which is worth
building only once a key is re-proposed in practice"* — and that predicate
exists: `scripts/gates/nl-ir-closure.sh` measures every row against the tree, in
both directions, and the ledger's `Status` column is held to it. So a row that
closes stops being admissible on its own, with nobody striking it out.

⚠ The two halves fail differently on purpose. This file being unreadable is
fatal — it is the primary population. The ledger being unreadable is not: the
script says so on stderr and answers from this file alone, because refusing
every proposal over one missing population would be the worse failure.

⚠⚠ Adding a row to that ledger is the same class of edit as adding a key here:
a person's. The loop may change a row's `Status`, and only to what the gate
measures — a `Status` the tree contradicts is red in either direction.

## Keys

Matching is case-insensitive and the separator is flexible, so `ATOMIC-B`,
`Atomic B` and `atomic b` all name the same key.

    ATOMIC-A   requirement manifest input and the four-way classification
    ATOMIC-B   transition table export carrying the source column
    ATOMIC-C   the acceptance report — done when a violating mutation moves it
    ATOMIC-D   a real consumer: one end-to-end conversion of a real specification
    ATOMIC-E   visualizer overlay for sce:req and sce:provenance, unclaimed in grey
    ATOMIC-F   the acceptance record and the scenario pin
    ATOMIC-G   review artefacts for the remaining Forge kind families
    ATOMIC-H   modality and variants: a question per modality, coverage per variant
    ATOMIC-I   disposition and decomposition: delegated, out_of_scope, parent/child
    ATOMIC-K   extraction properties and a locator that is not PDF-shaped
    ATOMIC-L   a gate refusing a specification's identity in executable code
    SURFACES   register every spec-bearing surface, and gate the registry
    UNIFY      measure, then promote one schema both the mirror and SCE read
    INVENTORY  measure whether the empty inventory layer can carry a modality
    HOLE-1     the manifest copyright guard covers entries, not section titles
    HOLE-2     RequirementId::validate has no caller and would refuse ISO ids
    HOLE-3     sce:req on a transition's own action reaches no reading
    HOLE-4     an opaque requirement id can break the C11 comment form
    ITEM-8     bounded close of the per-code anchor roster

The design behind ATOMIC-A through ATOMIC-I is
`claudedocs/rfc-nl-to-ir-requirement-closure.md` §12. That document is not
tracked, which is why the keys live here: a register the tree cannot read is not
a register.

## Where the HOLE keys came from

They are not planned work. Run 343 (2026-09-12) wrote the first manifest from a
real standard — ISO 13400-2:2019 §12.6 — and the three were what that document
exposed in machinery its own author had believed finished. They are registered
because a hole found once and not written down is found again at the same cost,
and because each of the three is a trap for the NEXT specification rather than a
tidy-up of this one:

- **HOLE-1** is a gap in the copyright guard itself. `ManifestSection.title` is
  unbounded free text, so a requirement sentence pasted there loads silently.
  ⚠ Measured and recorded so it is not re-proposed: a length bound cannot fix it.
  Over 166 REQ boxes the two populations **overlap** — headings reach 81
  characters, and 5 % of requirement first lines are 76 or shorter. A guard wrong
  in both directions is worse than none, because it advertises that the field is
  checked.
- **HOLE-2** is a trap, not an omission. Wiring the validator up as written would
  **reject the real standard's own spelling**: ISO numbers its requirements
  `3.DoIP-152`, and the rule refuses a leading digit. It is also stricter than
  the `NMTOKEN` its own summary claims. The question to settle is which of the
  two it was meant to be — not whether to call it.
- **HOLE-3** is a coverage gap in the walk: an annotation on a transition's own
  action is written and read by nobody. ⚠ Scoped by run 345 before it was
  started: the gap is in the walk and not the parse, codegen already emits what
  the report drops, and 1 of 32 tracked annotations sits there.
- **HOLE-4** was found by repairing HOLE-2 rather than by the standard: once an
  id is genuinely opaque, one can break the C11 backend's comment form. A repair
  that widens what an input may contain owes a look at every backend that
  formats it.

## Why SURFACES and UNIFY were added (2026-09-13)

A complete three-column trace table was found already running for W3C SCXML —
`docs/spec/scxml/.atomic/` holds 197 sections and 199 test bindings, and a
mnemosyne plugin already validates SCE's own `§scxml-n.m` citations against that
section set. A second `verifies-catalog.json` exists for Mesh. None of it was
found while this repository spent a day designing the same thing, because the
search used one vocabulary (`requirement`, `provenance`, `trace`, `coverage` —
**zero** hits in that catalogue) and the catalogue uses another (`section_ids`,
199 hits), while no entry document names any of the eight `.atomic` stores.

- **SURFACES** is the repair for *that*, and it is a registration gate rather
  than a rule because a rule would be forgotten the same way. ⚠ The registry
  already exists — `SCE_WIRE_CONTRACTS.md`, "the single registry of which
  surfaces are pre-release vs stable" — and simply does not name these. Add
  them there; do not build a second registry, which would be the same defect.
- **UNIFY** is the SSOT repair. ⛔ It is NOT "compare the two vocabularies and
  align them": two definitions plus a mapping is a drift generator. One model,
  many specifications — the existing one is the better base (tagged-union
  locator that survives repagination, URL source, `text` + `text_sha256`, which
  is also the answer to the copyright split this tree invented separately).
  ⚠⚠ UNIFY has a measurement prerequisite: can the existing section model
  express `shall_not` and `shall_within`? `coverage_expectation` carries only
  `informational` / `out_of_scope_here` / unset. Do not promote a schema before
  that is answered.

## Why INVENTORY was added (2026-09-13), and what UNIFY measured first

UNIFY's measurement came back and it settled the shape of the problem:

- **No existing section field is a modality**, and none can be overloaded into
  one. The three candidates each answer something else — `decision_status` is
  lifecycle (`active` ×197, one value in the whole corpus),
  `coverage_expectation` asks whether a section needs code, and
  `verification_expectation` asks how it is checked. ⚠ `mnemosyne.toml` already
  sets `severity_coverage = reject`, so a second meaning in that slot would make
  `MisclassifiedCoverage` fire on correctly-classified prohibitions.
- **The negative modality is already in the corpus with nowhere to live**: across
  192 stored excerpts, `must not` 32, `should not` 4, `shall not` 1. So the
  prohibition problem is not an ISO peculiarity — the W3C corpus has the same
  shape.
- **⭐ The decisive one is granularity.** 30 sections carry two distinct
  modalities, 12 carry three, 4 carry four. A section-level `modality` field
  would have to collapse `must` and `must not` into one cell for 46 sections.
  ⇒ **Modality belongs to a requirement, not to a section.**

And a requirement layer already exists in the format, **empty**:
`InventoryEntry` — the guide's own words are *"test cases / requirement ids …
internal requirement ids"* — with `{ id, status: active|reserved|deprecated,
section, source, reason }`, a CLI to register entries, and
`set_equality_validator` already checking existence and status at cite time. The
store reports `inventory_entries: 0`.

⚠⚠ So the right move is **not** to widen anything and **not** to stand a second
requirement layer beside it — that would repeat, as a design, the exact mistake
this register's SURFACES entry was added to prevent. It is to measure whether
that empty layer can carry `modality` and `disposition`, and only then decide.

⚠ INVENTORY is a MEASUREMENT checkpoint. Changing the mnemosyne schema is out of
its scope: that is another repository and another owner's format.

## ⭐ How ATOMIC-C is judged, and why the key says it that way

The acceptance report is what a person's acceptance rests on, so "it emits three
blocks" is not a completion test. The report is a **detector**, and this
repository already owns the way detectors are tested — `scripts/mutate`'s own
header: *"A test that passes proves nothing about whether it could fail. The way
to find out is to break the code it guards and watch it turn red."*

```
take an accepted (spec, SCXML) pair
inject a mutation that VIOLATES a requirement    after 3s -> after 5s
regenerate the report
  byte-identical -> the differing value is NOT ON THE PAGE; no reviewer,
                    however careful, could catch it. The report does not
                    support that requirement.
  changed        -> at least it is visible
```

⭐ This also settles what block A's diagram fragment must contain — **not a list
someone writes down, but whatever the mutations force**: transition event, guard,
delay, target state, entry and exit actions, each mutated, each required to move
the block. A fragment that never drew the delay then goes red on its own.

⚠ It does not judge whether a person *noticed* — the change could be small type
three screens down. That half is irreducible and stays a human judgement. The
point is to mechanise the half that can be and stop counting the other half as
covered.

⚠⚠ Two questions belong to C and must be answered **before** it is launched: what
draws the diagram fragment (C itself, the table alone, or ATOMIC-E first), and
whether a per-requirement fragment is the right unit. The mutation test
constrains any answer — whatever draws it must move under all of them.

## The general-purpose rule ATOMIC-L enforces

Owner's instruction, 2026-09-12: *"범용적으로 만들어야 해, 특정 스펙에 종속되면
안 돼"*.

> No production code may branch on the identity of a specification. It branches
> on declared properties, which a source SCE has never seen can supply too.

Measured the same day: standard names outside comments in `sce-build/src/**/*.rs`
number **4**, all of them fixture strings inside tests, and behavioural
dependencies number **0**. The gate is therefore cheap now and expensive later,
which is the whole argument for sequencing it early.

⚠ Its discriminator is *executable position*, not the word — a comment citing the
document that justifies a rule is exactly what should survive. ⚠⚠ And it needs an
allowlist for a protocol SCE actually implements: Mesh names SOME/IP the way it
may name TCP, because it emits that wire format. A standard SCE *transports* is
not a standard SCE *depends on for meaning*.

⚠ HOLE-1 and HOLE-2 are worth paying before the next specification is admitted:
the first lets copyrighted text in quietly, and the second turns a real
standard's ids into errors the moment anyone completes the obvious wiring.
