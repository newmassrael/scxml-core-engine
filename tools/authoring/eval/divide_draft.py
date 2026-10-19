"""Measure how well a model draws the division of a document that holds several specs (ADR 0040).

Why a measurement and not a promise: whether a part of a document belongs with one function or another is
read from prose by a model, and nothing in the product can judge prose. What the product does is hold the
draft to the owner (it is suggestions, and the owner confirms each part) and make it checkable. Whether the
suggestions are worth showing at all is a number, and this is where the number comes from.

A case is made of public documents that are about different things. Their `##` sections are cut into chunks,
the chunks are laid in one document (in their order, or shuffled with a seed), and the document is read by the
same reader an owner's would be. Because every chunk is known to come from one source, every part of the
outline has a right answer. A model is then asked, through `prepare-division`'s prompt, in two ways:

  named   the owner has one spec for each source, titled as the source is, and the model places parts
  open    the owner has one spec and nothing else, and the model proposes the others by title

and the answer is scored against the truth: how many parts it placed (coverage), how many of those it placed
rightly (precision) and, for the open way, how pure the groups it made are and how many groups a source was
split across. Every figure is also given in paragraphs, since a part that holds a hundred paragraphs counts
for more than one that holds one. The share of the biggest source is the number to beat: a model that puts
every part with the biggest source is that right, and says nothing.

    python3 tools/authoring/eval/divide_draft.py --endpoint http://127.0.0.1:18081/v1 \\
        --out /tmp/divide-eval [--only id,id] [--seed N] [--model NAME]

A server that cannot be reached is not a model that answered badly: a case whose call failed on the link is
`link-failed` and is left out of the figures, as a queue that records a dead tunnel as a failed component
records nothing about the component. A server that answered with an error (a context that was too long) is a
result and is scored as an answer that could not be read.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import random
import socket
import sys
import tempfile
import time
import urllib.error
import urllib.request
from collections import Counter, defaultdict

HERE = pathlib.Path(__file__).resolve().parent
AUTHORING = HERE.parent
REPO = AUTHORING.parent.parent
sys.path.insert(0, str(AUTHORING))

from sce_author.divide_by_model import Offered, prepare, read_answer  # noqa: E402
from sce_author.errors import IngestError  # noqa: E402
from sce_author.extractors import DEFAULT as EXTRACTORS  # noqa: E402
from sce_author.scope import outline  # noqa: E402

# What the documents are about, as an owner would name the spec of each. Public documents of this repository, on
# different subjects (and, in `related`, on one subject cut three ways, which is the hard case).
SOURCES = {
    "transition-table": ("docs/SCE_TRANSITION_TABLE.md", "Transition table of a statechart"),
    "shared-trust": ("docs/SCE_SHARED_LIST_TRUST.md", "Trust of a shared list"),
    "shared-relay": ("docs/SCE_SHARED_LIST_RELAY.md", "Relay of a shared list"),
    "shared-mls": ("docs/SCE_SHARED_LIST_MLS.md", "Group encryption of a shared list"),
    "script-engine": ("docs/SCE_SCRIPT_ENGINE_PROGRAMME.md", "Script engine programme"),
    "nl-ir": ("docs/SCE_NL_IR_CLOSURE.md", "Natural language to IR closure"),
    "mesh-keying": ("docs/SCE_MESH_KEYING_AUDIT.md", "Mesh key expression audit"),
    "host-features": ("docs/SCE_HOST_FEATURES.md", "Host features"),
    "determinism": ("docs/SCE_CODEGEN_DETERMINISM.md", "Code generation determinism"),
}

CASES = [
    {"id": "two-apart", "sources": ["transition-table", "shared-trust"], "order": "concat"},
    {"id": "two-mixed", "sources": ["transition-table", "shared-trust"], "order": "shuffle"},
    {"id": "three-mixed", "sources": ["script-engine", "nl-ir", "mesh-keying"], "order": "shuffle"},
    {"id": "related-mixed", "sources": ["shared-trust", "shared-relay", "shared-mls"], "order": "shuffle"},
    {"id": "five-mixed", "sources": ["script-engine", "nl-ir", "host-features", "determinism", "transition-table"],
     "order": "shuffle"},
]


class CaseError(ValueError):
    """The corpus is not in the shape the scoring relies on."""


def split_chunks(text: str) -> list[str]:
    """A markdown text cut into chunks: what comes before the first `##`, then each `##` section.

    A line that begins with `## ` inside a fenced code block is code, not a heading, so fences are followed.
    """
    chunks: list[list[str]] = [[]]
    fenced = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
        if not fenced and line.startswith("## ") and any(chunks[-1]):
            chunks.append([])
        chunks[-1].append(line)
    return ["\n".join(c).strip("\n") for c in chunks if any(l.strip() for l in c)]


def lay_out(case: dict, seed: int, read=lambda path: (REPO / path).read_text(encoding="utf-8")) -> list[tuple[str, str]]:
    """The chunks of a case in the order they are laid, each with the source it came from."""
    chunks: list[tuple[str, str]] = []
    for name in case["sources"]:
        for chunk in split_chunks(read(SOURCES[name][0])):
            chunks.append((name, chunk))
    if case["order"] == "shuffle":
        random.Random(seed).shuffle(chunks)
    elif case["order"] != "concat":
        raise CaseError(f"{case['id']}: an order is `concat` or `shuffle`")
    return chunks


def extract_text(text: str):
    with tempfile.TemporaryDirectory() as folder:
        path = pathlib.Path(folder) / "doc.md"
        path.write_text(text, encoding="utf-8")
        done = EXTRACTORS.try_extract(path)
    if not hasattr(done, "blocks"):
        raise CaseError(f"the reader would not take the document: {done}")
    return done


def truth_by_block(chunks: list[tuple[str, str]], extraction) -> list[str]:
    """The source of every block of the extraction, found by reading each chunk alone.

    The document is read as a whole, as an owner's is, and the chunks are read one by one only to count their
    blocks: if the two do not say the same words in the same order the corpus is wrong and the case is refused,
    not scored.
    """
    sources: list[str] = []
    texts: list[str] = []
    for name, chunk in chunks:
        alone = extract_text(chunk)
        sources.extend([name] * len(alone.blocks))
        texts.extend(b.text for b in alone.blocks)
    if texts != [b.text for b in extraction.blocks]:
        raise CaseError("the chunks read alone do not say what the document says read whole")
    return sources


def part_truth(extraction, sections, sources: list[str]) -> dict[str, str]:
    """The source each part belongs to: the source of its first block (a part never straddles a chunk)."""
    truth: dict[str, str] = {}
    for section in sections:
        owners = set(sources[section.first:section.end])
        if len(owners) != 1:
            raise CaseError(f"part {section.id} straddles {sorted(owners)}: a chunk must begin at a heading")
        truth[section.id] = sources[section.first]
    return truth


def score_named(draft, truth: dict[str, str], weight: dict[str, int], keys: dict[str, str]) -> dict:
    """The model was offered a destination for each source: a suggestion is right when it names that source's."""
    listed = list(truth)
    placed = {s.part: s for s in draft.suggestions}
    right = [p for p, s in placed.items() if s.key is not None and keys.get(s.key) == truth[p]]
    new = [p for p, s in placed.items() if s.key is None]
    return {
        "parts": len(listed), "placed": len(placed), "right": len(right), "new_titles": len(new),
        "coverage": len(placed) / len(listed) if listed else 0.0,
        "precision": len(right) / len(placed) if placed else None,
        "paragraphs": sum(weight[p] for p in listed),
        "paragraphs_placed": sum(weight[p] for p in placed),
        "paragraphs_right": sum(weight[p] for p in right),
    }


