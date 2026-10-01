"""Measure how far independent drafts of one specification part, when a
client reaching SCE only through MCP writes each of them afresh.

Why a measurement: the model runs in the owner's AI client, which SCE does
not control, so nothing in the product can make two samples equal. What the
product can do is say where they part -- `sce_author.compare` -- and this is
where the figures for that come from.

Each case is drafted `--reps` times, every draft in its own empty directory
through `kind_choice.py`'s client, request and isolation, unchanged. The
drafts of each case are then compared at every level `compare` knows, and the
report gives, per case, how many classes the drafts fall into at each level
(1: all agree), the open questions each draft marked, and the behaviour
verdict with its bound, renamings and witnesses.

    python3 tools/authoring/eval/reproducibility.py --out /tmp/repro \\
        [--reps 5] [--only id,id] [--model claude-sonnet-5-5] [--effort high] \\
        [--profile owner-profile.json] [--requirements fixed-lists/]

`--profile` runs the same cases under an owner's authoring profile: the file is
copied beside each specification, the request says where it is, and every draft
is judged under it. The report then says how many drafts the profile refused
and by which codes. Run a case with and without it, over the same `--reps`, and
the two tables are the measurement: whether a profile moves the classes.

`--requirements` gives every draft of a case the SAME owner's requirement list,
`<case id>.manifest_text.json` and `<case id>.sidecar_text.json` in the named
directory, made once by `scxml_requirement_set` and read against the prose. Each
draft is measured against that list by the product's own `requirements` records,
so the report says, per case, how many drafts left no id dangling, how many left
none missing, and for each requirement how many distinct parts of the design the
drafts used to carry it. Without it each draft is counted against whatever ids
its own author made up.

⚠ The model defaults to Sonnet because that is the model the owner fixed for
drafting (2026-09-29). The figures are about the product together with that
client and model, never about the product alone, and the report says which.

⚠ A case whose behaviour comes back `not judged` has not been shown to agree.
Read it as a gap in what could be driven -- typically inputs that carry data
nobody sent -- not as a pass.
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))

import kind_choice  # noqa: E402
from sce_author.compare import compare  # noqa: E402

CASES = HERE / "reproducibility_cases.json"
DEFAULT_MODEL = "claude-sonnet-5-5"
LEVELS = ("bytes", "canonical", "logic", "table", "page", "open", "vocabulary")


def load_cases(path: pathlib.Path = CASES) -> list[dict]:
    """The corpus: the kind-choice cases it names, then its own."""
    spec = json.loads(path.read_text(encoding="utf-8"))
    by_id = {c["id"]: c for c in json.loads(kind_choice.CASES.read_text(encoding="utf-8"))["cases"]}
    missing = [i for i in spec["from_kind_choice"] if i not in by_id]
    if missing:
        raise SystemExit(f"{path.name} names kind-choice cases that do not exist: {missing}")
    cases = [by_id[i] for i in spec["from_kind_choice"]] + spec["cases"]
    ids = [c["id"] for c in cases]
    if len(set(ids)) != len(ids):
        raise SystemExit(f"{path.name}: a case id is repeated")
    for case in cases:
        if "kind" not in case:
            raise SystemExit(f"{case['id']}: a reproducibility case names the kind it determines")
    return cases


def table(results: list[dict]) -> str:
    """One row per case: the number of classes at each level, and, under a
    profile, how many of its drafts the profile held (`held`, of the drafts
    written) beside the behaviour column."""
    under_profile = any("held" in r for r in results)
    head = ["case", "kind", *LEVELS, "behaviour", *(["held"] if under_profile else [])]
    rows = [head]
    for r in results:
        levels = r["comparison"]["levels"] if r.get("comparison") else {}
        behaviour = (r.get("comparison") or {}).get("behaviour", {})
        verdict = behaviour.get("verdict", "-")
        if verdict == "judged":
            verdict = str(len(behaviour["classes"]))
        rows.append([r["id"], str(len(r["kinds"])),
                     *[str(len(levels[level])) if level in levels else "-" for level in LEVELS],
                     verdict, *([f"{r['held']}/{r['drafts']}"] if under_profile else [])])
    widths = [max(len(row[i]) for row in rows) for i in range(len(head))]
    return "\n".join("  ".join(cell.ljust(w) for cell, w in zip(row, widths)) for row in rows)


def requirement_agreement(drafts: dict[str, dict | None]) -> dict:
    """How the drafts of one case stand against the owner's one list.

    `drafts` maps a draft label to `kind_choice.requirement_outcome`. A draft
    the product could not measure (`refused`) is named and left out of every
    count, so an unreadable draft is not one that left nothing dangling.

    `distinct_node_paths` is per requirement the number of different parts of
    the design the drafts that implemented it used to carry it. ⚠ A node path
    names states by their ids, so drafts that name a state differently differ
    here without differing in what they do: it is a ceiling on disagreement and
    not a measure of it, and `compare` is the judge of behaviour.
    """
    measured = {label: found for label, found in drafts.items()
                if found and "refused" not in found}
    unmeasured = sorted(label for label in drafts if label not in measured)
    outcome_of = lambda found, outcome: found["ids"].get(outcome, [])
    carried: dict[str, set[tuple[str, ...]]] = {}
    for found in measured.values():
        for ident, paths in found["node_paths"].items():
            carried.setdefault(ident, set()).add(tuple(paths))
    return {
        "measured": len(measured),
        "unmeasured": unmeasured,
        "no_dangling": sum(1 for f in measured.values() if not outcome_of(f, "dangling")),
        "none_missing": sum(1 for f in measured.values() if not outcome_of(f, "missing")),
        "dangling": {label: outcome_of(f, "dangling") for label, f in measured.items()
                     if outcome_of(f, "dangling")},
        "missing": {label: outcome_of(f, "missing") for label, f in measured.items()
                    if outcome_of(f, "missing")},
        "implemented_by": {ident: sum(1 for f in measured.values()
                                      if ident in f["node_paths"])
                           for ident in sorted(carried, key=_id_order)},
        "distinct_node_paths": {ident: len(paths)
                                for ident, paths in sorted(carried.items(),
                                                           key=lambda item: _id_order(item[0]))},
    }


def _id_order(ident: str) -> tuple[str, int]:
    """`R2` before `R10`: the owner's ids in the order they were numbered."""
    digits = "".join(ch for ch in ident if ch.isdigit())
    return (ident.rstrip("0123456789"), int(digits) if digits else 0)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--client", default="claude-restricted",
                        help=f"one of {sorted(kind_choice.CLIENTS)}")
    parser.add_argument("--out", required=True, type=pathlib.Path,
                        help="an empty directory for the drafts and the report")
    parser.add_argument("--reps", type=int, default=5, help="drafts per case")
    parser.add_argument("--only", help="comma-separated case ids")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--effort", default="high",
                        help="the client's effort level, fixed so repetitions differ by sampling only")
    parser.add_argument("--timeout", type=int, default=900)
    parser.add_argument("--profile", type=pathlib.Path,
                        help="the owner's authoring profile every case is drafted under")
    parser.add_argument("--requirements", type=pathlib.Path,
                        help="a directory holding each case's fixed requirement list")
    args = parser.parse_args(argv)

    if args.client not in kind_choice.CLIENTS:
        parser.error(f"--client must be one of {sorted(kind_choice.CLIENTS)}")
    if args.reps < 2:
        parser.error("--reps must be 2 or more: one draft is not a comparison")
    if args.profile is not None and not args.profile.is_file():
        parser.error(f"--profile {args.profile}: no such file")
    if args.out.exists() and any(args.out.iterdir()):
        parser.error(f"{args.out} is not empty; a report mixed with an old run says nothing")
    args.out.mkdir(parents=True, exist_ok=True)

    cases = load_cases()
    if args.only:
        wanted = set(args.only.split(","))
        cases = [c for c in cases if c["id"] in wanted]
    if args.requirements is not None:
        lacking = [f"{c['id']}.{part}.json" for c in cases
                   for part in ("manifest_text", "sidecar_text")
                   if not (args.requirements / f"{c['id']}.{part}.json").is_file()]
        if lacking:
            parser.error(f"--requirements {args.requirements}: missing {lacking}")
    client = kind_choice.CLIENTS[args.client]
    argv_template = [client["argv"][0], "--effort", args.effort, *client["argv"][1:]]

    def one_rep(rep: int) -> list[dict]:
        out = args.out / f"rep{rep}"
        out.mkdir()
        (out / "mcp.json").write_text(json.dumps(
            {"mcpServers": {"sce-author": {"command": str(kind_choice.LAUNCHER)}}},
            indent=2) + "\n", encoding="utf-8")
        rows = []
        for case in cases:
            row = kind_choice.run_case(case, argv_template, out, args.timeout, args.model,
                                       profile=args.profile,
                                       requirements=args.requirements)
            row["rep"] = rep
            rows.append(row)
            print(json.dumps(row), flush=True)
        return rows

    # Repetitions run in parallel and each case within one runs in turn, so a
    # slow case holds up only its own repetition.
    with cf.ThreadPoolExecutor(max_workers=args.reps) as pool:
        runs = [row for rows in pool.map(one_rep, range(1, args.reps + 1)) for row in rows]

    results = []
    for case in cases:
        mine = [r for r in runs if r["id"] == case["id"]]
        drafts = {f"rep{r['rep']}": args.out / f"rep{r['rep']}" / case["id"] / "draft.scxml"
                  for r in mine}
        present = {label: path for label, path in drafts.items() if path.is_file()}
        entry = {"id": case["id"], "expected_kind": case.get("kind"),
                 "kinds": sorted({str(r.get("chosen")) for r in mine}),
                 "outcomes": sorted(r["outcome"] for r in mine),
                 "drafts": len(present)}
        if args.profile is not None:
            # The codes of the profile's own findings, draft by draft, and how
            # many drafts the profile had nothing to say about. A draft that
            # was not written is not held: it is not counted among `drafts`.
            entry["profile_findings"] = {f"rep{r['rep']}": r.get("profile_findings", [])
                                         for r in mine if f"rep{r['rep']}" in present}
            entry["held"] = sum(1 for found in entry["profile_findings"].values() if not found)
        if args.requirements is not None:
            entry["requirements"] = requirement_agreement(
                {f"rep{r['rep']}": r.get("requirements") for r in mine
                 if f"rep{r['rep']}" in present})
        if len(present) >= 2:
            entry["comparison"] = compare(list(present.values()), labels=list(present))
        else:
            entry["comparison"] = None
            entry["why"] = "fewer than two drafts were written"
        results.append(entry)

    report = {"client": args.client, "isolation": client["isolation"],
              "model": args.model, "effort": args.effort, "reps": args.reps,
              "request": kind_choice.REQUEST, "results": results}
    if args.profile is not None:
        report["profile"] = {"file": args.profile.name,
                             "sha256": hashlib.sha256(args.profile.read_bytes()).hexdigest(),
                             "request": kind_choice.PROFILE_REQUEST.strip()}
    if args.requirements is not None:
        report["requirements"] = {
            "directory": args.requirements.name,
            "sha256": {name.name: hashlib.sha256(name.read_bytes()).hexdigest()
                       for name in sorted(args.requirements.glob("*_text.json"))},
            "request": kind_choice.MANIFEST_REQUEST.strip()}
    (args.out / "report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False, default=str) + "\n", encoding="utf-8")
    print(table(results))
    return 0


if __name__ == "__main__":
    sys.exit(main())
