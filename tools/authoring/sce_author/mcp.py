"""An MCP server over stdio, exposing what this core can do.

⚠ **It does not write the document.** The tools hand a model the
materials and then judge what it wrote; the writing is the model's. That split
is not a limitation waiting to be removed, it is the measured result: a
mechanical translator built against the same corpus reached 6 of 17 cases on a
component where a model reading the same materials reached 17 of 17. What the
translator could not do was decide, and deciding is the whole content of a
specification.

So the shape a caller gets is:

    brief      everything needed to write the document, assembled from any
               number of sources in any format there is a reader for
    questions  what the specification does not answer, which is the half a
               writer cannot discover by reading harder
    review     whether the PACK those two rest on is worth resting on
    check      whether what was written can reach the platform at all
    verify     whether it BEHAVES, by running it

⚠⚠ `review` reached this transport later than the rest, and the gap is worth
recording rather than quietly closing: a caller reaching this core over MCP
had no way to ask whether the pack it was being answered from was any good,
which is the first thing to ask when the pack is new or hand-written. Every
other tool here trusts it silently.

JSON-RPC 2.0, one message per line, no dependencies beyond the core's own.
A transport is not a place for cleverness: it parses, dispatches, and turns
every failure into an answer rather than a traceback, because a server that
dies takes with it the one account of why.
"""

from __future__ import annotations

import json
import pathlib
import sys
import traceback

from .brief import assemble
from .check import check
from .errors import AuthoringError
from .pack import load_pack
from .verify import verify as run_verify
from .prose import load_prose
from .questions import ask
from .review import review as run_review

PROTOCOL_VERSION = "2024-11-05"
SERVER_NAME = "sce-author"
SERVER_VERSION = "1"

_PACK_ARG = {
    "type": "string",
    "description": "Directory holding the interface model and the conventions.",
}
_PROSE_ARG = {
    "type": "array",
    "items": {"type": "string"},
    "description": (
        "One or more specification files. Several are resolved as one body,"
        " because a name introduced in one document is used in another and"
        " reading them apart reports the second file's use as unknown."
    ),
}

TOOLS = [
    {
        "name": "brief",
        "description": (
            "Assemble everything needed to write a document for this "
            "specification: the text itself, the addresses it touches with "
            "their value spaces, what each output becomes when its "
            "precondition is false, the precondition vocabulary, and the "
            "questions already known. Read this before writing anything -- "
            "every conversion that came out wrong against this corpus went "
            "wrong on a platform convention rather than on a misreading."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "prose"],
            "properties": {"pack": _PACK_ARG, "prose": _PROSE_ARG},
        },
    },
    {
        "name": "questions",
        "description": (
            "What the specification does not answer, in classes. Ask this "
            "before writing and again before believing a conversion: a "
            "source-not-fully-read answer invalidates every other class, "
            "because they all ask what the text does not say."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "prose"],
            "properties": {
                "pack": _PACK_ARG,
                "prose": _PROSE_ARG,
                "kind": {
                    "type": "string",
                    "description": "Return only this class.",
                },
            },
        },
    },
    {
        "name": "review",
        "description": (
            "Measure the PACK, before trusting anything the other tools say "
            "about a document. Every other answer here compares a document "
            "against the pack, so none of them can be more right than the "
            "pack is -- and the pack was written by whoever owns the "
            "platform, by hand or by a converter nothing here has ever run. "
            "Reports the share of the prose the subject partition attributes, "
            "how many addresses the text never writes under any spelling the "
            "pack gives them, how many output positions no example expects, "
            "and the shapes that cannot be right whatever the platform turns "
            "out to be. It gives figures and refuses a verdict: 'correct' is "
            "not something a claim about an absent platform can be told. Call "
            "it FIRST when the pack is new or hand-written."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "prose"],
            "properties": {"pack": _PACK_ARG, "prose": _PROSE_ARG},
        },
    },
    {
        "name": "check",
        "description": (
            "Judge a written document and its binding against the interface "
            "model: every address must exist, every field must be real, and "
            "every symbol must be one the field admits. Refusals are listed; "
            "an empty list means the document can reach the platform, which "
            "is a weaker claim than being correct."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "binding"],
            "properties": {
                "pack": _PACK_ARG,
                "binding": {
                    "type": "string",
                    "description": "The binding file, which names its own document.",
                },
            },
        },
    },
    {
        "name": "verify",
        "description": (
            "RUN the document against the pack's examples and report which "
            "cases it fails and where. This is the only thing here that says "
            "whether a document BEHAVES; `check` only says its names are real, "
            "which a document can satisfy while computing the wrong answer "
            "everywhere. Call it after writing, and again after every edit -- "
            "a conversion that has not been run has not been shown to work. A "
            "document with an open decision in it is refused by the code "
            "generator, and that refusal carries the reason its author wrote."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "binding"],
            "properties": {
                "pack": _PACK_ARG,
                "binding": {
                    "type": "string",
                    "description": "The binding file, which names its own document.",
                },
            },
        },
    },
]


