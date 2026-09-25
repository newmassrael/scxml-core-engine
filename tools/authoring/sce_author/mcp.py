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
from .coverage import coverage as run_coverage
from .errors import AuthoringError
from .pack import load_pack
from .pseudo import render as render_pseudo
from .verify import verify as run_verify
from .prose import load_prose
from .questions import ask
from .review import review as run_review
from .scaffold import write as write_scaffold

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
        "name": "pseudo",
        "description": (
            "SHOW the written document the way a person reads it, so the "
            "specification owner can approve it. Every other tool here "
            "answers a question a machine can answer -- the names are real, "
            "the examples pass, the set reaches every position -- and a "
            "document can satisfy all of them and still not be what the "
            "specification asked for. Nothing but a person can say so, and a "
            "person handed XML does not read it. The surface is TOTAL: every "
            "field of the model reaches the page, a value appears as the "
            "author spelled it, and a document that cannot be shown in full "
            "is refused by name rather than abbreviated -- so approving the "
            "page is approving the document and not a summary of it. Call it "
            "after `verify` passes: a page that behaves wrongly is not worth "
            "a reader's time."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["binding"],
            "properties": {
                "binding": {
                    "type": "string",
                    "description": "The binding file, which names its own document.",
                },
                "deploy": {
                    "type": "string",
                    "description": (
                        "A deployment descriptor, when the document is placed "
                        "on one. Every line the deployment decides is then "
                        "shown too, each marked with a leading '!' -- strike "
                        "those and what is left is the undeployed page, byte "
                        "for byte."
                    ),
                },
                "shape": {
                    "type": "string",
                    "description": (
                        "How lines and nesting are written -- 'indent' (the "
                        "default) nests by two spaces a level, 'endmark' "
                        "closes each block with the word that opened it. A "
                        "choice of layout and never of content: a value "
                        "still appears as the author spelled it, so the same "
                        "document says the same thing in every shape. An "
                        "unknown name is refused with the names there are."
                    ),
                },
                "lexicon": {
                    "type": "string",
                    "description": (
                        "What the grammar's own words are called -- 'en' (the "
                        "default) or 'ko'. Only the words the grammar spends "
                        "are translated; what the document wrote is never "
                        "touched. A page in any pair but the default says so "
                        "on its first line, so a reviewer's approval can be "
                        "filed and read back later."
                    ),
                },
            },
        },
    },
    {
        "name": "scaffold",
        "description": (
            "Write a binding skeleton from the interface model, as a file that "
            "does not exist yet: `version`, `document`, and one rule per "
            "position the model declares -- each output with its address, its "
            "field and a map keyed by the platform's numbers, each input with "
            "its address. Nothing that is a reading of the specification is "
            "written, and `activation` only if you give it. Start the binding "
            "from this, rename rules to your document's identifiers, delete "
            "what the document does not use, and run `check`: it names every "
            "rule that still does not fit."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "document", "binding"],
            "properties": {
                "pack": _PACK_ARG,
                "document": {
                    "type": "string",
                    "description": "The document the binding will name, as the binding writes it.",
                },
                "binding": {
                    "type": "string",
                    "description": "The binding file to create. Refused if it exists.",
                },
                "activation": {
                    "type": "string",
                    "enum": ["on-change", "periodic"],
                    "description": "When the host runs the document, if you know it.",
                },
            },
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
        "name": "coverage",
        "description": (
            "What the whole SET of documents reaches. Every other tool here "
            "is handed one binding and is right about one document, so a "
            "conversion that needed five components and produced three "
            "reports green -- the three that exist all pass, and the two "
            "nobody wrote are absent from no list, because there was no "
            "list. Positions two documents both write are an error: one "
            "field cannot take two answers. Positions nobody writes are "
            "reported as a figure, because unfinished work looks exactly "
            "like that."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["pack", "bindings"],
            "properties": {
                "pack": _PACK_ARG,
                "bindings": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": (
                        "Every binding in the subject matter. A binding left "
                        "out is indistinguishable from a document nobody "
                        "wrote, which is the thing this answers."
                    ),
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
                "backend": {
                    "type": "string",
                    "description": (
                        "Which lowering of the document to DRIVE, defaulting "
                        "to python. The product emits six; this drives the "
                        "one it can import into its own process and refuses "
                        "the rest, naming what would have to exist first. The "
                        "answer carries the backend it is about, because a "
                        "pass is a statement about one lowering and most of "
                        "this product ships as another."
                    ),
                },
            },
        },
    },
]


