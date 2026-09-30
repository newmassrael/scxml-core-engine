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
    scxml_kinds     what each document kind is for, before one is chosen
    validate_scxml  whether an SCXML document passes the product's structural check
    render_scxml_pseudocode show that SCXML document as pseudocode, without a binding
    render_scxml_diagram    draw it as print figures, one SVG per container
    scxml_unresolved        what it marks as not decided yet
    scxml_requirements      what it claims, or each requirement's outcome
    scxml_acceptance_report the page the owner reads before accepting
    scxml_accept            record the OWNER's acceptance, on their word only
    scxml_acceptance_check  whether that acceptance still holds

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

import hashlib
import json
import pathlib
import re
import sys
import tempfile
import traceback

from . import brief as brief_sections
from .brief import assemble

# The most the brief returns in one result. ⚠ Chosen for the smallest client:
# a result near this size is about 15k tokens, which a 32k-context model can
# still hold beside its work. Over it, the caller gets the section index and
# asks for sections. Measured 2026-09-26: a 227,761-character brief was
# refused by the client that asked for it.
BRIEF_LIMIT = 60_000
from .check import check
from .compare import compare as compare_drafts
from .compare import summary as compare_summary
from .counterfactual import MAX_RUNS, explore
from .coverage import coverage as run_coverage
from .decisions import hold as hold_decisions
from .decisions import load_record as load_decision_record
from .errors import AuthoringError, describe_path
from .gaps import ORDER as GAP_ORDER
from .gaps import report as gap_report
from .pack import load_pack
from .pseudo import render as render_pseudo
from .verify import validate_scxml as run_scxml_validation
from .verify import (accept_design, acceptance_holds, acceptance_page,
                     diagram_figures, kind_catalog, requirement_records,
                     unresolved_markers, validate_scxml_set)
from .verify import page_provenance, pseudo_page, verify as run_verify
from .prose import load_prose
from .questions import ask
from .review import review as run_review
from .scaffold import KINDS as SCAFFOLD_KINDS
from .scaffold import write as write_scaffold

PROTOCOL_VERSION = "2024-11-05"
# Every revision this server answers in. The tools and their results have
# one shape in all of them; the later two differ in the HTTP transport,
# which `mcp_http` serves.
SUPPORTED_PROTOCOL_VERSIONS = ("2024-11-05", "2025-03-26", "2025-06-18")
SERVER_NAME = "sce-author"
SERVER_VERSION = "1"
SERVER_INSTRUCTIONS = (
    "When a specification owner asks for pseudocode from a prose specification, "
    "let them use a short natural-language request. Read the source they supplied. "
    "If the owner keeps an acceptance record for it, call scxml_accepted_for "
    "with that record, the specification and their decision record before "
    "writing anything: when it answers accepted-design, show that design's "
    "page verbatim and write no new draft unless the owner asks for one, "
    "because a new draft of the same specification is never the same "
    "document. "
    "Before writing XML, call scxml_kinds and choose the document kind from "
    "the source's stated inputs, outputs, events, retained state, timing, and "
    "data format, matched against each kind's choose_when and distinct_from. "
    "Name the kind, the source clauses supporting it, and the neighbouring "
    "kind it was told apart from, and record the same in the document: a "
    "<sce:kind-basis> directly under the root, with an <sce:evidence> for "
    "each supporting clause (its provenance anchor where the source gives "
    "one) and an <sce:rejected kind=...> for each neighbour ruled out; "
    "scxml_kinds gives its grammar and an example under declaration.basis. "
    "If the source states neither side of the "
    "behaviour that separates two candidates, it has not determined the "
    "kind: explain the alternatives and ask the owner about that behaviour. "
    "If you still write a draft, mark the kind as open on the "
    "<sce:kind-basis>: sce:unresolved=\"kind\", sce:unresolved-reason saying "
    "what the source would have to state, and sce:unresolved-candidates "
    "naming the other kinds still open. "
    "Write every document with an SCXML root; declare a non-statechart "
    "(Forge) kind with sce:kind in the SCE namespace http://sce.dev/ext. "
    "Do not silently omit sce:kind: SCE otherwise reads the document as a "
    "statechart. Never treat that default as evidence from the source. "
    "Draft the document yourself, in the shape of the chosen kind's example, "
    "and do not invent missing policy values; a kind's notes name the "
    "defaults the source has to decide. "
    "For a statechart, write its interface before its states: an "
    "sce:kind=\"event-schema\" document for each event the source says it "
    "takes from or sends to its surroundings, imported with <sce:import>, and "
    "sce:interface=\"closed\" on the statechart root, so every draft of the "
    "same specification crosses one boundary. An event the source does not "
    "name is a question for the owner, not a name to invent. An event that "
    "carries no data is still declared, so the interface stays closed: write "
    "<datamodel sce:payload=\"none\"/> in its schema when the source says it "
    "carries none, and mark that <datamodel> sce:unresolved, with the reason, "
    "when the source does not say whether it carries any -- never invent a "
    "field, and never drop sce:interface=\"closed\" to make a check pass; "
    "state what is open and tell the owner. When the owner keeps an "
    "authoring profile -- a file stating what a design is held to, such as "
    "a closed interface -- pass it as profile (or profile_text) to "
    "validate_scxml and validate_scxml_set, and to scxml_accept, "
    "scxml_acceptance_check and scxml_accepted_for as the profile the design "
    "is held to: a statechart that departs from it is refused as a profile/* "
    "record, which you fix in the draft or put to the owner, and a design "
    "accepted under one profile is not the answer for another. Never write a "
    "profile yourself or choose one for the owner, and never edit the "
    "profile to make a draft pass; when the owner keeps none, say the design "
    "was checked under none, which the manifest shows by having no profile. "
    "When the profile lists house_rules, they are the owner's standing "
    "answers to gaps that recur: where a gap in the specification is one a "
    "rule answers, apply the rule instead of asking the owner, and cite it "
    "with sce:assumed=\"<rule id>\" on the element it applies to, so that the "
    "answer says every place a rule was applied. Never apply a rule without "
    "citing it, never cite one that does not answer the gap, and never write "
    "a rule yourself. Pass the profile to decisions too, so a citation of a "
    "rule is not refused as an uncited guess. "
    "Put the <?xml ...?> declaration "
    "first in every file, with nothing before it -- not a comment. What you "
    "check is the text you save and show: check the file as saved. "
    "Hand a document to the tools by path when this server runs beside the "
    "files, and otherwise as document_text with a document_name; documents "
    "that import one another are checked together with validate_scxml_set. "
    "Call validate_scxml on the draft and fix reported issues; confirm that "
    "the manifest's document_kind names the kind you chose and that its "
    "basis_recorded is true. An accepted answer that carries `open` is not "
    "finished: tell the owner each of its lines. A statechart's output "
    "needs a receiver -- a <send> with no target and no type goes to the "
    "machine itself and is discarded -- so send what the machine tells its "
    "surroundings to a host-served processor (<send type=...>), and use "
    "target=\"#_parent\" only when the specification names the statechart "
    "that invokes it. An event the machine sends ITSELF -- a deadline, a "
    "retry -- is its own only when the <send> names target=\"#_internal\": "
    "with no target it goes to the queue a caller delivers to, so a caller "
    "can send the same name, and a closed interface refuses a transition "
    "that takes it (declare the event, or send it to #_internal). When the "
    "product accepts the document, the answer of validate_scxml (and of "
    "validate_scxml_set, one page per document) carries its pseudocode page "
    "in the block after the JSON, with the sha256 of the bytes it was "
    "rendered from: show each page verbatim in a fenced block -- do not "
    "translate, summarize, rename labels, or add a source fact as if it were "
    "a rendered line -- and print that sha256 under the fence, outside it. "
    "Edit the document after that and the page is stale: validate again. "
    "render_scxml_pseudocode renders a page without a verdict, for a "
    "document whose page is wanted in another shape or lexicon; its answer "
    "is the page and then a note that names the same things. "
    "List decisions the source leaves "
    "open in prose outside that block, not inside it. "
    "When the owner keeps a decision record, a draft cites it: "
    "sce:assumed=\"<id>\" where it applies an answer, sce:unresolved=\"<id>\" "
    "where it asks a recorded question still unanswered, and a new id only "
    "for a question the record does not hold. Call decisions on the draft "
    "with that record and fix what it refuses; ask the owner each new "
    "question it reports, beside any recorded question on the same clause, "
    "and record their answer before drafting again. Explain that these "
    "tools describe kinds and inspect SCXML; they do not choose a kind "
    "from prose, convert prose, or prove agreement with it. The owner "
    "reviews the pseudocode against the source. Never call scxml_accept "
    "without the owner's explicit acceptance, and when you do, give it the "
    "specification and the decision record the design was written from as "
    "its sources, and the profile it is held to. No pack or binding is "
    "needed for this flow."
)

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


