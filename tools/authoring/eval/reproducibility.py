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
        [--reps 5] [--only id,id] [--model claude-sonnet-5-5] [--effort high]

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
    """One row per case: the number of classes at each level."""
    head = ["case", "kind", *LEVELS, "behaviour"]
    rows = [head]
    for r in results:
        levels = r["comparison"]["levels"] if r.get("comparison") else {}
        behaviour = (r.get("comparison") or {}).get("behaviour", {})
        verdict = behaviour.get("verdict", "-")
        if verdict == "judged":
            verdict = str(len(behaviour["classes"]))
        rows.append([r["id"], str(len(r["kinds"])),
                     *[str(len(levels[level])) if level in levels else "-" for level in LEVELS],
                     verdict])
    widths = [max(len(row[i]) for row in rows) for i in range(len(head))]
    return "\n".join("  ".join(cell.ljust(w) for cell, w in zip(row, widths)) for row in rows)


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
    args = parser.parse_args(argv)

    if args.client not in kind_choice.CLIENTS:
        parser.error(f"--client must be one of {sorted(kind_choice.CLIENTS)}")
    if args.reps < 2:
        parser.error("--reps must be 2 or more: one draft is not a comparison")
    if args.out.exists() and any(args.out.iterdir()):
        parser.error(f"{args.out} is not empty; a report mixed with an old run says nothing")
    args.out.mkdir(parents=True, exist_ok=True)

    cases = load_cases()
    if args.only:
        wanted = set(args.only.split(","))
        cases = [c for c in cases if c["id"] in wanted]
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
            row = kind_choice.run_case(case, argv_template, out, args.timeout, args.model)
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
        if len(present) >= 2:
            entry["comparison"] = compare(list(present.values()), labels=list(present))
        else:
            entry["comparison"] = None
            entry["why"] = "fewer than two drafts were written"
        results.append(entry)

    report = {"client": args.client, "isolation": client["isolation"],
              "model": args.model, "effort": args.effort, "reps": args.reps,
              "request": kind_choice.REQUEST, "results": results}
    (args.out / "report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False, default=str) + "\n", encoding="utf-8")
    print(table(results))
    return 0


if __name__ == "__main__":
    sys.exit(main())