def verification_payload(result) -> dict:
    """What a run of the examples looks like on the wire.

    ⚠ A FUNCTION RATHER THAN A LITERAL INSIDE THE DISPATCH, so a test can hand
    it a result it built and read what comes back. Inline, the only way to see
    this shape was to run the product's code generator -- and a checkout with
    no build has not got one, so the case that could have noticed a field
    missing here was a case that silently skipped.

    ⚠⚠ `unasserted` is why this exists. `verify` grew a figure saying which
    written positions no case expects, the command line printed it, and this
    transport did not: a caller over MCP read "every case passed" with no way
    to learn how little had been judged, which is the exact sentence the
    figure was added to prevent, reproduced one surface over.

    Same shape as `questions`: versioned, an object, and the counts beside the
    detail. A client drawing a pass/fail bar reads the counts; a model reads
    the cases.
    """
    return {
        "version": 1,
        # ⚠ Beside the counts, because the counts are about ONE lowering and
        # most of this product ships as another.
        "backend": result.backend,
        "counts": {"passed": result.passed, "failed": result.failed,
                   "unjudged": result.unjudged},
        "unbound": result.unbound,
        "unasserted": result.unasserted,
        # ⚠ What the RUN kept that the product will not. The generated code
        # takes one round's inputs, so a caller has to hold these between
        # calls; a reader shown only a pass would not know they were owed.
        "host_memory": result.host_memory,
        # ⚠ What the pass RESTS ON that no case can reach. A client that
        # draws a green bar from the counts alone draws it over these.
        "assumed_preconditions": result.assumed_preconditions,
        "refuted_assumptions": result.refuted,
        # ⚠ What the binding declared it does NOT know, and what that cost.
        # The client most likely to read this is the model that wrote the
        # binding -- and the reason the key was never used is that declaring
        # an unknown used to cost everything. It has to see the price is now
        # only the positions that genuinely turn on it, or it goes back to
        # guessing an address that passes.
        # ⚠ `outputs` is the DOCUMENT's undecided values; `output_addresses`
        # is the BINDING's unplaced ones. Two questions for two people, so
        # they are not merged under one name.
        "unresolved": {"inputs": result.unresolved,
                       "outputs": result.unresolved_outputs,
                       "output_addresses": result.unaddressed_outputs,
                       "withheld_positions": result.undetermined},
        "cases": [
            {"name": case.name,
             "passed": case.passed,
             "judged": case.judged,
             "refusal": case.refusal,
             "failures": [{"address": a, "expected": w, "got": g}
                          for a, w, g in case.failures],
             "unchecked": case.unchecked,
             "unwritten": case.unwritten,
             "undetermined": case.undetermined}
            for case in result.results
        ],
    }


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

        if name == "pseudo":
            binding = args.get("binding")
            if not binding or not isinstance(binding, str):
                raise ToolArgumentError("'binding' is required: the path to the "
                                        "binding file, which names its own document")
            deploy = args.get("deploy")
            if deploy is not None and not isinstance(deploy, str):
                raise ToolArgumentError("'deploy' has to be a path, as a string")
            # ⚠ Type only. WHICH shapes and lexicons exist is the product's
            # registry to answer, and a set listed here would refuse a name
            # the product accepts the day one is registered -- so an unknown
            # name travels to the generator and comes back as its refusal,
            # which names the real set.
            shape = args.get("shape")
            if shape is not None and not isinstance(shape, str):
                raise ToolArgumentError("'shape' has to be a name, as a string")
            lexicon = args.get("lexicon")
            if lexicon is not None and not isinstance(lexicon, str):
                raise ToolArgumentError("'lexicon' has to be a name, as a string")
            # ⚠ No pack is loaded, and none is asked for. Rendering needs the
            # document alone, and a caller handed a refusal about their pack
            # when they asked to read their document is told about the wrong
            # file.
            got = render_pseudo(pathlib.Path(binding), None,
                                pathlib.Path(deploy) if deploy else None,
                                shape, lexicon)
            if not got.produced:
                return _failure(got.refusal)
            # ⚠ The page itself, as text and not as JSON. This is the one
            # answer here whose reader is a PERSON: the other tools return an
            # object because a client draws from it, and quoting the page into
            # a JSON string would put backslashes through the very spellings
            # the surface exists to preserve.
            return _text(got.text)

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

        if name == "scaffold":
            pack = load_pack(_pack_arg(args))
            document, binding = args.get("document"), args.get("binding")
            for key, value in (("document", document), ("binding", binding)):
                if not value or not isinstance(value, str):
                    raise ToolArgumentError(f"{key!r} is required, as a string")
            activation = args.get("activation")
            if activation is not None and not isinstance(activation, str):
                raise ToolArgumentError("'activation' has to be a string")
            text = write_scaffold(pack, document, pathlib.Path(binding), activation)
            return _text(f"wrote {binding}:\n\n{text}")

        if name == "coverage":
            pack = load_pack(_pack_arg(args))
            bindings = args.get("bindings")
            if not bindings:
                raise ToolArgumentError(
                    "'bindings' is required: every binding in the subject "
                    "matter. With none, nothing is written and every position "
                    "reads as unreached, which is not what that means")
            if isinstance(bindings, str) or not isinstance(bindings, (list, tuple)):
                raise ToolArgumentError("'bindings' has to be a LIST of paths, "
                                        "even when there is only one")
            bad = [p for p in bindings if not isinstance(p, str)]
            if bad:
                raise ToolArgumentError(
                    f"'bindings' holds {bad[0]!r}, which is not a path")
            got = run_coverage(pack, [pathlib.Path(p) for p in bindings])
            # ⚠ Same shape as `questions` and `verify`: versioned, an object,
            # counts beside the detail. A client drawing a bar reads the
            # counts; a model reads the names.
            payload = {
                "version": 1,
                "counts": {"positions": len(got.positions),
                           "written": got.covered,
                           "unwritten": len(got.unwritten),
                           "contested": len(got.contested())},
                "unwritten": got.unwritten,
                "contested": [{"position": p, "documents": who}
                              for p, who in got.contested()],
                "undeclared": got.undeclared,
            }
            text = json.dumps(payload, ensure_ascii=False, indent=1)
            return _failure(text) if got.alarms() else _text(text)

        if name == "verify":
            pack = load_pack(_pack_arg(args))
            binding = args.get("binding")
            if not binding or not isinstance(binding, str):
                raise ToolArgumentError("'binding' is required: the path to the "
                                        "binding file, which names its own document")
            backend = args.get("backend", "python")
            if not isinstance(backend, str):
                raise ToolArgumentError("'backend' has to be a language name, "
                                        "as a string")
            result = run_verify(pack, pathlib.Path(binding), None, backend)
            if not result.ran:
                return _failure(result.refusal)
            payload = verification_payload(result)
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
