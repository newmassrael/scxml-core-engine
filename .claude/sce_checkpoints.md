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
know which keys are already finished, so a proposal re-naming completed work is
admitted. That is deliberate for now: deriving doneness would either need the
loop to mark this file (forbidden above) or a per-key predicate against the tree,
which is worth building only once a key is re-proposed in practice.

## Keys

Matching is case-insensitive and the separator is flexible, so `ATOMIC-B`,
`Atomic B` and `atomic b` all name the same key.

    ATOMIC-A   requirement manifest input and the four-way classification
    ATOMIC-B   transition table export carrying the source column
    ATOMIC-C   the acceptance report that folds the manifest review into one sitting
    ATOMIC-D   a real consumer: one end-to-end conversion of a real specification
    ATOMIC-E   visualizer overlay for sce:req and sce:provenance, unclaimed in grey
    ATOMIC-F   the acceptance record and the scenario pin
    ATOMIC-G   review artefacts for the remaining Forge kind families
    ATOMIC-H   modality and variants: a question per modality, coverage per variant
    ATOMIC-I   disposition and decomposition: delegated, out_of_scope, parent/child
    HOLE-1     the manifest copyright guard covers entries, not section titles
    HOLE-2     RequirementId::validate has no caller and would refuse ISO ids
    HOLE-3     sce:req on a transition's own action reaches no reading
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
  action is written and read by nobody.

⚠ HOLE-1 and HOLE-2 are worth paying before the next specification is admitted:
the first lets copyrighted text in quietly, and the second turns a real
standard's ids into errors the moment anyone completes the obvious wiring.
