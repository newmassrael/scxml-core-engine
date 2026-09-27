"""Cases for sce:std/sync/changes_apply — the one standard document here
whose expected values are NOT a reference's answers, because no reference
measured applies a batch of changes correctly.

The expected value is the document's definition: each item's last row
decides, else the state. What this script proves about each case is the
property the document exists for — the answer does not depend on how the
log was batched. For every case it applies the log whole and cut at every
point (apply the first part, then the rest to that answer) and refuses to
print a case where any two differ.

The two counterexample shapes come from reading two clients' source (not
from running them; neither applies a batch outside its app):
- Tutanota, src/common/api/worker/rest/DefaultEntityRestCache.ts
  updateCacheWithMissedEntityUpdates (764-790): per type, deleteMultiple of
  every DELETE, then putMultiple of every CREATE and UPDATE — so
  [create a, delete a] in one batch leaves a present.
- Proton Calendar iOS, CalendarModelEventProcessor.swift process(events:)
  (107-128): creates and updates stored first, deletes after — so
  [delete a, create a] in one response leaves a absent.
Each such case's note names what that client's order would leave.

Usage (see README.md):
    python3 tools/reference-oracles/batch_order.py > cases.json
"""

import json
import random


def apply(state: list[dict], log: list[dict]) -> list[dict]:
    held = {s["item"]: s["token"] for s in state}
    for row in log:
        if row["op"] == 3:
            held.pop(row["item"], None)
        else:
            held[row["item"]] = row["token"]
    return [{"item": i, "token": t} for i, t in sorted(held.items())]


def deletes_first(state: list[dict], log: list[dict]) -> list[dict]:
    """Tutanota's order within one batch (read, not run)."""
    return apply(apply(state, [r for r in log if r["op"] == 3]), [r for r in log if r["op"] != 3])


def deletes_last(state: list[dict], log: list[dict]) -> list[dict]:
    """Proton Calendar iOS's order within one response (read, not run)."""
    return apply(apply(state, [r for r in log if r["op"] != 3]), [r for r in log if r["op"] == 3])


def row(item: int, op: int, token: int) -> dict:
    return {"item": item, "op": op, "token": token}


def case(state: list[dict], log: list[dict], note: str) -> dict:
    whole = apply(state, log)
    for cut in range(len(log) + 1):
        split = apply(apply(state, log[:cut]), log[cut:])
        if split != whole:
            raise SystemExit(f"{note}: cut at {cut} gives {split}, whole gives {whole}")
    return {"args": [state, log], "expected": whole, "note": note}


def ids(items: list[dict]) -> str:
    return "{" + ", ".join(str(s["item"]) for s in items) + "}"


def main() -> None:
    held = [{"item": 1, "token": 5}, {"item": 2, "token": 5}]
    cases = [
        case([], [], "nothing held, nothing to apply"),
        case(held, [], "an empty batch leaves the state"),
        case(held, [row(3, 1, 6), row(1, 3, 7), row(3, 2, 8)], "an add, a delete and a modify of an added item"),
        case([], [row(9, 1, 1), row(4, 1, 2), row(7, 1, 3)], "the answer is in item order, not log order"),
    ]
    shapes = [
        ([row(4, 1, 6), row(4, 3, 7)], "a create then a delete in one batch", deletes_first, "Tutanota's deletes-first order"),
        ([row(2, 2, 6), row(2, 3, 7)], "a modify then a delete in one batch", deletes_first, "Tutanota's deletes-first order"),
        ([row(4, 3, 6), row(4, 1, 7)], "a delete then a create in one batch", deletes_last, "Proton Calendar iOS's deletes-last order"),
        ([row(2, 3, 6), row(2, 2, 7)], "a delete then a modify in one batch", deletes_last, "Proton Calendar iOS's deletes-last order"),
    ]
    for log, note, order, who in shapes:
        got = order(held, log)
        whole = apply(held, log)
        if got == whole:
            raise SystemExit(f"{note}: {who} agrees with the definition; not a counterexample")
        cases.append(case(held, log, f"counterexample: {note} — {who} (read from source, not run) would leave {ids(got)}"))
    rng = random.Random(0xba7c40de)
    for k in range(6):
        state = [{"item": i, "token": rng.randrange(1, 5)} for i in sorted(rng.sample(range(1, 12), rng.randrange(0, 5)))]
        token, log = 5, []
        for _ in range(rng.randrange(3, 9)):
            token += rng.randrange(0, 2)
            log.append(row(rng.randrange(1, 12), rng.choice((1, 2, 3)), token))
        held_n = f"{len(state)} held item{'' if len(state) == 1 else 's'}"
        cases.append(case(state, log, f"a pseudo-random batch of {len(log)} rows on {held_n}"))
        # The same batch cut in two, the second part applied to the answer
        # to the first: the generated code answers it as it answered the
        # whole, which is the property the harness then checks on every
        # backend rather than only here.
        cut = len(log) // 2
        rest = case(apply(state, log[:cut]), log[cut:], f"the batch before cut after row {cut}: its second part applied to the first part's answer")
        if rest["expected"] != apply(state, log):
            raise SystemExit("a cut case does not end where the whole batch does")
        cases.append(rest)
    fails = [
        {"args": [[{"item": 2, "token": 1}, {"item": 1, "token": 1}], []], "fails": "precondition", "note": "a state out of item order"},
        {"args": [[], [row(1, 1, 5), row(2, 1, 4)]], "fails": "precondition", "note": "a log out of token order"},
        {"args": [[], [row(1, 4, 5)]], "fails": "precondition", "note": "an op that is none of add, modify, delete"},
    ]
    print(json.dumps({"changes_apply": cases + fails}, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