def _text(payload: str) -> dict:
    return {"content": [{"type": "text", "text": payload}]}


def _failure(payload: str) -> dict:
    return {"content": [{"type": "text", "text": payload}], "isError": True}


def _pack_arg(args: dict) -> pathlib.Path:
    """The `pack` argument, or a sentence saying what is missing.

    ⚠ This used to surface as `KeyError: 'pack'` and a missing list as
    `TypeError: 'int' object is not iterable`. Both are true and neither tells
    a caller what to send instead, which is the whole job of an error crossing
    a wire to somebody else's program.
    """
    value = args.get("pack")
    if not value:
        raise ToolArgumentError("'pack' is required: the directory holding the "
                                "interface model and conventions")
    if not isinstance(value, str):
        raise ToolArgumentError("'pack' has to be a path, as a string")
    return pathlib.Path(value)


def _prose_arg(args: dict) -> list[pathlib.Path]:
    value = args.get("prose")
    if not value:
        raise ToolArgumentError("'prose' is required: a list of specification "
                                "files. With none, every question answers "
                                "itself clean, which reads like a complete "
                                "specification.")
    if isinstance(value, str) or not isinstance(value, (list, tuple)):
        raise ToolArgumentError("'prose' has to be a LIST of paths, even when "
                                "there is only one")
    bad = [p for p in value if not isinstance(p, str)]
    if bad:
        raise ToolArgumentError(f"'prose' holds {bad[0]!r}, which is not a path")
    return [pathlib.Path(p) for p in value]


class ToolArgumentError(AuthoringError):
    """The client's arguments, described so the client can fix them."""


def call_tool(name: str, args: dict) -> dict:
    """Run one tool. Every failure comes back as an answer, never a crash."""
    try:
        if name == "brief":
            pack = load_pack(_pack_arg(args))
            prose = load_prose(_prose_arg(args))
            return _text(assemble(prose, pack))

        if name == "questions":
            pack = load_pack(_pack_arg(args))
            prose = load_prose(_prose_arg(args))
            found = ask(prose, pack.model, pack.conventions, pack.examples)
            wanted = args.get("kind")
            if wanted is not None and not isinstance(wanted, str):
                raise ToolArgumentError("'kind' has to be a class name, as a string")
            if wanted:
                found = [q for q in found if q.kind == wanted]
            # ⚠ A version, and an object rather than a bare array.
            #
            # Two callers read this and they read it differently: a model
            # reads the text, and a program parses it to draw markers. The
            # second one needs to know which shape it got -- pinned to a bare
            # array, it has nothing to check and no way to notice the day the
            # shape moves. The first is unaffected either way.
            payload = {
                "version": 1,
                "counts": {k: sum(1 for q in found if q.kind == k)
                           for k in sorted({q.kind for q in found})},
                "questions": [q.as_dict() for q in found],
            }
            return _text(json.dumps(payload, ensure_ascii=False, indent=1))

        if name == "review":
            pack = load_pack(_pack_arg(args))
            prose = load_prose(_prose_arg(args))
            got = run_review(pack, prose)
            # ⚠ The same shape `questions` uses, and for the same reason: a
            # model reads it and a program parses it, and the second needs to
            # know which shape it got. `alarms` is kept apart from the figures
            # because it is the only part that claims anything -- everything
            # beside it is a count somebody still has to interpret.
            payload = {
                "version": 1,
                "figures": {
                    "addresses": got.addresses,
                    "outputs": got.outputs,
                    "single_spelling": got.single_spelling,
                    "never_written_in_the_prose": len(got.unmentioned),
                    "prose_attributed": round(got.attribution, 3),
                    "blocks": got.blocks_built,
                    "blocks_of_one_or_two_lines": got.thin_blocks,
                    "has_examples": got.has_examples,
                    "driven_but_undeclared": got.driven_undeclared,
                    "output_positions_expected": got.asserted_outputs,
                    "output_positions_never_expected":
                        len(got.unasserted_outputs),
                },
                "unmentioned": sorted(got.unmentioned),
                "never_expected": got.unasserted_outputs,
                "alarms": got.alarms(),
            }
            return _text(json.dumps(payload, ensure_ascii=False, indent=1))

        if name == "check":
            pack = load_pack(_pack_arg(args))
            binding = args.get("binding")
            if not binding or not isinstance(binding, str):
                raise ToolArgumentError("'binding' is required: the path to the "
                                        "binding file, which names its own document")
            findings = check(pack, pathlib.Path(binding))
            if not findings:
                return _text("no refusals: every address, field and symbol exists.")
            return _failure("\n".join(str(f) for f in findings))

        if name == "verify":
            pack = load_pack(_pack_arg(args))
            binding = args.get("binding")
            if not binding or not isinstance(binding, str):
                raise ToolArgumentError("'binding' is required: the path to the "
                                        "binding file, which names its own document")
            result = run_verify(pack, pathlib.Path(binding))
            if not result.ran:
                return _failure(result.refusal)
            # ⚠ Same shape as `questions`: versioned, an object, and the
            # counts beside the detail. A client drawing a pass/fail bar reads
            # the counts; a model reads the cases.
            payload = {
                "version": 1,
                "counts": {"passed": result.passed, "failed": result.failed,
                           "unjudged": result.unjudged},
                "unbound": result.unbound,
                "refuted_assumptions": result.refuted,
                "cases": [
                    {"name": case.name,
                     "passed": case.passed,
                     "judged": case.judged,
                     "refusal": case.refusal,
                     "failures": [{"address": a, "expected": w, "got": g}
                                  for a, w, g in case.failures],
                     "unchecked": case.unchecked}
                    for case in result.results
                ],
            }
            text = json.dumps(payload, ensure_ascii=False, indent=1)
            return _failure(text) if result.failed else _text(text)

        return _failure(f"no tool named {name!r}")
    # ⚠ The BASE type. Listing `PackError` alone sent every document that could
    # not be read down the arm below, so a client that named a missing file got
    # a Python traceback back over the wire -- this program's internals, as an
    # answer to the client's own ordinary mistake.
    except AuthoringError as exc:
        return _failure(str(exc))
    except (FileNotFoundError, KeyError, TypeError) as exc:
        return _failure(f"{type(exc).__name__}: {exc}")
    except Exception:  # noqa: BLE001 - a server that dies takes the account with it
        return _failure(traceback.format_exc(limit=4))