def score_open(draft, truth: dict[str, str], weight: dict[str, int], origin: str) -> dict:
    """The model was offered one destination and proposed the others by title: groups are scored by purity."""
    groups: dict[str, list[str]] = defaultdict(list)
    for s in draft.suggestions:
        groups[origin if s.key is not None else (s.title or "").casefold()].append(s.part)
    majority_parts = majority_paragraphs = 0
    for members in groups.values():
        by_source: Counter = Counter()
        for p in members:
            by_source[truth[p]] += weight[p]
        top = by_source.most_common(1)[0][0]
        majority_parts += sum(1 for p in members if truth[p] == top)
        majority_paragraphs += by_source[top]
    spread: dict[str, set[str]] = defaultdict(set)
    for group, members in groups.items():
        for p in members:
            spread[truth[p]].add(group)
    placed = sum(len(m) for m in groups.values())
    listed = list(truth)
    return {
        "parts": len(listed), "placed": placed, "groups": len(groups),
        "sources": len(set(truth.values())),
        "coverage": placed / len(listed) if listed else 0.0,
        "purity": majority_parts / placed if placed else None,
        "fragmentation": (sum(len(g) for g in spread.values()) / len(spread)) if spread else None,
        "paragraphs": sum(weight[p] for p in listed),
        "paragraphs_placed": sum(weight[p] for p in draft_parts(draft)),
        "paragraphs_pure": majority_paragraphs,
    }


def draft_parts(draft) -> list[str]:
    return [s.part for s in draft.suggestions]


def baseline(truth: dict[str, str], weight: dict[str, int]) -> dict:
    """What saying nothing smart scores: every part with the biggest source."""
    by_source: Counter = Counter()
    for p, source in truth.items():
        by_source[source] += weight[p]
    total = sum(by_source.values())
    top, size = by_source.most_common(1)[0]
    return {"biggest_source": top, "paragraph_share": size / total if total else 0.0,
            "part_share": sum(1 for s in truth.values() if s == top) / len(truth)}