def _file_input(key: str, what: str) -> dict:
    """The properties that hand over one file: a path on this server's
    machine, or the file itself as text.

    ⚠ Both, because a client is not always where the server is. A path is
    the local form -- the owner's own tree, read in place. Text is the only
    form that crosses machines, and a server reached over HTTP refuses a path
    outright: it names this machine's files, which the caller neither sees
    nor should reach."""
    return {
        key: {"type": "string", "description": (
            f"Path to {what} on the machine this server runs on (local "
            f"servers only).")},
        f"{key}_text": {"type": "string", "description": (
            f"{what[0].upper()}{what[1:]} itself, as text, in place of "
            f"`{key}`.")},
        f"{key}_name": {"type": "string", "description": (
            f"The file name `{key}_text` is read under. For a document its "
            f"stem is the document's name, and other documents import it by "
            f"it.")},
    }


_DOCUMENT_INPUT = _file_input("document", "the SCXML document")
_MANIFEST_INPUT = _file_input("manifest", "the requirement manifest")
# The owner's authoring profile: what a design is held to, stated in a file
# the owner keeps beside the specification. Only the product reads it, so it
# is handed over untouched and a setting the product adds needs no edit here.
_PROFILE_INPUT = _file_input("profile", "the owner's authoring profile")
# What a design was authored from and held to: the specification files, the
# owner's decision record and authoring profile when there are ones. The
# acceptance record pins them, so a revised specification lapses an
# acceptance, and the same one asked about again is answered with the
# accepted design -- but only under the profile it was accepted under.
_AUTHORED_FROM_INPUT = {
    "sources": {"type": "array", "items": {"type": "string"},
                "description": "Paths to the specification files (local servers only)."},
    "sources_text": {
        "type": "array",
        "description": "The specification files themselves, each under its own name.",
        "items": {
            "type": "object",
            "required": ["name", "text"],
            "properties": {"name": {"type": "string"}, "text": {"type": "string"}},
        },
    },
    **_file_input("decisions", "the owner's decision record"),
    **_PROFILE_INPUT,
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
            "properties": {
                "pack": _PACK_ARG,
                "prose": _PROSE_ARG,
                "sections": {
                    "type": "array",
                    "items": {"type": "integer", "minimum": 1},
                    "description": (
                        "Only these numbered sections. Without it the whole "
                        "brief is returned when it fits, and otherwise its "
                        "section index with each section's size."),
                },
            },
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
                        "keeps that indentation and also closes each block "
                        "with the word that opened it. A "
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
        "name": "scxml_kinds",
        "description": (
            "The product's catalog of document kinds (sce-codegen kinds), "
            "for choosing a document's sce:kind BEFORE writing it. For each "
            "kind: its role, what a document of it describes, the evidence "
            "a specification offers for it (choose_when), the behaviour "
            "that separates it from the kinds it is confused with "
            "(distinct_from), what the author must decide first (notes), "
            "the backends that emit it, whether it may be declared inline, "
            "and a checked-in example document the product accepts. Needs "
            "no pack, binding or document. It does not choose: when the "
            "source states neither side of the behaviour a distinct_from "
            "entry names, the source has not determined the kind, and the "
            "owner is the one to ask."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "description": "Only this kind's entry, by its sce:kind name.",
                },
            },
        },
    },
    {
        "name": "validate_scxml",
        "description": (
            "Run sce-codegen check --lint on an existing SCXML-root document, "
            "including an explicit sce:kind Forge document. "
            "Needs only the document -- a path, or its text -- not a pack "
            "or binding. Returns "
            "JSON: verdict (accepted/refused), the manifest, and EVERY "
            "diagnostic record -- all lint findings in one run, not the "
            "first. Applies the declared kind's checks (including statechart "
            "design lints when appropriate); a pass does not "
            "say it matches the prose specification. The manifest's "
            "document_kind names the kind the product read the document as, "
            "and document_kind.declared is false when no sce:kind was found "
            "and the statechart default applied. `accepted` is the "
            "product's verdict, not a finished design: when the run leaves "
            "something to a person -- a question the specification leaves "
            "open (manifest.unresolved), a send to a parent session that "
            "nothing here invokes (manifest.needs_parent), a processor the "
            "host must serve -- the answer also carries `open`, one line "
            "for each, and `next`. Tell the owner each line of `open` before "
            "calling the design done. When the owner keeps an authoring "
            "profile, pass it (`profile` or `profile_text`): a statechart "
            "that is valid and is not what the profile asks for is refused "
            "as a profile/* record, and an accepted manifest names the "
            "profile by digest. Without one the check holds the design to "
            "nothing the owner asked for, and the manifest has no `profile`. "
            "When the product ACCEPTS the document, the answer carries its "
            "pseudocode page as the block after the JSON -- the page of the "
            "very bytes that were checked -- and `pages` names its sha256: "
            "show that page verbatim in a fenced block and print the sha256 "
            "under it (`show` says the rest). A refused document has no page."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {**_DOCUMENT_INPUT, **_PROFILE_INPUT},
        },
    },
    {
        "name": "validate_scxml_set",
        "description": (
            "Check documents that refer to one another as one set -- a "
            "statechart and the event schemas or enums it imports, a worker "
            "and the link it reads, a link and its framer codec -- with "
            "sce-codegen check. Each document's kind is read by the product, "
            "and an import is resolved by file name among the set's "
            "documents. Returns JSON: verdict, the set's manifest, and "
            "every diagnostic record -- and `open` and `next` when a "
            "member leaves something to a person, such as a payload the "
            "specification does not settle. Give either `documents` (paths) "
            "or `documents_text` (each document's name and text). When the "
            "owner keeps an authoring profile, pass it: every document of "
            "the set the profile asks something of is judged and every "
            "departure is listed, and manifest.profile.judged says how many "
            "that was -- zero when it asks nothing of any document of the set. "
            "When the set is ACCEPTED, each document's pseudocode page follows "
            "the JSON as its own block, in the order the documents were given, "
            "and `pages` names each one's sha256: show each verbatim in a "
            "fenced block and print its sha256 under it (`show` says the rest)."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "documents": {"type": "array", "items": {"type": "string"},
                              "description": "Paths to the documents (local servers only)."},
                "documents_text": {
                    "type": "array",
                    "description": "The documents themselves, each named as the others import it.",
                    "items": {
                        "type": "object",
                        "required": ["name", "text"],
                        "properties": {"name": {"type": "string"},
                                       "text": {"type": "string"}},
                    },
                },
                **_file_input("deploy", "the deploy.yaml the set is deployed by"),
                **_PROFILE_INPUT,
            },
        },
    },
    {
        "name": "compare",
        "description": (
            "Compare two or more drafts of one specification -- written in "
            "separate conversations, or in separate sessions -- at every level: "
            "bytes, canonical XML, the logic the build compiles, the review "
            "table, the pseudocode page, the open questions each draft marks, "
            "and, for statecharts, what they do when driven alike. Returns "
            "JSON: for each level the classes of drafts that agree; for "
            "behaviour, the renaming of input events that makes two drafts "
            "alike, or a witness drive after which they end in different "
            "states, with the bound the drives were run to. Show the owner "
            "each witness and each question only some drafts marked: those "
            "are the places the specification left open or hard to read. "
            "Agreement is not correctness -- drafts can agree and all be "
            "wrong -- and `not judged` means the drives moved nothing."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "documents": {"type": "array", "items": {"type": "string"},
                              "description": "Paths to the drafts (local servers only)."},
                "documents_text": {
                    "type": "array",
                    "description": "The drafts themselves, each under its own name.",
                    "items": {
                        "type": "object",
                        "required": ["name", "text"],
                        "properties": {"name": {"type": "string"},
                                       "text": {"type": "string"}},
                    },
                },
            },
        },
    },
    {
        "name": "decisions",
        "description": (
            "Hold a draft to the owner's decision record: their answers to "
            "what the specification leaves open, each under an id the draft "
            "cites (sce:assumed=\"<id>\" where it applied an answer, "
            "sce:unresolved=\"<id>\" where it asks a question still open). "
            "Refused: a guess citing no decision, a guess on a question not "
            "answered yet, a question the owner already answered, and a "
            "decision variable holding a value other than the one chosen. "
            "Reported, never refused: a new question the record has not seen, "
            "shown beside every recorded question on the same clause, and an "
            "answer no marker cites. Returns JSON: verdict, findings (each "
            "with the marker, the decision and a message), next. Ask the "
            "owner each new question and record the answer before drafting "
            "again; whether a new question is one already recorded is the "
            "owner's reading. When the owner keeps an authoring profile with "
            "house rules, pass it (`profile` or `profile_text`): a guess that "
            "cites one of them, sce:assumed=\"<rule id>\", is the owner's "
            "standing answer and is not refused as an uncited guess; it is "
            "reported as the rule it applies."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                **_DOCUMENT_INPUT,
                **_file_input("decisions", "the owner's decision record"),
                **_PROFILE_INPUT,
                "files_text": {
                    "type": "array",
                    "description": (
                        "Files the document imports, as text, under the names "
                        "it imports them by (with document_text only)."),
                    "items": {
                        "type": "object",
                        "required": ["name", "text"],
                        "properties": {"name": {"type": "string"},
                                       "text": {"type": "string"}},
                    },
                },
            },
        },
    },
    {
        "name": "render_scxml_pseudocode",
        "description": (
            "Render an existing statechart or sce:kind Forge document as "
            "complete pseudocode for "
            "the specification owner to review. Needs only the document -- a "
            "path, or its text -- not a pack or binding. Show the returned "
            "text verbatim to the owner, in a fenced block: do not "
            "translate, summarize, or rename a line inside it. The owner "
            "must compare the page "
            "with the prose specification. The answer has TWO blocks: the "
            "page, and after it a short provenance note -- the sha256 of the "
            "document the page was rendered from, what the product's check "
            "says of that same document alone, what it leaves open, and that "
            "no owner acceptance is recorded. Print the note unchanged under "
            "the fenced page, outside it; it is not part of the page. Give "
            "the owner's `profile` when they keep one, so the check is "
            "held to it. A document that imports schemas is checked with its "
            "set by validate_scxml_set, and that answer is the one to quote."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                **_DOCUMENT_INPUT,
                **_PROFILE_INPUT,
                "shape": {"type": "string", "description": "Pseudocode layout name, passed to the generator."},
                "lexicon": {"type": "string", "description": "Pseudocode vocabulary name, passed to the generator."},
            },
        },
    },
    {
        "name": "render_scxml_diagram",
        "description": (
            "Draw an existing SCXML document as print figures for a "
            "specification: one SVG per container (the document, each "
            "compound or parallel state), written into `out`. Every "
            "transition is described once, numbered on its arrow with its "
            "row in the table under the figure, in the pseudocode page's "
            "own words. Returns JSON: verdict, figures, diagnostics -- the "
            "files written into `out`, or, without `out`, each figure's "
            "name and SVG text. A figure larger than the page at `min_pt` is "
            "refused with both sizes (cli/diagram-does-not-fit), never "
            "shrunk: use a larger page or split the container."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                **_DOCUMENT_INPUT,
                "out": {"type": "string", "description": (
                    "Directory the SVG files are written into (local servers "
                    "only). Without it the figures come back as text.")},
                "page": {"type": "string", "description": "Page name, passed to the generator (default a4-portrait)."},
                "min_pt": {"type": "number", "description": "Smallest type size in points (default 7)."},
                "lexicon": {"type": "string", "description": "Vocabulary name, passed to the generator."},
                # With a manifest the figures gain the requirement checklist
                # pages: every requirement, its outcome, and where the
                # figures show it -- 'not shown' marks a gap.
                **_MANIFEST_INPUT,
            },
        },
    },
    {
        "name": "scxml_unresolved",
        "description": (
            "List every sce:unresolved marker in an SCXML document -- what "
            "the author recorded as not decided by the specification. "
            "Returns JSON: verdict, markers (one record each, with its "
            "reason and location), diagnostics. An empty list means nothing "
            "is marked, not that nothing is open."
        ),
        "inputSchema": {
            "type": "object",
            "properties": dict(_DOCUMENT_INPUT),
        },
    },
    {
        "name": "scxml_requirements",
        "description": (
            "Report the requirements an SCXML document claims (sce:req). "
            "With `manifest` -- the specification's closed requirement set "
            "-- each requirement gets its outcome, including `missing`, "
            "which only the manifest can show. Returns JSON: verdict, "
            "records, diagnostics."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {**_DOCUMENT_INPUT, **_MANIFEST_INPUT},
        },
    },
    {
        "name": "scxml_acceptance_report",
        "description": (
            "Render the acceptance report the specification owner reads in "
            "one sitting before accepting a design: per requirement, "
            "everything its behaviour depends on, then the risk surface. "
            "Returns the page as text."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["variant"],
            "properties": {
                **_DOCUMENT_INPUT,
                **_MANIFEST_INPUT,
                "variant": {"type": "string", "description": "The variant the page is for."},
                # Verbatim sentences by requirement id.
                **_file_input("sidecar", "the sentences sidecar"),
            },
        },
    },
    {
        "name": "scxml_accept",
        "description": (
            "Record that the specification OWNER accepted a design: writes "
            "an acceptance record pinning the manifest, the variant and "
            "every file the document read, by hash. Call this only when "
            "the owner has read the acceptance report and said to accept "
            "-- the record states a person's decision, and a caller that "
            "writes one on its own turns an unreviewed design into an "
            "accepted one. Returns JSON: verdict, record, diagnostics. With "
            "the files given as text the record comes back as `record_text`, "
            "its paths being the names the files were given; keep it, and "
            "give it with the same files to scxml_acceptance_check. Give the "
            "specification the design was written from (`sources`) and the "
            "owner's decision record if there is one: the record pins them "
            "too, so scxml_accepted_for can hand this design back when the "
            "same specification is asked for again."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["variant"],
            "properties": {
                **_DOCUMENT_INPUT,
                **_MANIFEST_INPUT,
                **_AUTHORED_FROM_INPUT,
                "variant": {"type": "string", "description": "The variant accepted."},
                "root": {"type": "string", "description": (
                    "Directory every pinned path is recorded relative to "
                    "(local servers, with paths).")},
                "out": {"type": "string", "description": (
                    "Where the acceptance record is written (local servers, "
                    "with paths).")},
            },
        },
    },
    {
        "name": "scxml_acceptance_check",
        "description": (
            "Ask whether an acceptance record still holds against the tree. "
            "Returns JSON: verdict `holds`, or `lapsed` with the product's "
            "record of what moved (cli/acceptance-lapsed) -- a lapsed "
            "acceptance is an answer, not an error. With `sources` or a "
            "decision record, holding also means the design was authored "
            "from exactly those files, compared by content."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["variant"],
            "properties": {
                **_file_input("record", "the acceptance record"),
                **_AUTHORED_FROM_INPUT,
                "variant": {"type": "string", "description": "The variant being asked about."},
                "root": {"type": "string", "description": (
                    "Directory the record's paths are read against (local "
                    "servers, with a record path).")},
                "files_text": {
                    "type": "array",
                    "description": (
                        "With `record_text`: every file the record names, "
                        "under the name it names it by."),
                    "items": {
                        "type": "object",
                        "required": ["name", "text"],
                        "properties": {"name": {"type": "string"},
                                       "text": {"type": "string"}},
                    },
                },
            },
        },
    },
    {
        "name": "scxml_accepted_for",
        "description": (
            "Before writing a draft, ask whether the owner already accepted a "
            "design for this specification. Give the acceptance record the "
            "owner keeps, the specification (`sources`) and their decision "
            "record if they have one. When the record still holds and was "
            "authored from exactly these files (compared by content), the "
            "answer is `accepted-design`, with the document, its text and its "
            "pseudocode page: show that page verbatim and write no new draft "
            "unless the owner asks for one -- a new draft of the same "
            "specification is never the same document. Otherwise the answer "
            "is `lapsed`, saying what differs, and a draft is written."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["variant"],
            "properties": {
                **_file_input("record", "the acceptance record"),
                **_AUTHORED_FROM_INPUT,
                "variant": {"type": "string", "description": "The variant being asked about."},
                "root": {"type": "string", "description": (
                    "Directory the record's paths are read against (local "
                    "servers, with a record path).")},
                "files_text": {
                    "type": "array",
                    "description": (
                        "With `record_text`: every file the record names, "
                        "under the name it names it by."),
                    "items": {
                        "type": "object",
                        "required": ["name", "text"],
                        "properties": {"name": {"type": "string"},
                                       "text": {"type": "string"}},
                    },
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
            "written, and `activation` only if you give it and the pack's "
            "`host` does not already say it for the platform. Start the binding "
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
                    "description": "When the host runs the document, if you know it and the pack's `host` does not say it.",
                },
                "kind": {
                    "type": "string",
                    "enum": list(SCAFFOLD_KINDS),
                    "description": (
                        "Also write the document itself, as this kind, where the "
                        "binding names it: a `transform` -- every output a "
                        "function of the inputs' current values -- gets its root "
                        "and one `<data>` per rule, with the binding's names, and "
                        "no output computed yet. Give it when the specification "
                        "makes no output remember anything across rounds."),
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
            "is a weaker claim than being correct. Give `prose` too, and a "
            "precondition the specification states that the document never "
            "reads is refused as well."
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
                "prose": _PROSE_ARG,
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
    {
        "name": "gaps",
        "description": (
            "Where the SPECIFICATION is silent, and what running the document "
            "found there. Every guess the document or binding records "
            "(`sce:assumed`, `assumed`) comes back refuted (the tests answer "
            "it, differently), untested (nothing compares it -- a pass says "
            "nothing about these) or held (the tests agree, the text should "
            "still say it); every unresolved value and assumed precondition "
            "is listed too, most urgent first, each with what the text should "
            "gain. Give `prose` to also locate each gap in the text and count "
            "what the text leaves open before anything is run."
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
                "prose": _PROSE_ARG,
                "backend": {
                    "type": "string",
                    "description": "As for verify: the lowering to drive, default python.",
                },
                "counterfactual": {
                    "type": "boolean",
                    "description": (
                        "Change each binding guess a failure implicates to every "
                        "alternative it has and run the cases again: a guess "
                        "whose every alternative leaves the failures as they "
                        "were comes back `cleared`, one whose alternative "
                        "repairs them cleanly comes back refuted with that "
                        "value. One run per alternative."),
                },
                "max_runs": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Runs `counterfactual` may spend (default 64); what it could not try is reported.",
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
        # ⚠ Every recorded guess and what the cases said of it -- refuted,
        # held, or UNTESTED. The last is the one a pass says nothing about,
        # and the author is the reader best placed to go and settle it.
        "assumptions": [
            {"subject": a.subject, "marker": a.marker, "status": a.status,
             "positions": a.positions, "held_in": len(a.held_in),
             "refuted_by": [{"case": c, "address": addr, "expected": want,
                             "got": got} for c, addr, want, got in a.refuted_by],
             "implicated_by": [{"case": c, "address": addr, "expected": want,
                                "got": got, "with": others}
                               for c, addr, want, got, others in a.implicated_by]}
            for a in result.assumptions.values()],
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


def _file_arg(args: dict, key: str, what: str,
              required: bool = True) -> pathlib.Path | None:
    """An argument naming a file that must already exist."""
    path = _path_arg(args, key, what, required)
    if path is not None and not path.is_file():
        raise ToolArgumentError(describe_path(path))
    return path


def _path_arg(args: dict, key: str, what: str,
              required: bool = True) -> pathlib.Path | None:
    """An argument naming a path, which need not exist yet."""
    value = args.get(key)
    if value is None and not required:
        return None
    if not isinstance(value, str) or not value:
        raise ToolArgumentError(f"'{key}' is required: {what}")
    return pathlib.Path(value)


def _name_arg(args: dict, key: str, what: str, required: bool = False) -> str | None:
    """An argument that is a name, passed to the product as it stands."""
    value = args.get(key)
    if value is None and not required:
        return None
    if not isinstance(value, str) or not value:
        raise ToolArgumentError(f"'{key}' has to be {what}, as a string")
    return value


class ToolArgumentError(AuthoringError):
    """The client's arguments, described so the client can fix them."""


# A file name a caller may give a file handed over as text: one plain name,
# no directory, so it lands in the staging directory and nowhere else.
_FILE_NAME = re.compile(r"[A-Za-z0-9_][A-Za-z0-9_.-]*")


class _Staging:
    """Where files handed over as text are written for the product to read.

    One directory per call, removed when the call returns, and the product
    runs in it -- so it names a staged document by the name its author gave
    it (`door.scxml`), an `<sce:import src>` between staged documents
    resolves the way it does beside each other on disk, and nothing a call
    writes outlives it.

    A file handed over by path is read where it is. On a remote server that
    form is refused: the path names this machine's files, not the caller's.
    """

    def __init__(self, remote: bool):
        self.remote = remote
        self._tmp = tempfile.TemporaryDirectory(prefix="sce-author-")
        self.dir = pathlib.Path(self._tmp.name)
        self._names: set[str] = set()

    def close(self) -> None:
        self._tmp.cleanup()

    def write(self, name, text, key: str) -> pathlib.Path:
        """Stage `text` as `name`; the path the product is given for it."""
        if not isinstance(text, str):
            raise ToolArgumentError(f"'{key}_text' has to be the file's text, as a string")
        if (not isinstance(name, str) or not _FILE_NAME.fullmatch(name)
                or set(name) == {"."}):
            raise ToolArgumentError(
                f"'{key}_name' has to be a plain file name such as door.scxml, "
                f"not {name!r}")
        if name in self._names:
            raise ToolArgumentError(f"two files are named {name!r}")
        self._names.add(name)
        (self.dir / name).write_text(text, encoding="utf-8")
        # Relative: the run starts in this directory.
        return pathlib.Path(name)

    def file(self, args: dict, key: str, what: str, default_name: str,
             required: bool = True) -> pathlib.Path | None:
        """`key` (a path) or `key_text` (the file), never both."""
        text, path = args.get(f"{key}_text"), args.get(key)
        if text is not None and path is not None:
            raise ToolArgumentError(f"give '{key}' or '{key}_text', not both")
        if text is not None:
            # Defaulted only when absent: an empty name is a caller's
            # mistake, and naming the file for them would hide it.
            name = args.get(f"{key}_name")
            return self.write(default_name if name is None else name, text, key)
        if path is None:
            if required:
                raise ToolArgumentError(f"'{key}' or '{key}_text' is required: {what}")
            return None
        self.refuse_path(key)
        return _file_arg(args, key, what).resolve()

    def many(self, args: dict, key: str, what: str) -> list[pathlib.Path]:
        """`key` (paths) or `key_text` (named files), never both."""
        texts, paths = args.get(f"{key}_text"), args.get(key)
        if (texts is None) == (paths is None):
            raise ToolArgumentError(f"give exactly one of '{key}' and '{key}_text': {what}")
        if texts is not None:
            if not isinstance(texts, list) or not texts:
                raise ToolArgumentError(f"'{key}_text' has to be a non-empty list of files")
            staged = []
            for entry in texts:
                if not isinstance(entry, dict):
                    raise ToolArgumentError(f"each '{key}_text' entry has a name and a text")
                staged.append(self.write(entry.get("name"), entry.get("text"), key))
            return staged
        self.refuse_path(key)
        if (not isinstance(paths, list) or not paths
                or not all(isinstance(p, str) and p for p in paths)):
            raise ToolArgumentError(f"'{key}' has to be a non-empty list of paths")
        found = [pathlib.Path(p) for p in paths]
        for path in found:
            if not path.is_file():
                raise ToolArgumentError(describe_path(path))
        return [path.resolve() for path in found]

    def write_aside(self, name, text, key: str) -> pathlib.Path:
        """Stage `text` apart from the files a record names, in `asked/`.

        For the files a question is ABOUT, which may carry the very name of
        a file the record pins -- the owner's current specification beside
        the one the design was accepted from -- and must not be read as it.
        """
        if not isinstance(text, str):
            raise ToolArgumentError(f"'{key}_text' has to be the file's text, as a string")
        if (not isinstance(name, str) or not _FILE_NAME.fullmatch(name)
                or set(name) == {"."}):
            raise ToolArgumentError(
                f"'{key}_name' has to be a plain file name such as spec.md, not {name!r}")
        aside = self.dir / "asked"
        aside.mkdir(exist_ok=True)
        if (aside / name).exists():
            raise ToolArgumentError(f"two files asked about are named {name!r}")
        (aside / name).write_text(text, encoding="utf-8")
        return pathlib.Path("asked") / name

    def refuse_path(self, key: str) -> None:
        if self.remote:
            raise ToolArgumentError(
                f"'{key}' names a file on the machine this server runs on, "
                f"which a remote caller cannot reach; send it as '{key}_text'")

    def local_path(self, args: dict, key: str, what: str) -> pathlib.Path | None:
        """A path argument naming where something is WRITTEN or read
        against -- local servers only."""
        if args.get(key) is None:
            return None
        self.refuse_path(key)
        return _path_arg(args, key, what).resolve()


def _answer(report: str, refusal: str) -> dict:
    return _failure(refusal) if refusal else _text(report)


def _kinds_tool(args: dict, staging: _Staging) -> dict:
    return _answer(*kind_catalog(_name_arg(args, "kind", "a kind name")))


def _profile_file(args: dict, staging: _Staging) -> pathlib.Path | None:
    """The owner's authoring profile, when they keep one: a path, or its text
    staged beside the documents. Not read here -- the product is the only
    reader of a profile, and refuses one it cannot use."""
    return staging.file(args, "profile", "the owner's authoring profile", "profile.json",
                        required=False)


def _digest_of(document: pathlib.Path, staging: _Staging) -> str:
    """The sha256 of the bytes the product reads: a document handed as text is
    named relative to the directory it was staged in, and the product runs
    there, so that is where its bytes are."""
    path = document if document.is_absolute() else staging.dir / document
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _with_pages(answer: dict, documents: list[pathlib.Path], staging: _Staging,
                digests: list[str]) -> dict:
    """The check's answer and, when the product ACCEPTED, the page of each
    document, one block apiece after the JSON.

    ⚠ Why the page rides on the check. Measured 2026-09-30, fifteen drafts by
    a real client asked for "a pseudocode draft, checked with SCE": the answer
    showed a page in ONE of them. The tool that renders it was a second call
    the instructions asked for and the client skipped fourteen times, saying it
    needed a pack it does not. A page that only exists if a second call is
    remembered is not shown to the owner; one that comes with the verdict is
    in front of the client every time the design is checked, and it is the page
    OF the bytes that were checked, by construction.

    ⚠ Raw text blocks, not a string inside the JSON: the client is told to show
    a page verbatim, and copying seventy lines out of an escaped JSON string is
    the step that paraphrases. The JSON says which block is whose.

    ⚠ `digests` were taken BEFORE the check; each is taken again after the
    page, and a document that changed in between gets no page: nothing may be
    said to come from a document that was checked when it is not the one that
    was.
    """
    pages, blocks = [], []
    for document, before in zip(documents, digests):
        entry = {"document": document.name, "sha256": before}
        page, refusal = pseudo_page(document, None, None, cwd=staging.dir)
        if _digest_of(document, staging) != before:
            entry["page"] = None
            entry["page_refusal"] = (f"{document.name} changed while it was being "
                                     f"checked, so no page is offered for it")
        elif refusal:
            entry["page"] = None
            entry["page_refusal"] = refusal.splitlines()[0] if refusal else "not rendered"
        else:
            entry["page"] = f"block {len(blocks) + 2}"
            blocks.append(page)
        pages.append(entry)
    answer["pages"] = pages
    answer["show"] = ("Show each page verbatim in its own fenced block, and under "
                      "it print that document's sha256. Then tell the owner each "
                      "line of `open`, and that no owner acceptance is recorded: "
                      "`accepted` is the product's verdict on the document, not "
                      "the owner's.")
    return {"content": [{"type": "text",
                         "text": json.dumps(answer, indent=2, ensure_ascii=False) + "\n"},
                        *({"type": "text", "text": block} for block in blocks)]}


def _validate_tool(args: dict, staging: _Staging) -> dict:
    document = staging.file(args, "document", "the SCXML document", "document.scxml")
    profile = _profile_file(args, staging)
    before = _digest_of(document, staging)
    report, refusal = run_scxml_validation(document, profile=profile, cwd=staging.dir)
    if refusal or not report:
        return _answer(report, refusal)
    return _with_pages(json.loads(report), [document], staging, [before])


def _validate_set_tool(args: dict, staging: _Staging) -> dict:
    documents = staging.many(args, "documents", "the documents of the set")
    deploy = staging.file(args, "deploy", "the deploy.yaml", "deploy.yaml", required=False)
    profile = _profile_file(args, staging)
    digests = [_digest_of(document, staging) for document in documents]
    report, refusal = validate_scxml_set(documents, deploy, profile=profile, cwd=staging.dir)
    if refusal or not report:
        return _answer(report, refusal)
    return _with_pages(json.loads(report), documents, staging, digests)


def _compare_tool(args: dict, staging: _Staging) -> dict:
    documents = staging.many(args, "documents", "the drafts to compare")
    # Staged drafts come back named relative to the staging directory; the
    # comparison runs the product on each, so it is handed where they are.
    located = [path if path.is_absolute() else staging.dir / path for path in documents]
    report = compare_drafts(located)
    report["summary"] = compare_summary(report)
    return _text(json.dumps(report, indent=2, ensure_ascii=False, default=str) + "\n")


def _decisions_tool(args: dict, staging: _Staging) -> dict:
    document = staging.file(args, "document", "the draft", "document.scxml")
    record = staging.file(args, "decisions", "the owner's decision record", "decisions.json")
    files = args.get("files_text")
    if files is not None:
        if args.get("document_text") is None:
            raise ToolArgumentError(
                "'files_text' goes with 'document_text': a document read by "
                "path imports the files beside it")
        if not isinstance(files, list):
            raise ToolArgumentError("'files_text' has to be a list of files")
        for entry in files:
            if not isinstance(entry, dict):
                raise ToolArgumentError("each 'files_text' entry has a name and a text")
            staging.write(entry.get("name"), entry.get("text"), "files")
    profile = _profile_file(args, staging)
    return _answer(*hold_decisions(document, record, profile=profile, cwd=staging.dir))


def _pseudocode_tool(args: dict, staging: _Staging) -> dict:
    document = staging.file(args, "document", "the SCXML document", "document.scxml")
    shape, lexicon = args.get("shape"), args.get("lexicon")
    for key, value in (("shape", shape), ("lexicon", lexicon)):
        if value is not None and not isinstance(value, str):
            raise ToolArgumentError(f"'{key}' has to be a name, as a string")
    page, refusal = pseudo_page(document, None, None, shape, lexicon, cwd=staging.dir)
    if refusal:
        return _failure(refusal)
    # The page is the FIRST block, byte for byte, as before. What it was
    # rendered from, and what the product says of that same document, is a
    # second block: a digest inside the page would make every pair of drafts
    # differ at the `compare` tool's `page` level.
    provenance, changed = page_provenance(document, profile=_profile_file(args, staging),
                                          cwd=staging.dir)
    if changed:
        return _failure(changed)
    return {"content": [{"type": "text", "text": page},
                        {"type": "text", "text": provenance}]}


def _diagram_tool(args: dict, staging: _Staging) -> dict:
    min_pt = args.get("min_pt")
    if min_pt is not None and (isinstance(min_pt, bool)
                               or not isinstance(min_pt, (int, float))):
        raise ToolArgumentError("'min_pt' has to be a number of points")
    document = staging.file(args, "document", "the SCXML document", "document.scxml")
    manifest = staging.file(args, "manifest", "the requirement manifest",
                            "requirements.manifest.json", required=False)
    out = staging.local_path(args, "out", "the directory the figures are written into")
    inline = out is None
    report, refusal = diagram_figures(
        document, out if out is not None else pathlib.Path("figures"),
        _name_arg(args, "page", "a page name"), min_pt,
        _name_arg(args, "lexicon", "a lexicon name"), manifest, cwd=staging.dir)
    if refusal or not inline:
        return _answer(report, refusal)
    # No directory to leave them in, so the figures come back themselves.
    answer = json.loads(report)
    answer["figures"] = [
        {"name": pathlib.Path(written).name,
         "svg": (staging.dir / written).read_text(encoding="utf-8")}
        for written in answer["figures"]]
    return _text(json.dumps(answer, indent=2, ensure_ascii=False) + "\n")


def _unresolved_tool(args: dict, staging: _Staging) -> dict:
    document = staging.file(args, "document", "the SCXML document", "document.scxml")
    return _answer(*unresolved_markers(document, cwd=staging.dir))


def _requirements_tool(args: dict, staging: _Staging) -> dict:
    document = staging.file(args, "document", "the SCXML document", "document.scxml")
    manifest = staging.file(args, "manifest", "the requirement manifest",
                            "requirements.manifest.json", required=False)
    return _answer(*requirement_records(document, manifest, cwd=staging.dir))


def _acceptance_report_tool(args: dict, staging: _Staging) -> dict:
    document = staging.file(args, "document", "the SCXML document", "document.scxml")
    manifest = staging.file(args, "manifest", "the requirement manifest",
                            "requirements.manifest.json")
    sidecar = staging.file(args, "sidecar", "the sentences sidecar",
                           "sentences.sidecar.json", required=False)
    return _answer(*acceptance_page(
        document, manifest, _name_arg(args, "variant", "the variant's name", required=True),
        sidecar, cwd=staging.dir))


def _authored_from(args: dict, staging: _Staging, *, aside: bool = False):
    """The specification files (`sources` / `sources_text`), the decision
    record (`decisions` / `decisions_text`) and the authoring profile
    (`profile` / `profile_text`) a design was authored from and held to, or a
    question is about. All optional; `aside` stages text apart from the
    files a record names (see `_Staging.write_aside`)."""
    sources: list[pathlib.Path] = []
    if args.get("sources") is not None or args.get("sources_text") is not None:
        if aside and args.get("sources_text") is not None:
            texts = args["sources_text"]
            if not isinstance(texts, list) or not texts:
                raise ToolArgumentError("'sources_text' has to be a non-empty list of files")
            for entry in texts:
                if not isinstance(entry, dict):
                    raise ToolArgumentError("each 'sources_text' entry has a name and a text")
                sources.append(staging.write_aside(entry.get("name"), entry.get("text"), "sources"))
        else:
            sources = staging.many(args, "sources", "the specification files")
    decisions = None
    if aside and args.get("decisions_text") is not None:
        if args.get("decisions") is not None:
            raise ToolArgumentError("give 'decisions' or 'decisions_text', not both")
        name = args.get("decisions_name")
        decisions = staging.write_aside("decisions.json" if name is None else name,
                                        args["decisions_text"], "decisions")
    else:
        decisions = staging.file(args, "decisions", "the owner's decision record",
                                 "decisions.json", required=False)
    if decisions is not None:
        # ⚠ Read, not only pinned. The acceptance record hashes whatever file
        # it is given, so a specification handed over as the decision record
        # -- the two sit side by side -- would be pinned as one and every
        # later question asked with the real record would read as a revision.
        load_decision_record(decisions if decisions.is_absolute() else staging.dir / decisions)
    # The profile is not read here, unlike the decision record: the product is
    # its only reader, and `accept` and `acceptance-check` both refuse a file
    # that is no profile as `cli/profile-unusable`, so a specification handed
    # over in its place is refused where the profile is used, by the reader
    # that knows what a profile is.
    if aside and args.get("profile_text") is not None:
        if args.get("profile") is not None:
            raise ToolArgumentError("give 'profile' or 'profile_text', not both")
        name = args.get("profile_name")
        profile = staging.write_aside("profile.json" if name is None else name,
                                      args["profile_text"], "profile")
    else:
        profile = _profile_file(args, staging)
    return sources, decisions, profile


def _accept_tool(args: dict, staging: _Staging) -> dict:
    variant = _name_arg(args, "variant", "the variant's name", required=True)
    root = staging.local_path(args, "root", "the directory the record's paths are relative to")
    out = staging.local_path(args, "out", "where the acceptance record is written")
    if root is not None or out is not None:
        # The owner's own tree: the record names their files where they are.
        if root is None or out is None:
            raise ToolArgumentError("'root' and 'out' go together")
        sources, decisions, profile = _authored_from(args, staging)
        report, refusal = accept_design(
            _file_arg(args, "document", "the accepted SCXML document").resolve(),
            _file_arg(args, "manifest", "the requirement manifest").resolve(),
            variant, root, out, sources=sources, decisions=decisions, profile=profile)
        if refusal:
            return _failure(refusal)
        return _text(_with_open_at_acceptance(report, out))
    # Handed over as text: the record pins them by the names they were
    # given, relative to the staging directory, and comes back itself --
    # there is no tree of the caller's to leave it in.
    if args.get("document") is not None or args.get("manifest") is not None:
        raise ToolArgumentError(
            "without 'root' and 'out', give the document and the manifest as "
            "text, so the record can name them")
    document = staging.file(args, "document", "the accepted SCXML document", "document.scxml")
    manifest = staging.file(args, "manifest", "the requirement manifest",
                            "requirements.manifest.json")
    sources, decisions, profile = _authored_from(args, staging)
    record = pathlib.Path("acceptance.json")
    report, refusal = accept_design(document, manifest, variant, pathlib.Path("."),
                                    record, sources=sources, decisions=decisions,
                                    profile=profile, cwd=staging.dir)
    if refusal:
        return _failure(refusal)
    answer = json.loads(_with_open_at_acceptance(report, staging.dir / record))
    answer["record"] = str(record)
    answer["record_text"] = (staging.dir / record).read_text(encoding="utf-8")
    return _text(json.dumps(answer, indent=2, ensure_ascii=False) + "\n")


def _with_open_at_acceptance(report: str, record: pathlib.Path) -> str:
    """`report` with what the record says the design was accepted WITH.

    Read back from the record the product just wrote, so the answer and the
    file cannot differ. Untouched when the design left nothing open: an
    acceptance of a finished design answers exactly as it did.
    """
    answer = json.loads(report)
    open_ = json.loads(record.read_text(encoding="utf-8")).get("open_at_acceptance")
    if open_:
        answer["accepted_with"] = [matter["message"] for matter in open_]
        answer["next"] = ("this design was accepted with the matters in `accepted_with` "
                          "still open: say so to the owner, who decided to accept it")
    return json.dumps(answer, indent=2, ensure_ascii=False) + "\n"


def _staged_record(args: dict, staging: _Staging):
    """The record and the root its paths are read against: the caller's own
    tree for a record path, or the staging directory for a record handed
    over as text with every file it names (`files_text`). Returns
    `(record, root, cwd)` for `acceptance_holds`."""
    if args.get("record_text") is None:
        record = staging.file(args, "record", "the acceptance record", "acceptance.json")
        root = staging.local_path(args, "root", "the directory the record's paths are read against")
        if root is None:
            raise ToolArgumentError("'root' is required with a record path")
        return record, root, None
    # The record and every file it names, as text, under the names it
    # names them by.
    record = staging.file(args, "record", "the acceptance record", "acceptance.json")
    files = args.get("files_text")
    if not isinstance(files, list) or not files:
        raise ToolArgumentError(
            "'files_text' is required with 'record_text': every file the "
            "record names, under the name it names it by")
    for entry in files:
        if not isinstance(entry, dict):
            raise ToolArgumentError("each 'files_text' entry has a name and a text")
        staging.write(entry.get("name"), entry.get("text"), "files")
    return record, pathlib.Path("."), staging.dir


def _acceptance_check_tool(args: dict, staging: _Staging) -> dict:
    variant = _name_arg(args, "variant", "the variant's name", required=True)
    record, root, cwd = _staged_record(args, staging)
    sources, decisions, profile = _authored_from(args, staging, aside=True)
    return _answer(*acceptance_holds(record, variant, root, sources=sources,
                                     decisions=decisions, profile=profile, cwd=cwd))


def _accepted_for_tool(args: dict, staging: _Staging) -> dict:
    """The accepted design for this specification, when there is one.

    ⚠ Asked BEFORE a draft is written. The model that writes drafts is the
    caller's and nothing makes two of them equal -- measured 2026-09-29, no
    two of thirty drafts of six specifications were byte-identical -- so the
    only way a second request for the same inputs gets the same design is to
    be given the one the owner already accepted. `holds` here means the
    record still holds AND the design was authored from exactly these files,
    compared by content; anything else is `lapsed`, with what differs.
    """
    variant = _name_arg(args, "variant", "the variant's name", required=True)
    record, root, cwd = _staged_record(args, staging)
    sources, decisions, profile = _authored_from(args, staging, aside=True)
    if not sources:
        raise ToolArgumentError(
            "'sources' or 'sources_text' is required: the specification whose "
            "accepted design is asked for")
    report, refusal = acceptance_holds(record, variant, root, sources=sources,
                                       decisions=decisions, profile=profile, cwd=cwd)
    if refusal:
        return _failure(refusal)
    answer = json.loads(report)
    if answer["verdict"] != "holds":
        answer["next"] = ("no accepted design answers for these files: write a draft, "
                          "and show the owner what differs")
        return _text(json.dumps(answer, indent=2, ensure_ascii=False) + "\n")
    base = root if cwd is None else cwd / root
    record_path = record if cwd is None else cwd / record
    document = json.loads(record_path.read_text(encoding="utf-8"))["document"]
    page, refused = pseudo_page(base / document, None, None, cwd=cwd)
    answer.update(
        verdict="accepted-design",
        document=document,
        document_text=(base / document).read_text(encoding="utf-8"),
        page=page if not refused else None,
        next=("show the owner this design's page verbatim; write no new draft "
              "unless the owner asks for one"),
    )
    if refused:
        answer["page_refusal"] = json.loads(refused)
    return _text(json.dumps(answer, indent=2, ensure_ascii=False) + "\n")


# The tools that need no pack: each takes its files by path or as text.
_PACK_FREE = {
    "scxml_kinds": _kinds_tool,
    "validate_scxml": _validate_tool,
    "validate_scxml_set": _validate_set_tool,
    "compare": _compare_tool,
    "decisions": _decisions_tool,
    "render_scxml_pseudocode": _pseudocode_tool,
    "render_scxml_diagram": _diagram_tool,
    "scxml_unresolved": _unresolved_tool,
    "scxml_requirements": _requirements_tool,
    "scxml_acceptance_report": _acceptance_report_tool,
    "scxml_accept": _accept_tool,
    "scxml_acceptance_check": _acceptance_check_tool,
    "scxml_accepted_for": _accepted_for_tool,
}


def call_tool(name: str, args: dict, *, remote: bool = False) -> dict:
    """Run one tool. Every failure comes back as an answer, never a crash.

    `remote` is true when the caller reached this server over HTTP: it may
    not name this machine's files, and hands every document over as text.
    """
    try:
        if name in _PACK_FREE:
            staging = _Staging(remote)
            try:
                return _PACK_FREE[name](args, staging)
            finally:
                staging.close()
        if remote:
            # The pack tools read a pack and prose from this machine's
            # disk; there is no text form of a pack to hand over.
            raise ToolArgumentError(
                f"'{name}' reads a pack from the machine this server runs on, "
                f"so it is not offered to a remote caller")

        if name == "brief":
            pack = load_pack(_pack_arg(args))
            prose = load_prose(_prose_arg(args))
            wanted = args.get("sections")
            if wanted is not None:
                if (not isinstance(wanted, list) or not wanted
                        or not all(isinstance(n, int) and not isinstance(n, bool) for n in wanted)):
                    raise ToolArgumentError("'sections' has to be a non-empty list of section numbers")
                try:
                    return _text(brief_sections.pick(prose, pack, wanted))
                except ValueError as exc:
                    raise ToolArgumentError(str(exc)) from None
            whole = assemble(prose, pack)
            if len(whole) > BRIEF_LIMIT:
                return _text(brief_sections.index(prose, pack, BRIEF_LIMIT))
            return _text(whole)

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
            prose = load_prose(_prose_arg(args)) if args.get("prose") is not None else None
            findings = check(pack, pathlib.Path(binding), prose)
            if not findings:
                return _text("no refusals: every address, field and symbol exists.")
            return _failure("\n".join(str(f) for f in findings))

        if name == "scaffold":
            pack = load_pack(_pack_arg(args))
            document, binding = args.get("document"), args.get("binding")
            for key, value in (("document", document), ("binding", binding)):
                if not value or not isinstance(value, str):
                    raise ToolArgumentError(f"{key!r} is required, as a string")
            activation, kind = args.get("activation"), args.get("kind")
            for key, value in (("activation", activation), ("kind", kind)):
                if value is not None and not isinstance(value, str):
                    raise ToolArgumentError(f"{key!r} has to be a string")
            text = write_scaffold(pack, document, pathlib.Path(binding), activation, kind)
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

        if name == "gaps":
            pack = load_pack(_pack_arg(args))
            binding = args.get("binding")
            if not binding or not isinstance(binding, str):
                raise ToolArgumentError("'binding' is required: the path to the "
                                        "binding file, which names its own document")
            backend = args.get("backend", "python")
            if not isinstance(backend, str):
                raise ToolArgumentError("'backend' has to be a language name, "
                                        "as a string")
            prose = load_prose(_prose_arg(args)) if args.get("prose") is not None else None
            result = run_verify(pack, pathlib.Path(binding), None, backend)
            if not result.ran:
                # No run, no attribution -- every guess would read untested.
                return _failure(result.refusal)
            questions = (ask(prose, pack.model, pack.conventions, pack.examples)
                         if prose is not None else ())
            wanted = args.get("counterfactual", False)
            if not isinstance(wanted, bool):
                raise ToolArgumentError("'counterfactual' is true or false")
            max_runs = args.get("max_runs", MAX_RUNS)
            if not isinstance(max_runs, int) or isinstance(max_runs, bool) or max_runs < 0:
                raise ToolArgumentError("'max_runs' is a count of runs, 0 or more")
            counterfactuals = (explore(
                pack, pathlib.Path(binding), result,
                lambda variant: run_verify(pack, variant, None, backend), max_runs)
                if wanted else None)
            found = gap_report(result, pack, prose, questions, counterfactuals)
            # The prose's own questions are counted, not carried: `questions`
            # is the tool that lists them, and there can be hundreds.
            counts = {kind: sum(1 for g in found if g.kind == kind) for kind in GAP_ORDER}
            payload = {"version": 1, "counts": counts,
                       "gaps": [g.as_dict() for g in found if g.kind != "question"]}
            return _text(json.dumps(payload, ensure_ascii=False, indent=1, default=str))

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


def handle(message, *, remote: bool = False) -> dict | None:
    """One request to one response. None means the message wanted no reply.

    `remote` is true for a message that arrived over HTTP -- see
    [`call_tool`].

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
        # The version the client asked for when this server speaks it, so a
        # client on a later revision is not told to fall back for nothing;
        # otherwise the one this server was written against.
        params = message.get("params")
        requested = params.get("protocolVersion") if isinstance(params, dict) else None
        result = {
            "protocolVersion": (requested if requested in SUPPORTED_PROTOCOL_VERSIONS
                                else PROTOCOL_VERSION),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
            "instructions": SERVER_INSTRUCTIONS,
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
        result = call_tool(params.get("name") or "", arguments, remote=remote)
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


def main(argv: list[str] | None = None) -> int:
    """stdio by default; `--http HOST:PORT` serves the same tools over HTTP
    to a caller on another machine (see `mcp_http`)."""
    import argparse

    parser = argparse.ArgumentParser(prog="python3 -m sce_author.mcp")
    parser.add_argument("--http", metavar="HOST:PORT",
                        help="serve over HTTP instead of stdio")
    parser.add_argument("--token-file", type=pathlib.Path,
                        help="a file holding the bearer token HTTP callers must send")
    args = parser.parse_args(argv)
    if args.http is None:
        if args.token_file is not None:
            parser.error("--token-file goes with --http")
        return serve()
    from .mcp_http import serve_http

    host, _, port = args.http.rpartition(":")
    if not host or not port.isdigit():
        parser.error("--http takes HOST:PORT, e.g. 127.0.0.1:8765")
    token = (args.token_file.read_text(encoding="utf-8").strip()
             if args.token_file is not None else None)
    return serve_http(host, int(port), token)


if __name__ == "__main__":
    raise SystemExit(main())
