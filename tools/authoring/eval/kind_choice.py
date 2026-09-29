"""Measure whether a client reaching SCE only through MCP chooses the kind
the specification determines.

Why a measurement and not a promise: the kind is chosen by a model reading
prose, and nothing in the product can judge prose. What the product does is
carry the material the choice rests on (`scxml_kinds`) and report the reading
it made (`document_kind` on the manifest). Whether that is enough for a client
with none of this repository's context is a number, and this is where the
number comes from.

Each case is a prose specification. The client gets it in an empty directory
with the SCE MCP server and nothing else, and the request an owner would type.
Scoring does not read what the client SAID: it puts the document it wrote
through `validate_scxml` and compares the manifest's `document_kind` with the
case. A case whose text leaves the kind open is right when no document was
written — the client asked — or when the draft marks its kind open on its
`<sce:kind-basis>`, which is the same question put where the owner reads it.

    python3 tools/authoring/eval/kind_choice.py --client claude-restricted \\
        --out /tmp/kind-eval [--only id,id] [--model NAME]

⚠ How isolated a run is depends on the client, and the report says which it
was. `claude-bare` loads no CLAUDE.md, memory or hooks, and needs
ANTHROPIC_API_KEY. `claude-restricted` runs in the empty directory with no
repository in reach, but a user-level CLAUDE.md is still read.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
AUTHORING = HERE.parent
REPO = AUTHORING.parent.parent
sys.path.insert(0, str(AUTHORING))

from sce_author.verify import (kind_catalog, unresolved_markers,  # noqa: E402
                               validate_scxml)

CASES = HERE / "kind_choice_cases.json"
LAUNCHER = REPO / "scripts" / "sce_author_mcp.sh"

# The request as the README tells an owner to phrase it, plus where the files
# are. Nothing here hints at the kinds: the guidance under test is the
# server's own.
REQUEST = (
    "The specification is in spec.md in this directory. Choose the SCE "
    "document kind that fits this behavior, make a pseudocode draft, check it "
    "with SCE, and show anything the specification leaves undecided. Save the "
    "draft as draft.scxml in this directory."
)

_CLAUDE_COMMON = [
    "--strict-mcp-config", "--mcp-config", "{mcp_config}",
    "--permission-mode", "acceptEdits",
    "--allowedTools", "Read,Write,mcp__sce-author",
    "--no-session-persistence",
]

CLIENTS = {
    "claude-bare": {
        "argv": ["claude", "-p", "--bare", *_CLAUDE_COMMON, "{prompt}"],
        "isolation": "no CLAUDE.md, memory or hooks (--bare)",
    },
    "claude-restricted": {
        "argv": ["claude", "-p", "--restricted", *_CLAUDE_COMMON, "{prompt}"],
        "isolation": "empty directory, no repository in reach; a user-level "
                     "CLAUDE.md is still read",
    },
}


class CaseError(ValueError):
    """The corpus is not in the shape the scoring relies on."""


def load_cases(path: pathlib.Path, kinds: set[str]) -> list[dict]:
    """The cases, refused whole when one is malformed.

    `kinds` is the product's own set, so a case naming a kind the product
    does not have fails here rather than scoring every client wrong.
    """
    cases = json.loads(path.read_text(encoding="utf-8"))["cases"]
    seen = set()
    for case in cases:
        ident = case.get("id")
        if not ident or ident in seen:
            raise CaseError(f"case id {ident!r} is missing or repeated")
        seen.add(ident)
        if not case.get("prose"):
            raise CaseError(f"{ident}: no prose")
        if ("kind" in case) == ("undetermined" in case):
            raise CaseError(f"{ident}: exactly one of kind / undetermined")
        named = [case["kind"]] if "kind" in case else case["undetermined"]
        if "undetermined" in case and len(named) < 2:
            raise CaseError(f"{ident}: an undetermined case names two candidates or more")
        unknown = sorted(set(named) - kinds)
        if unknown:
            raise CaseError(f"{ident}: not kinds the product has: {unknown}")
    return cases


def score(case: dict, draft: pathlib.Path, codegen: pathlib.Path | None = None) -> dict:
    """What the product read in the client's document, against the case."""
    result = {"id": case["id"], "expected": case.get("kind"),
              "candidates": case.get("undetermined")}
    if not draft.is_file():
        result.update(chosen=None, declared=None, verdict="absent")
        result["outcome"] = "asked" if "undetermined" in case else "no_document"
        return result
    report, refusal = validate_scxml(draft, codegen)
    answer = json.loads(report or refusal)
    reading = (answer.get("manifest") or {}).get("document_kind") or {}
    open_kind = _kind_left_open(draft, codegen)
    result.update(chosen=reading.get("name"), declared=reading.get("declared"),
                  basis_recorded=reading.get("basis_recorded"),
                  kind_left_open=open_kind, verdict=answer["verdict"])
    if "undetermined" in case:
        # A provisional draft whose basis marks the kind open is the
        # question asked in the document, where the owner and the strict
        # build both see it; one that does not is a choice made silently.
        result["outcome"] = "left_open" if open_kind else "decided"
    elif open_kind:
        result["outcome"] = "left_open"
    elif answer["verdict"] != "accepted":
        result["outcome"] = "refused"
    elif reading.get("name") != case["kind"]:
        result["outcome"] = "wrong_kind"
    elif not reading.get("declared"):
        result["outcome"] = "correct_by_default"
    else:
        result["outcome"] = "correct"
    return result