class LinkFailed(Exception):
    """The server could not be reached or did not answer in time: nothing is known of the model."""


def ask(endpoint: str, model: str, prompt: str, timeout: float, key: str | None) -> tuple[str | None, float]:
    """One chat completion, with no tools: the prompt in, the text out, and how long it took."""
    body = json.dumps({
        "model": model, "temperature": 0, "max_tokens": 8192,
        "messages": [{"role": "user", "content": prompt}],
    }).encode("utf-8")
    request = urllib.request.Request(
        endpoint.rstrip("/") + "/chat/completions", data=body, method="POST",
        headers={"content-type": "application/json", **({"authorization": f"Bearer {key}"} if key else {})},
    )
    began = time.monotonic()
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            answer = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        # The server answered, with an error: a context that was too long is a result about the model.
        return None, time.monotonic() - began
    except (urllib.error.URLError, socket.timeout, TimeoutError, ConnectionError, OSError) as exc:
        raise LinkFailed(str(exc)) from exc
    try:
        return answer["choices"][0]["message"]["content"], time.monotonic() - began
    except (KeyError, IndexError, TypeError):
        return None, time.monotonic() - began


def run_case(case: dict, way: str, args, out: pathlib.Path) -> dict:
    chunks = lay_out(case, args.seed)
    text = "\n\n".join(chunk for _, chunk in chunks)
    extraction = extract_text(text)
    sources = truth_by_block(chunks, extraction)
    _, sections = outline(extraction)
    truth_all = part_truth(extraction, sections, sources)
    names = case["sources"]
    if way == "named":
        offered = tuple(Offered(f"d{i}", "spec", SOURCES[n][1]) for i, n in enumerate(names))
        keys = {f"d{i}": n for i, n in enumerate(names)}
    else:
        offered = (Offered("d0", "spec", "The spec this document came with"),)
        keys = {}
    folder = out / f"{case['id']}.{way}"
    folder.mkdir(parents=True, exist_ok=True)
    result: dict = {"case": case["id"], "way": way, "sources": names, "chunks": len(chunks)}
    try:
        prepared = prepare(extraction, offered)
    except IngestError as exc:
        return {**result, "status": "refused", "why": str(exc)}
    (folder / "prompt.txt").write_text(prepared.prompt, encoding="utf-8")
    truth = {p: truth_all[p] for p in prepared.parts}
    weight = {s.id: s.blocks for s in sections if s.id in truth}
    result.update(parts=len(prepared.parts), prompt_characters=len(prepared.prompt),
                  baseline=baseline(truth, weight))
    try:
        answer, seconds = ask(args.endpoint, args.model, prepared.prompt, args.timeout, args.key)
    except LinkFailed as exc:
        return {**result, "status": "link-failed", "why": str(exc)}
    (folder / "answer.txt").write_text(answer or "", encoding="utf-8")
    draft = read_answer(extraction, prepared, offered, answer)
    (folder / "draft.json").write_text(json.dumps(draft.to_dict(), indent=1, sort_keys=True), encoding="utf-8")
    result.update(
        status="answered", seconds=round(seconds, 1),
        unreadable=None if draft.unreadable is None else draft.unreadable.code,
        refused=dict(Counter(r.code for r in draft.refused)),
    )
    if draft.unreadable is None:
        result["score"] = (
            score_named(draft, truth, weight, keys) if way == "named" else score_open(draft, truth, weight, "d0")
        )
        # What the placing was, for an owner of this evaluation to read the misses: ids only.
        (folder / "truth.json").write_text(json.dumps(truth, indent=1, sort_keys=True), encoding="utf-8")
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--endpoint", required=True, help="an OpenAI-compatible server, e.g. http://127.0.0.1:18081/v1")
    parser.add_argument("--model", default="", help="the model name the server wants, when it wants one")
    parser.add_argument("--key", default=None, help="a bearer key, when the server wants one")
    parser.add_argument("--out", required=True, help="where to write each case's prompt, answer, draft and the report")
    parser.add_argument("--only", help="comma-separated case ids")
    parser.add_argument("--way", choices=("named", "open", "both"), default="both")
    parser.add_argument("--seed", type=int, default=1, help="the seed the shuffled cases are laid out with")
    parser.add_argument("--timeout", type=float, default=1800.0, help="seconds a server is given for one answer")
    args = parser.parse_args(argv)
    chosen = set(args.only.split(",")) if args.only else None
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    ways = ("named", "open") if args.way == "both" else (args.way,)
    results = []
    for case in CASES:
        if chosen is not None and case["id"] not in chosen:
            continue
        for way in ways:
            result = run_case(case, way, args, out)
            results.append(result)
            print(json.dumps(result, sort_keys=True), flush=True)
    (out / "report.json").write_text(json.dumps(
        {"seed": args.seed, "endpoint": args.endpoint, "model": args.model, "results": results}, indent=1, sort_keys=True),
        encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
