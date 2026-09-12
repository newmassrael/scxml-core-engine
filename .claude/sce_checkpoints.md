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
    ITEM-8     bounded close of the per-code anchor roster

The design behind ATOMIC-A through ATOMIC-G is
`claudedocs/rfc-nl-to-ir-requirement-closure.md` §12. That document is not
tracked, which is why the keys live here: a register the tree cannot read is not
a register.