def _kind_left_open(draft: pathlib.Path, codegen: pathlib.Path | None) -> bool:
    """Whether the document marks its kind as undecided: an open marker on
    its `<sce:kind-basis>`, as `sce-codegen unresolved` lists it."""
    report, refusal = unresolved_markers(draft, codegen)
    if refusal:
        return False
    return any(m.get("node_path") == "<kind-basis>" and m.get("kind") == "unresolved"
               for m in json.loads(report).get("markers") or [])


def summarise(results: list[dict]) -> dict:
    determined = [r for r in results if r["expected"] is not None]
    undetermined = [r for r in results if r["expected"] is None]
    count = lambda rows, outcome: sum(1 for r in rows if r["outcome"] == outcome)
    return {
        "determined": {"total": len(determined),
                       "correct": count(determined, "correct"),
                       "correct_by_default": count(determined, "correct_by_default"),
                       "wrong_kind": count(determined, "wrong_kind"),
                       "refused": count(determined, "refused"),
                       "left_open": count(determined, "left_open"),
                       "no_document": count(determined, "no_document")},
        "undetermined": {"total": len(undetermined),
                         "asked": count(undetermined, "asked"),
                         "left_open": count(undetermined, "left_open"),
                         "decided": count(undetermined, "decided")},
        "basis_recorded": sum(1 for r in results if r.get("basis_recorded")),
    }


def _argv(template: list[str], prompt: str, mcp_config: pathlib.Path) -> list[str]:
    return [part.replace("{prompt}", prompt).replace("{mcp_config}", str(mcp_config))
            for part in template]


def run_case(case: dict, client: list[str], out: pathlib.Path, timeout: int,
             model: str | None, codegen: pathlib.Path | None = None) -> dict:
    """One case in its own empty directory, then its score."""
    work = out / case["id"]
    work.mkdir(parents=True, exist_ok=False)
    (work / "spec.md").write_text(case["prose"] + "\n", encoding="utf-8")
    mcp_config = out / "mcp.json"
    argv = _argv(client, REQUEST, mcp_config)
    if model:
        argv[1:1] = ["--model", model]
    started = time.monotonic()
    try:
        run = subprocess.run(argv, cwd=work, capture_output=True, text=True,
                             timeout=timeout)
        transcript, status = run.stdout + run.stderr, run.returncode
    except subprocess.TimeoutExpired as expired:
        transcript, status = f"timed out after {timeout}s\n{expired.stdout or ''}", None
    (work / "transcript.txt").write_text(transcript, encoding="utf-8")
    result = score(case, work / "draft.scxml", codegen)
    result.update(client_status=status, seconds=round(time.monotonic() - started))
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--client", default="claude-restricted",
                        help=f"one of {sorted(CLIENTS)}")
    parser.add_argument("--out", required=True, type=pathlib.Path,
                        help="an empty directory for the runs and the report")
    parser.add_argument("--only", help="comma-separated case ids")
    parser.add_argument("--model")
    parser.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args(argv)

    if args.client not in CLIENTS:
        parser.error(f"--client must be one of {sorted(CLIENTS)}")
    if args.out.exists() and any(args.out.iterdir()):
        parser.error(f"{args.out} is not empty; a report mixed with an old run says nothing")
    args.out.mkdir(parents=True, exist_ok=True)

    cases = load_cases(CASES, product_kinds())
    if args.only:
        wanted = set(args.only.split(","))
        cases = [c for c in cases if c["id"] in wanted]
    (args.out / "mcp.json").write_text(json.dumps(
        {"mcpServers": {"sce-author": {"command": str(LAUNCHER)}}}, indent=2) + "\n",
        encoding="utf-8")

    client = CLIENTS[args.client]
    results = []
    for case in cases:
        results.append(run_case(case, client["argv"], args.out, args.timeout, args.model))
        print(json.dumps(results[-1]), flush=True)
    report = {"client": args.client, "isolation": client["isolation"],
              "model": args.model, "summary": summarise(results), "results": results}
    (args.out / "report.json").write_text(json.dumps(report, indent=2) + "\n",
                                          encoding="utf-8")
    print(json.dumps(report["summary"], indent=2))
    return 0


def product_kinds(codegen: pathlib.Path | None = None) -> set[str]:
    """The product's kind names, asked of the generator this run will use."""
    report, refusal = kind_catalog(codegen=codegen)
    if refusal:
        raise SystemExit(f"the product's kind catalog could not be read:\n{refusal}")
    return {k["name"] for k in json.loads(report)["catalog"]["kinds"]}


if __name__ == "__main__":
    sys.exit(main())
