"""A requirement set built from what a client QUOTES out of the source.

The product consumes a closed requirement set and never derives one
(`sce-build/src/requirement_manifest.rs`): reading a specification is the
authoring pass's job. This is that pass's deterministic half. The client says
which words of the source state a requirement, and everything that can be
decided without trusting the client is decided here:

* a quote that is not in the source, word for word, is refused, so a requirement
  cannot be invented;
* a quote that points at more than one place in the source is refused, so it
  cannot be anchored to the wrong one;
* the ids of a first list come from WHERE the words are in the source, in
  reading order, never from the client's numbering or the order it listed them
  in; a later revision of the specification keeps the id of every requirement
  it still has (`requirement_lineage`), so an id means the same requirement in
  both revisions or is closed and never issued again;
* every sentence of the source is a section, so a sentence no requirement quotes
  shows as an empty one, which is the only way a requirement list is checked for
  completeness by something that is not the list's author.

⚠ Measured 2026-10-01, Sonnet, three specifications, five independent
extractions each: the quotes were in the source 35 times of 35, and the lists
were the same in content -- 5 of 5 identical for one specification, 5 of 5 on
five of six requirements for another (one clause quoted whole or in two), and
11 of 12 for the third, where the twelfth differed by three words of span. The
drafts of that third specification fell into five classes at every level. What
a specification asks for converges where what a draft does with it does not,
which is what makes the list a denominator worth checking a draft against.

⚠ It is still `ids: synthesized`. A prose specification names no requirements of
its own, so nothing in the source audits WHICH sentences were counted, and the
product says so of any coverage figure over it. The manifest states that; this
module does not soften it.

The manifest carries coordinates only and the sentences go in a sidecar, because
the product refuses a manifest with prose in any string
(`requirement_manifest.rs`, "Why the manifest holds no requirement text"): the
sidecar holds the client's QUOTE, verbatim, and never its paraphrase.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field

from . import requirement_lineage as rl
from .errors import AuthoringError

# `sce-build` reads the requirement's modality from the manifest. A requirement
# met by something being ABSENT ("buttons are ignored") can only be tested, and
# the product answers it `needs-scenario` rather than `implemented`.
MODALITIES = ("shall", "shall_not")


class RequirementSetError(AuthoringError):
    """The request cannot be read as a list of quotes at all."""


@dataclass
class Requirement:
    id: str
    quote: str
    statement: str
    sentence: str
    modality: str
    status: str = "new"


@dataclass
class Refusal:
    index: int
    quote: str
    why: str


@dataclass
class Built:
    """Either `requirements` and the two files, or `refused` and neither: a
    partial list is a wrong denominator, so nothing is offered until every
    quote can be anchored."""

    requirements: list[Requirement] = field(default_factory=list)
    refused: list[Refusal] = field(default_factory=list)
    manifest: dict | None = None
    sidecar: dict | None = None
    unclaimed_sentences: list[tuple[str, str]] = field(default_factory=list)
    unclaimed_words: list[tuple[str, str]] = field(default_factory=list)
    rev: str = "1"
    lineage: dict | None = None
    delta: dict | None = None
    notes: list[str] = field(default_factory=list)


def normalise(text: str) -> str:
    """Whitespace collapsed to single spaces: a specification wrapped at
    another width, or a quote copied across a line break, is the same words."""
    return re.sub(r"\s+", " ", text).strip()


def sentences(text: str) -> list[tuple[int, int]]:
    """The spans `[start, end)` of each sentence of already-normalised `text`.

    A sentence ends at `.`, `!` or `?` followed by a space or the end. Nothing
    cleverer: an abbreviation splits a sentence in two, which costs a section
    that holds half of one requirement's words, and a cleverer rule would be a
    second place that decides what a sentence is."""
    spans, start = [], 0
    for match in re.finditer(r"[.!?]+(?=\s|$)", text):
        spans.append((start, match.end()))
        start = match.end() + 1
    if start < len(text):
        spans.append((start, len(text)))
    return [(a, b) for a, b in spans if text[a:b].strip()]


def build(specification: str, items: object, *, doc_id: str = "spec",
          rev: str = "1", lineage_text: str | None = None,
          previous_manifest_text: str | None = None,
          previous_sidecar_text: str | None = None,
          continues: object = None) -> Built:
    """The manifest and sidecar for `items`, a list of `{quote, statement}`
    (and optionally `modality`), or the refusals that stand in the way.

    With no lineage the list is the first of its specification: ids in reading
    order, `rev` as given. With `lineage_text` (what a previous call returned), or
    with the previous manifest and sidecar of a list that predates lineages, it is
    a revision: each requirement keeps the id it had, and `rev` follows the
    specification. `previous_sidecar_text` gives the words behind the ids, which
    is what lets a reworded requirement keep its id; `continues` maps a quote to
    the id it continues, for a rewording too large to recognise."""
    if not isinstance(items, list) or not items:
        raise RequirementSetError(
            "'requirements' has to be a non-empty list of objects with a "
            "'quote' and a 'statement'")
    source = normalise(specification)
    if not source:
        raise RequirementSetError("the specification is empty")
    built = Built()
    located: list[tuple[int, int, dict, str]] = []
    seen: dict[str, int] = {}
    for index, item in enumerate(items):
        if not isinstance(item, dict) or not isinstance(item.get("quote"), str) \
                or not item["quote"].strip():
            built.refused.append(Refusal(index, str(item)[:80],
                                         "not an object with a non-empty 'quote'"))
            continue
        quote = normalise(item["quote"])
        modality = item.get("modality", "shall")
        if modality not in MODALITIES:
            built.refused.append(Refusal(
                index, quote, f"modality '{modality}' is not one of {', '.join(MODALITIES)}"))
            continue
        if quote in seen:
            built.refused.append(Refusal(
                index, quote, f"the same quote as item {seen[quote]}: one requirement, once"))
            continue
        seen[quote] = index
        first = source.find(quote)
        if first < 0:
            built.refused.append(Refusal(
                index, quote, "not in the specification, word for word: quote the "
                              "words of the source, not a rewording of them"))
            continue
        if source.find(quote, first + 1) >= 0:
            built.refused.append(Refusal(
                index, quote, "appears more than once in the specification, so it "
                              "points at no one place: quote enough words to be unique"))
            continue
        located.append((first, first + len(quote), item, quote))
    if built.refused:
        return built

    # Reading order, then the longer span first: a requirement that holds
    # another comes before it, whichever the client listed first.
    located.sort(key=lambda entry: (entry[0], -entry[1]))
    spans = sentences(source)

    def sentence_of(offset: int) -> int:
        for number, (start, end) in enumerate(spans, 1):
            if start <= offset < end:
                return number
        return len(spans)

    spec = rl.Spec(rl.sha256_text(source),
                   [rl.sha256_text(source[a:b].strip()) for a, b in spans])
    quotes = [quote for _, _, _, quote in located]
    try:
        step = _identify(doc_id, rev, spec, quotes, lineage_text, previous_manifest_text,
                         previous_sidecar_text, continues)
    except rl.LineageError as error:
        raise RequirementSetError(str(error)) from error
    built.rev, built.lineage, built.delta, built.notes = step.rev, step.lineage, step.delta, step.notes

    for number, (start, end, item, quote) in enumerate(located):
        built.requirements.append(Requirement(
            id=step.ids[number], quote=quote,
            statement=str(item.get("statement", "")).strip(),
            sentence=f"S{sentence_of(start)}",
            modality=item.get("modality", "shall"),
            status=step.statuses[number]))

    entries = []
    for requirement in built.requirements:
        entry = {"id": requirement.id, "section": requirement.sentence}
        if requirement.modality != "shall":
            entry["modality"] = requirement.modality
        entries.append(entry)
    built.manifest = {
        "doc_id": doc_id, "rev": built.rev,
        "extraction": {"ids": "synthesized", "trace": "none",
                       "modality_convention": "english-modal-verbs",
                       "method": "ai-pass-1"},
        "sections": [{"id": f"S{n}", "title": f"Sentence {n}"}
                     for n in range(1, len(spans) + 1)],
        "requirements": entries,
    }
    built.sidecar = {"doc_id": doc_id, "rev": built.rev,
                     "text": {r.id: r.quote for r in built.requirements}}

    covered = [False] * len(source)
    for start, end, _, _ in located:
        for offset in range(start, end):
            covered[offset] = True
    for number, (start, end) in enumerate(spans, 1):
        label = f"S{number}"
        if not any(covered[start:end]):
            built.unclaimed_sentences.append((label, source[start:end].strip()))
            continue
        run = None
        for offset in range(start, end + 1):
            open_ = offset < end and not covered[offset]
            if open_ and run is None:
                run = offset
            elif not open_ and run is not None:
                piece = source[run:offset].strip()
                if re.search(r"\w", piece):
                    built.unclaimed_words.append((label, piece))
                run = None
    return built


def _json(text: str, what: str) -> dict:
    try:
        data = json.loads(text)
    except ValueError as error:
        raise RequirementSetError(f"{what} is not JSON: {error}") from error
    if not isinstance(data, dict):
        raise RequirementSetError(f"{what} is not a JSON object")
    return data


def _identify(doc_id: str, rev: str, spec: rl.Spec, quotes: list[str], lineage_text: str | None,
              manifest_text: str | None, sidecar_text: str | None, continues: object) -> rl.Advance:
    """The ids of `quotes`: a first list's, or a revision's against a lineage (or against
    the manifest and sidecar of a list that predates lineages, which are adopted as one)."""
    if lineage_text is not None and manifest_text is not None:
        raise RequirementSetError("give the lineage or the previous manifest, not both: the lineage already "
                                  "holds what the manifest would be adopted for")
    if lineage_text is None and manifest_text is None:
        if sidecar_text is not None or continues:
            raise RequirementSetError("'previous_sidecar' and 'continues' belong to a revision: give the "
                                      "lineage a previous call returned (or the previous manifest) as well")
        return rl.first(doc_id, spec, quotes, rev)
    if continues is not None and (not isinstance(continues, dict) or not all(
            isinstance(k, str) and isinstance(v, str) for k, v in continues.items())):
        raise RequirementSetError("'continues' has to map a quote of this list to the id it continues")
    texts = None
    if sidecar_text is not None:
        sidecar = _json(sidecar_text, "the previous sidecar")
        words = sidecar.get("text")
        if sidecar.get("doc_id") != doc_id or not isinstance(words, dict):
            raise RequirementSetError(f"the previous sidecar is not the sidecar of '{doc_id}'")
        texts = {id_: normalise(words_) for id_, words_ in words.items() if isinstance(words_, str)}
    if lineage_text is not None:
        previous = rl.parse(lineage_text)
    else:
        if texts is None:
            raise RequirementSetError("the previous manifest needs its sidecar: the words behind its ids "
                                      "are not in the manifest")
        previous = rl.adopt(doc_id, _json(manifest_text, "the previous manifest"), texts)
    return rl.advance(previous, doc_id, spec, quotes, texts=texts,
                      continues={normalise(k): v for k, v in (continues or {}).items()})


def answer(built: Built) -> dict:
    """The JSON a tool returns. `manifest_text` and `sidecar_text` are the two
    files a later call takes as `manifest_text` and `sidecar_text`; `lineage_text`
    is the third, which the next revision of the specification is built against."""
    if built.refused:
        return {
            "verdict": "refused",
            "refused": [{"item": r.index, "quote": r.quote, "why": r.why}
                        for r in built.refused],
            "next": ("no requirement set is offered while any quote cannot be "
                     "anchored: a partial list is a wrong denominator. Fix every "
                     "item listed and call again."),
        }
    revised = built.delta is not None
    reply = {
        "verdict": "built",
        "rev": built.rev,
        "requirements": [{"id": r.id, "sentence": r.sentence, "quote": r.quote,
                          "statement": r.statement, "modality": r.modality,
                          **({"status": r.status} if revised else {})}
                         for r in built.requirements],
        "unclaimed_sentences": [{"sentence": s, "text": t}
                                for s, t in built.unclaimed_sentences],
        "unclaimed_words": [{"sentence": s, "text": t}
                            for s, t in built.unclaimed_words],
        "manifest_text": json.dumps(built.manifest, indent=2) + "\n",
        "sidecar_text": json.dumps(built.sidecar, indent=2, ensure_ascii=False) + "\n",
        "lineage_text": rl.render(built.lineage),
        "basis": ("synthesized: this prose specification names no requirements of "
                  "its own, so the list is the client's reading of it and nothing "
                  "in the source audits which sentences were counted"),
        "next": ("save manifest_text as requirements.manifest.json, sidecar_text "
                 "as requirements.sidecar.json and lineage_text as "
                 "requirements.lineage.json beside the specification, exactly as "
                 "returned (the ids in the design mean what the manifest and sidecar "
                 "say, and the sidecar holds the quoted sentences: tell the owner "
                 "where they are, or give the texts in your reply when you cannot "
                 "write files; the lineage holds no words and is the one the next "
                 "revision of the specification is built against, with the sidecar "
                 "as `previous_sidecar`); "
                 "put each id on the state or transition that carries it "
                 "(sce:req=\"R3\"); check the design with validate_scxml, or "
                 "validate_scxml_set when it imports other documents, passing "
                 "manifest_text; give scxml_acceptance_report both texts; and tell "
                 "the owner the sentences in unclaimed_sentences that no requirement "
                 "quotes"),
    }
    if revised:
        reply["delta"] = built.delta
        reply["next"] += ("; this is a revision: tell the owner what `delta` says moved, and that a "
                          "requirement with status `changed` has other words than its id had")
    else:
        reply["next"] += ("; this is the first list of this specification as far as this call can tell: "
                          "if it was built before, give that call's `lineage_text` (and its sidecar as "
                          "`previous_sidecar`) and build it again, or the ids will not follow the "
                          "requirements it already named")
    if built.notes:
        reply["notes"] = built.notes
    return reply