def _invalid(ident, message: str, code: int = -32600) -> dict:
    return {"jsonrpc": "2.0", "id": ident, "error": {"code": code, "message": message}}


def handle(message) -> dict | None:
    """One request to one response. None means the message wanted no reply.

    ⚠ `message` is whatever the client sent, which is not necessarily an
    object. A bare array reached `message.get` and raised, and because the
    serve loop did not guard the call either, ONE malformed frame ended the
    session -- every later request, valid ones included, got no reply at all.
    """
    if not isinstance(message, dict):
        return _invalid(None, "a request has to be a JSON object")
    method = message.get("method")
    ident = message.get("id")
    if not isinstance(method, str):
        return _invalid(ident, "'method' has to be a string")

    if method == "initialize":
        result = {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
        }
    elif method == "tools/list":
        result = {"tools": TOOLS}
    elif method == "tools/call":
        params = message.get("params") or {}
        if not isinstance(params, dict):
            return _invalid(ident, "'params' has to be an object", -32602)
        arguments = params.get("arguments") or {}
        if not isinstance(arguments, dict):
            return _invalid(ident, "'arguments' has to be an object", -32602)
        result = call_tool(params.get("name") or "", arguments)
    elif method == "ping":
        result = {}
    elif ident is None:
        # A notification. `notifications/initialized` is the one that matters
        # and it wants silence; answering it is a protocol error.
        return None
    else:
        return {
            "jsonrpc": "2.0",
            "id": ident,
            "error": {"code": -32601, "message": f"unknown method {method!r}"},
        }

    if ident is None:
        return None
    return {"jsonrpc": "2.0", "id": ident, "result": result}


def serve(stdin=None, stdout=None) -> int:
    stdin = stdin or sys.stdin
    stdout = stdout or sys.stdout
    for line in stdin:
        line = line.strip()
        if not line:
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError as exc:
            # No id to answer against, so this is all that can be said.
            stdout.write(json.dumps({
                "jsonrpc": "2.0", "id": None,
                "error": {"code": -32700, "message": f"parse error: {exc}"},
            }) + "\n")
            stdout.flush()
            continue
        # ⚠⚠ The loop survives ONE bad message, whatever is wrong with it.
        # Before this guard a single malformed frame ended the session and
        # every later request -- including well-formed ones -- went unanswered,
        # which a client sees as a hang rather than as its own mistake.
        try:
            reply = handle(message)
        except Exception as exc:  # noqa: BLE001 - staying up outranks the bug
            ident = message.get("id") if isinstance(message, dict) else None
            reply = {
                "jsonrpc": "2.0", "id": ident,
                "error": {"code": -32603,
                          "message": f"internal error: {type(exc).__name__}: {exc}"},
            }
        if reply is not None:
            stdout.write(json.dumps(reply, ensure_ascii=False) + "\n")
            stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(serve())
