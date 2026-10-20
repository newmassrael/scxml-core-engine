"""Command line for the authoring core.

Every input is named on the command line. The core opens no file it was not
given, which is the structural half of the domain-free guarantee: a subject
matter has no route in except through a pack the caller chose.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import pathlib
import sys
import tempfile

from .brief import write as write_brief
from .check import check, picture_notice, unnamed_pictures_of
from .compare import DRIVES, STEPS
from .compare import compare as run_compare
from .compare import summary as compare_summary
from .counterfactual import MAX_RUNS, explore
from .counterfactual import lines as counterfactual_lines
from .coverage import coverage as run_coverage
from .decisions import cited_by_binding
from .decisions import hold as hold_decisions
from .decisions import summary as decisions_summary
from .assembly import assemble, decisions_from_json, report_of
from .conversion import Conversion, Converter, check_conversion, convert_by_rules
from .conversion_by_model import (
    KIND as RECIPE_KIND,
    RECIPE_VERSION,
    UNIT_CHARACTERS,
    assemble_units,
    parse_answer,
    prompt_for,
    split_units,
)
from .errors import AuthoringError, IngestError
from .extraction import Extraction, NotCarried
from .notes import LANGUAGES as notes_languages
from .extractors import DEFAULT as DEFAULT_EXTRACTORS
from .extractors import Unreadable
from .gaps import ORDER as GAP_ORDER
from . import pdf_pictures
from .errors import describe_path
from .ingest import MAX_PICTURE_BYTES, picture_names_held, read_pictures
from .gaps import report as gap_report
from .pack import check_pack, load_pack
from .prose import load_prose
from .pseudo import render as render_pseudo
from .questions import ask
from .review import review as run_review
from .revision_gate import (DEFAULT_ROUNDS, MAX_ROUNDS, OUTSIDE_REACH, REVISER_TIMEOUT_S, STALLED,
                            GateError, command_reviser, guarded, keep_baseline)
from .revision_gate import hold as hold_revision
from .revision_gate import judge as judge_revision
from .scaffold import KINDS
from .scaffold import write as write_scaffold
from .carry import carry_ledger
from .divide_by_model import KIND as DIVIDE_KIND
from .divide_by_model import RECIPE_VERSION as DIVIDE_RECIPE_VERSION
from .divide_by_model import Prepared as DividePrepared
from .divide_by_model import offered_from, prepare as prepare_division, read_answer as read_division_answer
from .ledger import Ledger, check_ledger, make_ledger, scopes_of
from . import reading as picture_reading
from .scope import OUTLINES, Scope, outline as outline_of, scope_of
from .verify import product_findings_of_binding
from .verify import verify as run_verify


def _pack(args):
    return load_pack(pathlib.Path(args.pack))


def cmd_brief(args) -> int:
    pack = _pack(args)
    prose = load_prose([pathlib.Path(p) for p in args.prose])
    out = pathlib.Path(args.out)
    size = write_brief(prose, pack, out)
    # Only the size is printed. The brief carries specification text and a pack
    # may be confidential; a general tool does not decide that for its caller.
    print(f"brief: {size} bytes -> {out}")
    return 0


def cmd_picture(args) -> int:
    """Write the picture a `[picture: NAME]` mark names to a file, for whoever is going to look at it."""
    paths = [pathlib.Path(p) for p in args.prose]
    found = read_pictures(paths, args.name)
    if not found:
        held = picture_names_held(paths)
        said = "; ".join(f"{path}: {', '.join(names) or 'none'}" for path, names in held.items())
        raise IngestError(f"no specification file holds a picture named {args.name!r}."
                          + (f" The documents hold: {said}." if said else ""))
    if len(found) > 1:
        raise IngestError(f"{len(found)} specification files hold a picture named {args.name!r} "
                          f"({', '.join(str(one.source) for one in found)}); name one file in --prose")
    out = pathlib.Path(args.out)
    out.write_bytes(found[0].data)
    if args.json:
        # The format is said so that a host shows the picture or says it cannot; `null` is one a client
        # cannot be shown (a metafile, a TIFF), whose bytes are still written.
        print(json.dumps({"bytes": len(found[0].data), "media_type": found[0].mime,
                          "sha256": hashlib.sha256(found[0].data).hexdigest()}, sort_keys=True))
    else:
        print(f"picture: {len(found[0].data)} bytes -> {out}")
    return 0


def cmd_render_pictures(args) -> int:
    """Draw every picture a PDF holds that can be handed over, to be kept beside it as its renditions."""
    document = pathlib.Path(args.document)
    if document.suffix.lower() != ".pdf":
        raise IngestError(f"{document}: renditions are made of the images of a PDF, and this is not one")
    if not document.is_file():
        raise IngestError(describe_path(document))
    rendered, skipped = pdf_pictures.render_all(document, MAX_PICTURE_BYTES)
    out = pathlib.Path(args.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    for one in rendered:
        (out / one.name).write_bytes(one.png)
    listing = {
        "document": document.name,
        "renderer": pdf_pictures.renderer() if rendered else None,
        "pictures": [
            {"name": one.name, "media_type": "image/png", "bytes": len(one.png), "sha256": one.sha256}
            for one in rendered
        ],
        "skipped": [{"name": name, "why": why} for name, why in skipped],
    }
    (out / "renditions.json").write_text(json.dumps(listing, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"renditions: {len(rendered)} picture(s) drawn, {len(skipped)} not -> {out}")
    return 0


def cmd_extract(args) -> int:
    """Read one file into an extraction (ADR 0026, 0036) and say what it could not carry.

    Not a tool a client of a run is offered: a host that holds an original calls it, and a person may.
    """
    path = pathlib.Path(args.file)
    done = DEFAULT_EXTRACTORS.try_extract(path)
    if isinstance(done, Unreadable):
        # A typed answer and not a crash: the words name what would read it, the code says which kind.
        print(f"unread [{done.code}]: {done.words(path)}", file=sys.stderr)
        return 1
    out = pathlib.Path(args.out)
    # Exactly the canonical text and nothing after it, so that the SHA-256 of the file is the revision
    # printed below, which is also the name the specs folder keeps an extraction under (`save_extraction`).
    out.write_text(done.to_json(), encoding="utf-8")
    kinds = collections.Counter(block.kind for block in done.blocks)
    # Counts and the revision only: the blocks hold the document's own words, and a general tool does not
    # decide that they may be printed.
    print(
        f"extraction: {len(done.blocks)} block(s) "
        f"({', '.join(f'{n} {kind}' for kind, n in sorted(kinds.items())) or 'none'}) "
        f"by {done.extractor} {done.extractor_version}, revision {done.revision()} -> {out}"
    )
    for entry in done.not_carried:
        print(f"not carried: {entry.what}" + (f" ({entry.why})" if entry.why else ""))
    return 0


def _read_extraction(path: str) -> Extraction:
    try:
        return Extraction.from_json(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError) as exc:
        raise IngestError(f"{path}: not readable as an extraction ({exc})") from exc


def _read_conversion(path: str) -> Conversion:
    try:
        return Conversion.from_json(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError) as exc:
        raise IngestError(f"{path}: not readable as a conversion ({exc})") from exc


def _read_scope(path: str | None) -> Scope | None:
    """The scope the owner chose, or `None` for the whole extraction when none was named."""
    if path is None:
        return None
    try:
        return Scope.from_json(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError) as exc:
        raise IngestError(f"{path}: not readable as a scope ({exc})") from exc


def _summary(report) -> str:
    said = report.to_dict()
    left = f", {said['out_of_scope']} outside the chosen part" if said["out_of_scope"] else ""
    pictures = f", {len(said['read_from_picture'])} read from a picture" if said["read_from_picture"] else ""
    return (
        f"{'complete' if said['complete'] else 'not complete'}: {said['matched']} matched, "
        f"{len(said['check'])} to check, {len(said['not_in_original'])} not in the original, "
        f"{len(said['missing'])} missing, {len(said['recognised'])} read from an image{pictures}{left}"
    )


def cmd_outline(args) -> int:
    """The parts of an extraction the owner can choose between, each with an id, and what it holds.

    `--json` prints the outline for a host; the text form is for a person at a terminal. Titles are
    the document's own words, so they are printed here and not in any log a host keeps.
    """
    extraction = _read_extraction(args.extraction)
    way, sections = outline_of(extraction, args.by)
    if args.json:
        # `block_ids` are the paragraphs `scope_of` takes for the section as its own (`first` to `end`), with and
        # without words, so that a host reads a saved ledger, which keeps paragraphs, back into parts without
        # working out a block range of its own.
        listed = [
            {**s.to_dict(), "block_ids": [b.id for b in extraction.blocks[s.first:s.end]]} for s in sections
        ]
        print(json.dumps(
            {"v": 1, "extraction": extraction.revision(), "by": way, "sections": listed},
            sort_keys=True, ensure_ascii=False))
        return 0
    print(f"outline by {way}: {len(sections)} section(s)")
    for section in sections:
        pages = "" if section.page_first is None else (
            f" p.{section.page_first}" if section.page_first == section.page_last
            else f" p.{section.page_first}-{section.page_last}")
        title = section.title or f"({section.kind})"
        print(f"  {section.id}{'  ' * (section.level - 1)} {title}  [{section.blocks} block(s), "
              f"{section.characters} characters{pages}]")
        for item in section.items:
            print(f"    {'  ' * (section.level - 1)}- {item.kind} {item.id}: {item.label}")
    return 0


def cmd_render_notes(args) -> int:
    """Say what could not be carried in a language: a list of entries in, the same list out with the sentences
    of `--lexicon` written from each entry's code and facts.

    The sentences are written in `notes.py` and nowhere else, so a host that shows them to a reader asks here
    and writes none (ADR 0026). An entry the language has no sentence for (no code, a code this version does not
    know) comes out as it went in, in the English its extractor kept.
    """
    path = pathlib.Path(args.notes)
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise IngestError(f"{path}: not readable as a list of what was not carried ({exc})") from exc
    if not isinstance(raw, list):
        raise IngestError(f"{path}: is not a list of what was not carried")
    said = [NotCarried.from_dict(entry).in_language(args.lexicon).to_dict() for entry in raw]
    print(json.dumps(said, sort_keys=True, ensure_ascii=False))
    return 0


def cmd_make_scope(args) -> int:
    """Write the scope of the sections the owner chose: each `--section` as its own paragraphs, each `--tree`
    with everything under it, `--all` every section, less each `--leave-out` block of what was chosen."""
    extraction = _read_extraction(args.extraction)
    made = scope_of(
        extraction, list(args.section or []), args.by, list(args.tree or []),
        leave_out=list(args.leave_out or []), everything=args.all,
    )
    try:
        pathlib.Path(args.out).write_text(made.to_json(), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{args.out}: cannot be written ({exc})") from exc
    whole = len(made.blocks) == len(extraction.blocks)
    if args.json:
        # `whole` says the choice is every block, which is no scope: the specs folder refuses one, and a
        # host leaves it out (and leaves `--scope` off the tools) rather than work out the rule itself.
        print(json.dumps(
            {"blocks": len(made.blocks), "of": len(extraction.blocks), "whole": whole, "revision": made.revision()},
            sort_keys=True))
    else:
        print(f"scope: {len(made.blocks)} of {len(extraction.blocks)} block(s), revision {made.revision()} -> {args.out}"
              f"{' (every block, which is no scope)' if whole else ''}")
    return 0


def _read_ledger(path: str) -> Ledger:
    try:
        return Ledger.from_json(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError) as exc:
        raise IngestError(f"{path}: not readable as a ledger ({exc})") from exc


def _ledger_summary(report) -> str:
    said = report.to_dict()
    return (
        f"{'complete' if said['complete'] else 'not complete'}: {said['in_specs']} of {said['words']} "
        f"paragraph(s) in specs, {said['left_out']} left out, {len(said['unplaced'])} unplaced, "
        f"{len(said['contradicting'])} both used and left out, {len(said['shared'])} shared, "
        f"{len(said['empty'])} spec(s) with nothing for them"
    )


def cmd_make_ledger(args) -> int:
    """Write the ledger of a plan: for each destination, the parts of the outline it was given."""
    extraction = _read_extraction(args.extraction)
    try:
        plan = json.loads(pathlib.Path(args.plan).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as exc:
        raise IngestError(f"{args.plan}: not readable as a plan ({exc})") from exc
    made = make_ledger(extraction, plan, args.by)
    try:
        pathlib.Path(args.out).write_text(made.to_json(), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{args.out}: cannot be written ({exc})") from exc
    report = check_ledger(extraction, made)
    if args.json:
        # Counts and ids only: the blocks hold the document's own words.
        print(json.dumps({"revision": made.revision(), **report.to_dict()}, sort_keys=True))
    else:
        print(f"ledger: {len(made.allotments)} allotment(s), revision {made.revision()} -> {args.out}")
        print(f"check: {_ledger_summary(report)}")
    return 0


def cmd_check_ledger(args) -> int:
    """SCE's count of a ledger against the extraction it names. The exit code says whether it is complete."""
    report = check_ledger(_read_extraction(args.extraction), _read_ledger(args.ledger))
    if args.json:
        print(json.dumps(report.to_dict(), sort_keys=True))
    else:
        print(f"check: {_ledger_summary(report)}")
        for block_id in report.unplaced:
            print(f"  ask: unplaced {block_id}")
        for block_id in report.contradicting:
            print(f"  ask: both used and left out {block_id}")
        for index in report.empty:
            print(f"  ask: a spec with nothing for it (allotment {index + 1})")
    return 0 if report.complete else 1


def cmd_ledger_scopes(args) -> int:
    """Write the scope each spec of a complete ledger is imported under, one file for each.

    A ledger that is not complete writes nothing and exits 1 with what it is missing. A spec given the whole
    extraction has no scope and no file (`scope` is null): that is no scope, and the specs folder refuses one.
    """
    extraction = _read_extraction(args.extraction)
    ledger = _read_ledger(args.ledger)
    report = check_ledger(extraction, ledger)
    if not report.complete:
        print(f"ledger: {_ledger_summary(report)}; no scope is written for it", file=sys.stderr)
        return 1
    out = pathlib.Path(args.out_dir)
    listing = []
    try:
        out.mkdir(parents=True, exist_ok=True)
        for number, (allotment, scope) in enumerate(scopes_of(extraction, ledger), 1):
            name = None
            if scope is not None:
                name = f"scope-{number:03d}.json"
                (out / name).write_text(scope.to_json(), encoding="utf-8")
            listing.append({"to": allotment.to.to_dict(), "blocks": len(allotment.blocks), "scope": name})
    except OSError as exc:
        raise IngestError(f"{out}: cannot be written ({exc})") from exc
    print(json.dumps({"ledger": ledger.revision(), "scopes": listing}, sort_keys=True))
    return 0


def cmd_carry_ledger(args) -> int:
    """Write the ledger a revised document gets from the old one, for what is the same, and say what is asked.

    The draft holds the paragraphs whose words did not change and nothing else, so the check of it counts the
    changed and the new ones as unplaced; what is gone is named with where it had been. Counts and ids only.
    """
    old = _read_extraction(args.old_extraction)
    new = _read_extraction(args.new_extraction)
    carried = carry_ledger(old, _read_ledger(args.old_ledger), new)
    try:
        pathlib.Path(args.out).write_text(carried.ledger.to_json(), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{args.out}: cannot be written ({exc})") from exc
    if args.json:
        print(json.dumps(carried.to_dict(), sort_keys=True))
    else:
        print(
            f"carried {carried.carried} paragraph(s) to {len(carried.ledger.allotments)} allotment(s) "
            f"-> {args.out}; {len(carried.changed)} changed, {len(carried.fresh)} new, {len(carried.gone)} gone"
        )
        for changed in carried.changed:
            print(f"  ask: changed {changed.block} (was {changed.from_block})")
        for block_id in carried.fresh:
            print(f"  ask: new {block_id}")
        for gone in carried.gone:
            print(f"  note: gone {gone.block}")
    return 0


def _destinations_of(path: str):
    try:
        raw = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as exc:
        raise IngestError(f"{path}: not readable as destinations ({exc})") from exc
    return offered_from(raw)


def cmd_prepare_division(args) -> int:
    """Write what a model is told to draw a division of an extraction, and the parts it is told of.

    `prompt.txt` is the whole text for the model, and `parts.json` keeps what only `finish-division` reads: the
    parts listed, the destinations offered and the extraction they are of. A host takes the prompt to the
    model it chose, writes the answer beside it (`answer.txt`) and runs `finish-division`. Nothing here
    reaches a model, and what is printed is counts: the document's words are in the prompt file and nowhere else.
    """
    extraction = _read_extraction(args.extraction)
    destinations = _destinations_of(args.destinations)
    prepared = prepare_division(extraction, destinations, args.by)
    out = pathlib.Path(args.out_dir)
    try:
        out.mkdir(parents=True, exist_ok=True)
        (out / "prompt.txt").write_text(prepared.prompt, encoding="utf-8")
        (out / "parts.json").write_text(json.dumps({
            "v": 1,
            "recipe": f"{DIVIDE_KIND} {DIVIDE_RECIPE_VERSION}",
            "extraction": extraction.revision(),
            "parts": list(prepared.parts),
            "destinations": [{"key": d.key, "kind": d.kind, "title": d.title} for d in destinations],
        }, sort_keys=True), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{out}: cannot be written ({exc})") from exc
    print(json.dumps({
        "recipe": f"{DIVIDE_KIND} {DIVIDE_RECIPE_VERSION}", "parts": len(prepared.parts),
        "characters": len(prepared.prompt), "destinations": len(destinations),
    }, sort_keys=True))
    return 0


def cmd_finish_division(args) -> int:
    """Read the model's answer to a prepared division, and say what it placed, what was not taken and what is left.

    Exit 0 when the answer could be read, even when some of its items were refused (they are named, by item and
    a typed reason, and the others are kept); exit 1 when no suggestion could be read at all. Ids and counts
    only: nothing of the document's words is printed.
    """
    extraction = _read_extraction(args.extraction)
    directory = pathlib.Path(args.dir)
    try:
        listing = json.loads((directory / "parts.json").read_text(encoding="utf-8"))
        named = listing["extraction"]
        parts = tuple(str(p) for p in listing["parts"])
        destinations = offered_from({"v": 1, "destinations": listing["destinations"]})
    except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError) as exc:
        raise IngestError(f"{directory / 'parts.json'}: not what `prepare-division` writes ({exc})") from exc
    if named != extraction.revision():
        raise IngestError("the division was prepared from another extraction than this one")
    try:
        answer = (directory / "answer.txt").read_text(encoding="utf-8")
    except FileNotFoundError:
        answer = None
    except (OSError, UnicodeDecodeError) as exc:
        raise IngestError(f"{directory / 'answer.txt'}: not readable ({exc})") from exc
    draft = read_division_answer(extraction, DividePrepared("", parts), destinations, answer)
    try:
        (directory / "draft.json").write_text(json.dumps(draft.to_dict(), sort_keys=True), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{directory}: cannot be written ({exc})") from exc
    print(json.dumps(draft.to_dict(), sort_keys=True))
    return 1 if draft.unreadable is not None else 0


def cmd_convert(args) -> int:
    """Write the Markdown of an extraction by rule, and say what SCE's check makes of it.

    For an original that carries its text. The check is run on what was just made, so a converter that
    ever wrote something its own check refuses is a failure here and not a conversion handed on.
    """
    extraction = _read_extraction(args.extraction)
    scope = _read_scope(args.scope)
    made = convert_by_rules(extraction, scope)
    report = check_conversion(extraction, made, scope)
    out = pathlib.Path(args.out)
    # Exactly the canonical text, as `extract` writes one: the file's hash is the conversion's revision.
    out.write_text(made.to_json(), encoding="utf-8")
    if args.markdown:
        pathlib.Path(args.markdown).write_text(made.markdown(), encoding="utf-8")
    print(f"conversion: {len(made.blocks)} block(s) by {made.converter.name} {made.converter.version}, "
          f"revision {made.revision()} -> {out}")
    print(f"check: {_summary(report)}")
    return 0 if report.complete else 1


def cmd_check_conversion(args) -> int:
    """SCE's count of a conversion against the extraction it names. The exit code says whether it is complete."""
    report = check_conversion(
        _read_extraction(args.extraction), _read_conversion(args.conversion), _read_scope(args.scope)
    )
    if args.json:
        # Ids and counts only: the blocks hold the document's own words.
        print(json.dumps(report.to_dict(), sort_keys=True))
    else:
        print(f"check: {_summary(report)}")
        for why, block_id in report.queue():
            print(f"  ask: {why} {block_id}")
    return 0 if report.complete else 1


def _unit_file(directory: pathlib.Path, n: int, part: str) -> pathlib.Path:
    return directory / f"unit-{n:04d}.{part}.txt"


def cmd_prepare_conversion(args) -> int:
    """Cut an extraction into the units a model is asked to convert, and write what it is told for each.

    One prompt file for each unit (`unit-0001.prompt.txt`), and a list of which blocks each unit holds,
    which only `finish-conversion` reads. A host takes each prompt to the model it chose, writes the
    answer beside it (`unit-0001.answer.txt`), and runs `finish-conversion`. Nothing here reaches a model.
    """
    extraction = _read_extraction(args.extraction)
    scope = _read_scope(args.scope)
    units = split_units(extraction, args.characters, scope)
    if not units:
        raise IngestError("the extraction has no block with words in it, so there is nothing to ask a model")
    out = pathlib.Path(args.out_dir)
    try:
        out.mkdir(parents=True, exist_ok=True)
        for unit in units:
            _unit_file(out, unit.n, "prompt").write_text(prompt_for(extraction, unit), encoding="utf-8")
        listing = {
            "v": 1,
            "recipe": f"{RECIPE_KIND} {RECIPE_VERSION}",
            "extraction": extraction.revision(),
            "scope": "" if scope is None else scope.revision(),
            "units": [{"n": unit.n, "blocks": list(unit.blocks)} for unit in units],
        }
        (out / "units.json").write_text(json.dumps(listing, sort_keys=True), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{out}: cannot be written ({exc})") from exc
    print(f"units: {len(units)} unit(s) of {sum(len(u.blocks) for u in units)} block(s), recipe {RECIPE_KIND} {RECIPE_VERSION} -> {out}")
    return 0


def cmd_finish_conversion(args) -> int:
    """Take the model's answers to the units, and write the conversion they make.

    A unit whose answer cannot be read is named, by its number and a typed reason, and the others are
    kept: the blocks of the one that failed are *missing* to the check, which is what they are. Exit 1
    when any unit failed, so a host asks that model again for it; nothing of the document's words is
    printed.
    """
    extraction = _read_extraction(args.extraction)
    directory = pathlib.Path(args.dir)
    try:
        listing = json.loads((directory / "units.json").read_text(encoding="utf-8"))
        wanted = [int(unit["n"]) for unit in listing["units"]]
        named = listing["extraction"]
    except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError) as exc:
        raise IngestError(f"{directory}: holds no units.json that `prepare-conversion` wrote ({exc})") from exc
    if named != extraction.revision():
        raise IngestError(
            f"{directory} was prepared for the extraction {str(named)[:12]}, not for this one "
            f"({extraction.revision()[:12]}): answers about other blocks are not put together"
        )
    answers: dict[int, object] = {}
    for n in wanted:
        path = _unit_file(directory, n, "answer")
        text = None
        if path.is_file():
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError) as exc:
                raise IngestError(f"{path}: not readable ({exc})") from exc
        answers[n] = parse_answer(n, text)
    made, failed = assemble_units(
        extraction, answers, Converter("model", args.converter_name, args.converter_version)  # type: ignore[arg-type]
    )
    try:
        pathlib.Path(args.out).write_text(made.to_json(), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{args.out}: cannot be written ({exc})") from exc
    report = {
        "units": len(wanted),
        "converted": [n for n in wanted if n not in {f.n for f in failed}],
        "failed": [{"n": f.n, "code": f.code} for f in failed],
        "revision": made.revision(),
    }
    if args.json:
        print(json.dumps(report, sort_keys=True))
    else:
        print(
            f"conversion: {len(made.blocks)} block(s) from {len(report['converted'])} of {len(wanted)} unit(s) "
            f"by {made.converter.name} {made.converter.version}, revision {made.revision()} -> {args.out}"
        )
        for f in failed:
            print(f"  unit {f.n} failed: {f.code}")
    return 1 if failed else 0


def cmd_prepare_reading(args) -> int:
    """Take the pictures the named figure blocks mark out of the document, and write what a model is told.

    For each picture that can be read, its file as it is in the document (`unit-0001.png`) and the text a
    model is given with it (`unit-0001.prompt.txt`); the form the answer is held to (`answer.schema.json`);
    and `units.json`, which says which figure and picture each unit is, by hash, and which pictures were not
    handed over, with a typed reason. A host gives each picture to a model that can see, writes its answer
    beside it (`unit-0001.answer.json`), and runs `finish-reading`. Nothing here reaches a model.
    """
    extraction = _read_extraction(args.extraction)
    scope = _read_scope(args.scope)
    handed, refused = picture_reading.units_for(extraction, list(args.block), pathlib.Path(args.document), scope)
    units = [h.unit for h in handed]
    out = pathlib.Path(args.out_dir)
    try:
        out.mkdir(parents=True, exist_ok=True)
        for h in handed:
            (out / h.unit.file_name()).write_bytes(h.data)
            (out / h.unit.prompt_name()).write_text(picture_reading.prompt_for(h.unit), encoding="utf-8")
        (out / "answer.schema.json").write_text(
            json.dumps(picture_reading.ANSWER_SCHEMA, sort_keys=True), encoding="utf-8")
        listing = {
            "v": 1,
            "recipe": picture_reading.RECIPE_NAME,
            "extraction": extraction.revision(),
            "scope": "" if scope is None else scope.revision(),
            "units": [unit.to_dict() for unit in units],
            "refused": [r.to_dict() for r in refused],
        }
        (out / "units.json").write_text(json.dumps(listing, sort_keys=True), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{out}: cannot be written ({exc})") from exc
    said = {"units": len(units), "refused": [r.to_dict() for r in refused]}
    if args.json:
        print(json.dumps(said, sort_keys=True))
    else:
        print(f"reading: {len(units)} picture(s) to read, {len(refused)} not handed over, "
              f"recipe {picture_reading.RECIPE_NAME} -> {out}")
        for r in refused:
            print(f"  {r.block}{'' if r.picture is None else ' ' + r.picture}: {r.code}")
    return 0


def cmd_finish_reading(args) -> int:
    """Add what a model read of each picture to the conversion by rule, after the mark it read.

    A unit whose answer cannot be taken is named, by its number and a typed reason, and its mark stays as it
    was; the others are added. The conversion written keeps one reading for each picture read: what was
    handed over, by its hash, and which runtime and model read it. Exit 1 when any unit failed, so a host
    can say so; nothing of the picture's words is printed.
    """
    extraction = _read_extraction(args.extraction)
    conversion = _read_conversion(args.conversion)
    if conversion.extraction != extraction.revision():
        raise IngestError(
            f"the conversion is of the extraction {conversion.extraction[:12]}, not of this one "
            f"({extraction.revision()[:12]}): a reading is not added to another document's conversion"
        )
    directory = pathlib.Path(args.dir)
    try:
        listing = json.loads((directory / "units.json").read_text(encoding="utf-8"))
        named = listing["extraction"]
        units = [picture_reading.Unit.from_dict(unit) for unit in listing["units"]]
    except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError) as exc:
        raise IngestError(f"{directory}: holds no units.json that `prepare-reading` wrote ({exc})") from exc
    if named != extraction.revision():
        raise IngestError(
            f"{directory} was prepared for the extraction {str(named)[:12]}, not for this one "
            f"({extraction.revision()[:12]}): pictures of another document are not added"
        )
    answers: dict[int, picture_reading.Answer] = {}
    failed: list[picture_reading.Refused] = []
    for unit in units:
        path = directory / unit.answer_name()
        text = None
        if path.is_file():
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError) as exc:
                raise IngestError(f"{path}: not readable ({exc})") from exc
        taken = picture_reading.parse_answer(unit.n, unit.block, text)
        if isinstance(taken, picture_reading.Refused):
            failed.append(taken)
        else:
            answers[unit.n] = taken
    made = picture_reading.with_readings(
        conversion, units, answers,
        picture_reading.Runtime(args.runtime, args.runtime_version, args.model),
    )
    try:
        pathlib.Path(args.out).write_text(made.to_json(), encoding="utf-8")
    except OSError as exc:
        raise IngestError(f"{args.out}: cannot be written ({exc})") from exc
    report = {
        "units": len(units),
        "read": [unit.n for unit in units if unit.n in answers and answers[unit.n].blocks],
        "nothing_to_read": [unit.n for unit in units if unit.n in answers and answers[unit.n].nothing_to_read],
        "failed": [f.to_dict() for f in failed],
        "revision": made.revision(),
    }
    if args.json:
        print(json.dumps(report, sort_keys=True))
    else:
        print(f"reading: {len(report['read'])} of {len(units)} picture(s) read by {args.runtime}, "
              f"revision {made.revision()} -> {args.out}")
        for f in failed:
            print(f"  unit {f.n} failed: {f.code}")
    return 1 if failed else 0


def cmd_assemble(args) -> int:
    """The text a review ends in: a conversion with the owner's decisions applied, and what it changes.

    The text goes to `--out` exactly as it is to be saved, so the SHA-256 of the file is the revision the
    specs folder keeps it under. What is printed is the ids, the counts and the lines the text changes:
    the lines are the document's own words, so they are printed only with `--json`, for the host that
    asked.
    """
    extraction = _read_extraction(args.extraction)
    conversion = _read_conversion(args.conversion)
    try:
        log = decisions_from_json(pathlib.Path(args.decisions).read_text(encoding="utf-8"))
        current = pathlib.Path(args.current).read_text(encoding="utf-8") if args.current else None
    except (OSError, UnicodeDecodeError) as exc:
        raise IngestError(f"{getattr(exc, 'filename', None) or 'a file'}: not readable ({exc})") from exc
    made = assemble(conversion, extraction, log)
    pathlib.Path(args.out).write_text(made.text, encoding="utf-8")
    report = report_of(made, current)
    if args.json:
        print(json.dumps(report, sort_keys=True))
    else:
        said = report["changes"]
        print(
            f"assembled from {made.decisions} decision(s): {len(made.kept)} kept, {len(made.corrected)} corrected, "
            f"{len(made.dropped)} dropped, {len(made.added)} added, revision {made.revision()} -> {args.out}"
        )
        print("changes: " + ("none" if said["unchanged"] else f"{said['added']} line(s) added, {said['removed']} removed"))
    return 0


def cmd_questions(args) -> int:
    pack = _pack(args)
    prose = load_prose([pathlib.Path(p) for p in args.prose])
    questions = ask(prose, pack.model, pack.conventions, pack.examples)
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            for q in questions:
                fh.write(json.dumps(q.as_dict(), ensure_ascii=False) + "\n")
    by_kind = collections.Counter(q.kind for q in questions)
    for kind, n in by_kind.most_common():
        print(f"  {kind:<22} {n}")
    print(f"  {'total':<22} {len(questions)}")
    # A class that swamps the others is visible here rather than averaged into
    # one number. A single total once hid a class that was wrong 41 times in 42.
    return 0


def cmd_pseudo(args) -> int:
    got = render_pseudo(pathlib.Path(args.binding),
                        pathlib.Path(args.codegen) if args.codegen else None,
                        pathlib.Path(args.deploy) if args.deploy else None,
                        args.shape, args.lexicon)
    if not got.produced:
        print(f"refused: {got.refusal}", file=sys.stderr)
        return 1
    # ⚠ The page goes to stdout unadorned, with no banner and no trailing
    # summary. It is text somebody reads and a caller may redirect, and a line
    # this tool added to be friendly is a line the reader has to know is not
    # the document.
    sys.stdout.write(got.text)
    return 0


def cmd_views(args) -> int:
    from .views import ViewError, build

    try:
        lines = build(pathlib.Path(args.document), pathlib.Path(args.out),
                      scenario_set=pathlib.Path(args.scenarios) if args.scenarios else None,
                      labels_path=pathlib.Path(args.labels) if args.labels else None,
                      codegen=pathlib.Path(args.codegen) if args.codegen else None,
                      lang=args.lang, simulator=not args.no_simulator, internal=args.show_internal)
    except ViewError as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 1
    for line in lines:
        print(line)
    return 0


def cmd_check(args) -> int:
    pack = _pack(args)
    prose = load_prose([pathlib.Path(p) for p in args.prose]) if args.prose else None
    cited, refused = cited_by_binding(
        pathlib.Path(args.binding),
        pathlib.Path(args.decisions) if args.decisions else None,
        pathlib.Path(args.profile) if args.profile else None)
    if refused:
        print(f"refused: {refused}", file=sys.stderr)
        return 1
    findings = check(pack, pathlib.Path(args.binding), prose, cited)
    # The product's own refusals (`product_findings`), so this command and the `check` tool say
    # the same of a document; `--codegen` names the generator, else `SCE_CODEGEN`, else this tree's.
    from_product, unasked = product_findings_of_binding(
        pathlib.Path(args.binding), pathlib.Path(args.codegen) if args.codegen else None)
    findings = [*findings, *from_product]
    for f in findings:
        print(f"  {f}")
    print(f"  {len(findings)} refusal(s)")
    for notice in (picture_notice(unnamed_pictures_of(pathlib.Path(args.binding), prose)), unasked):
        if notice:
            print(f"  {notice}")
    return 1 if findings else 0


def cmd_gaps(args) -> int:
    pack = _pack(args)
    prose = load_prose([pathlib.Path(p) for p in args.prose]) if args.prose else None
    result = run_verify(pack, pathlib.Path(args.binding),
                        pathlib.Path(args.codegen) if args.codegen else None,
                        args.backend)
    if not result.ran:
        # No run, no attribution: a gap list without it would present every
        # recorded guess as untested, which is a claim about cases nobody ran.
        print(result.refusal)
        return 1
    questions = (ask(prose, pack.model, pack.conventions, pack.examples)
                 if prose is not None else ())
    counterfactuals = None
    if args.counterfactual:
        codegen = pathlib.Path(args.codegen) if args.codegen else None
        counterfactuals = explore(
            pack, pathlib.Path(args.binding), result,
            lambda variant: run_verify(pack, variant, codegen, args.backend),
            args.max_runs)
    cited, refused = cited_by_binding(
        pathlib.Path(args.binding),
        pathlib.Path(args.decisions) if args.decisions else None,
        pathlib.Path(args.profile) if args.profile else None,
        pathlib.Path(args.codegen) if args.codegen else None)
    if refused:
        print(f"refused: {refused}", file=sys.stderr)
        return 1
    gaps = gap_report(result, pack, prose, questions, counterfactuals, cited)
    if args.ask_out:
        # Only the questions, one per line: nothing a case holds, so the file
        # can go to the author as it is (`gaps.ask_of_author` says why).
        with open(args.ask_out, "w", encoding="utf-8") as fh:
            for gap in gaps:
                if gap.ask:
                    fh.write(gap.ask + "\n")
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            for gap in gaps:
                fh.write(json.dumps(gap.as_dict(), ensure_ascii=False, default=str) + "\n")
    for gap in gaps:
        if gap.kind == "question":
            continue
        head = f"{gap.marker} " if gap.marker else ""
        print(f"  {gap.kind.upper():<10} {head}({gap.subject})")
        print(f"             {gap.reason}")
        for f, line, name in gap.where:
            print(f"             in the text: {f}:{line} ({name})")
        if gap.kind == "refuted":
            for case, address, want, got in gap.evidence:
                print(f"             case {case!r}: {address} expected {want!r}, "
                      f"the guess gave {got!r}")
        elif gap.kind == "implicated":
            for case, address, want, got, others in gap.evidence:
                print(f"             case {case!r}: {address} expected {want!r}, "
                      f"got {got!r}; decided together with {', '.join(others)}")
            print(f"             agreed in {gap.agreed} other case(s)")
        elif gap.kind == "held":
            print(f"             agreed in {len(gap.evidence)} case(s)")
        for line in counterfactual_lines(gap):
            print(f"             {line}")
        print(f"             -> {gap.fix}")
        if gap.ask:
            print(f"             -> ask its author {gap.ask}")
    # The prose's own questions are counted, not listed: there can be hundreds,
    # and `questions` is the command that lists them.
    counts = collections.Counter(g.kind for g in gaps)
    print("  " + " · ".join(f"{kind} {counts[kind]}" for kind in GAP_ORDER if counts[kind]))
    return 0


def cmd_verify(args) -> int:
    pack = _pack(args)
    result = run_verify(pack, pathlib.Path(args.binding),
                        pathlib.Path(args.codegen) if args.codegen else None,
                        args.backend, tuple(args.explain))
    if not result.ran:
        # The product's own refusal, whole. It names the placeholder and the
        # reason the author wrote beside it, and that reason is the message.
        print(result.refusal)
        return 1
    for case in result.results:
        if not case.judged:
            print(f"  ????  {case.name}: {case.refusal}")
            continue
        if case.failures:
            print(f"  FAIL  {case.name}")
            for address, want, got in case.failures:
                print(f"          {address}: expected {want!r}, got {got!r}")
        if case.shown:
            # Beside the verdict it explains, and printed for a passing case too: a case somebody
            # asked to see is shown whether or not it failed.
            print(f"  SHOW  {case.name}")
            for part in ("inputs", "happened", "returned", "kept"):
                values = case.shown.get(part) or {}
                print(f"          {part} ({len(values)}):")
                for key in sorted(values):
                    print(f"            {key} = {values[key]!r}")
        if case.unchecked:
            print(f"  ----  {case.name}: no rule of this binding writes "
                  f"{', '.join(case.unchecked)}")
        if case.unwritten:
            # Only reached beside a failure: on its own it makes the case
            # unjudged, and the refusal above already says it.
            print(f"  ....  {case.name}: this round wrote nothing at "
                  f"{', '.join(case.unwritten)}, which this binding writes")
        if case.undetermined and case.judged:
            print(f"  ~~~~  {case.name}: depends on an unresolved input, not "
                  f"judged at {', '.join(case.undetermined)}")
    for address, reason in sorted(result.refuted.items()):
        # ⚠ The most useful line in the report. The author wrote this value
        # down as a guess; a case has now refuted it, and that is a different
        # thing from the document being wrong.
        print(f"  ????  {address}: this value is marked as an ASSUMPTION and "
              f"a case refutes it.")
        print(f"          the author wrote: {reason}")
    if result.unbound:
        print(f"  the examples read {len(result.unbound)} address(es) the "
              f"binding never writes: {', '.join(result.unbound)}")
    # ⚠ PRINTED WHETHER OR NOT ANYTHING FAILED, and beside the count rather
    # than under it. A pass is a statement about the cases, and a reader who
    # is not told what the cases never looked at will finish the sentence
    # themselves in the generous direction.
    if result.unasserted:
        print(f"  {len(result.unasserted)} written position(s) no case "
              f"expects, so nothing was judged there: "
              f"{', '.join(result.unasserted)}")
    # ⚠ Printed whenever it is non-empty, beside the counts rather than under
    # them. These values were remembered BY THE RUN: the generated code takes
    # this round's inputs and nothing else, so in production the caller has to
    # keep them between calls. A pass that does not say so is a pass about a
    # system nobody has agreed to build yet.
    if result.host_memory:
        print(f"  {len(result.host_memory)} value(s) this run remembered for "
              f"the host, which the caller must keep between rounds: "
              f"{', '.join(result.host_memory)}")
    # ⚠ Printed whenever it is non-empty, beside the counts for the same
    # reason. No case can move these: a precondition the pack reads as a
    # constant is not in the document, so the pass above is conditional on
    # each reason below being true, and nothing in this run tested one.
    if result.assumed_preconditions:
        print(f"  {len(result.assumed_preconditions)} precondition(s) the "
              f"pack reads by assumption, which no case here can test:")
        for phrase, held in result.assumed_preconditions.items():
            print(f"          {phrase!r} as {held['expression']!r}: "
                  f"{held['reason']}")
    # ⚠ Printed whenever an input is unresolved, WITH what it cost. A declared
    # unknown is the honest answer and the run no longer punishes it -- but
    # an unknown that withholds every position must be exactly as visible as
    # one that withholds none, or `unresolved` becomes the way to make a hard
    # case disappear.
    if result.still_open:
        blocked = sum(1 for c in result.results if c.undetermined)
        if result.unresolved:
            # ⚠ Worded for both paths. A computation is run under each value
            # an open input could take; a statechart's cases share one run and
            # are judged only while an open event could not have acted. What
            # the two have in common is the claim, and the claim is the line.
            print(f"  {len(result.unresolved)} input(s) the binding leaves "
                  f"UNRESOLVED; a position is judged only where it cannot "
                  f"depend on them:")
            for name, why in result.unresolved.items():
                print(f"          {name}: {why}")
        if result.unresolved_outputs:
            # ⚠ The product's refusal named these and their reasons; that
            # feedback is the point of the marker, so it is repeated here
            # rather than lost with the refusal.
            print(f"  {len(result.unresolved_outputs)} output(s) the document "
                  f"leaves UNRESOLVED or computes from one; built with a "
                  f"placeholder for this run only, and never judged:")
            for name, why in result.unresolved_outputs.items():
                print(f"          {name}: {why}")
        if result.unaddressed_outputs:
            print(f"  {len(result.unaddressed_outputs)} output(s) the binding "
                  f"has no address for yet; any position no named rule "
                  f"writes may be theirs, and none of those is judged:")
            for name, why in result.unaddressed_outputs.items():
                print(f"          {name}: {why}")
        print(f"  {result.undetermined} position(s) in {blocked} case(s) "
              f"rest on them and were not judged")
    # ⚠ Unjudged is reported beside the other two and never folded into either.
    # Counting it as a pass claims a run that did not happen; counting it as a
    # failure blames a document for a case nobody could drive.
    # ⚠ The backend is on the same line as the counts, not in a footer. This
    # ran ONE lowering of the document, and most of this product ships as
    # another one -- a reader shown only "passed" will read it as a statement
    # about what they are about to ship.
    print(f"  {result.passed} passed, {result.failed} failed, "
          f"{result.unjudged} could not be judged "
          f"(the {result.backend} lowering)")
    if result.passed_short:
        print(f"  {result.passed_short} of the {result.passed} passed case(s) did not compare "
              f"every position the record expects: no rule of this binding writes one of them "
              f"(the `----` lines above). A component that leaves an output out passes these "
              f"and still fails the host's own tests.")
    # ⚠ An open value is not a failure of the document, and it is not a pass
    # either: a pipeline that gates on this status must not read "incomplete"
    # as "verified". Before open values were run rather than refused, the
    # refusal carried status 1 and that was the protection; running them
    # instead kept the counts honest and, for one change, let the status
    # become 0 -- so the status now says so explicitly.
    if result.still_open:
        print(f"  not a pass: {result.still_open} "
              f"value(s) are still unresolved, so the status is non-zero even "
              f"though {'nothing' if not result.failed else 'more than that'} "
              f"failed")
        return 1
    return 1 if result.failed else 0


def cmd_check_pack(args) -> int:
    """Everything wrong with a pack, in one pass.

    Every other command refuses at the first problem, so a pack with three
    mistakes took three round trips to learn about. A pack is prepared by the
    people who build it and handed to a specification owner already verified,
    so the builder needs all of them at once, and what could not be checked
    because something it depends on did not load.

    Exit 0 only for a pack with no problem and nothing skipped. A refusal to
    read the pack at all is not special-cased: it is the first problem listed."""
    report = check_pack(pathlib.Path(args.pack))
    for number, problem in enumerate(report.problems, 1):
        print(f"problem {number}: {problem}")
    for what in report.skipped:
        print(f"not checked: {what}")
    if report.clean:
        print(f"{report.root}: no problem found (the pack loads, and every rule it "
              f"states is held to what it declares)")
        return 0
    print(f"{report.root}: {len(report.problems)} problem(s), "
          f"{len(report.skipped)} check(s) not made")
    return 1


def cmd_review(args) -> int:
    """Numbers about the pack, and no verdict on it.

    ⚠ The exit status is 0 unless a shape that cannot be right was found.
    "The pack is correct" is not a claim anything here can make -- the
    platform it describes is not in this tree -- so the status says only
    whether one of the known-impossible shapes is present.
    """
    pack = _pack(args)
    prose = load_prose([pathlib.Path(p) for p in args.prose])
    got = run_review(pack, prose)
    print(f"addresses {got.addresses} ({got.outputs} output)"
          f" · one spelling only {got.single_spelling}"
          f" · never written in the prose {len(got.unmentioned)}")
    print(f"prose attributed {got.attribution:.0%}"
          f" over {got.blocks_built} block(s)"
          f" · {got.thin_blocks} of them one or two lines")
    print(f"examples {'present' if got.has_examples else 'ABSENT'}"
          f" · addresses they drive that the model lacks"
          f" {got.driven_undeclared}")
    if got.has_examples:
        total = got.asserted_outputs + len(got.unasserted_outputs)
        print(f"output positions any case expects {got.asserted_outputs}"
              f" of {total} · never expected {len(got.unasserted_outputs)}")
    for alarm in got.alarms():
        print(f"  ALARM: {alarm}")
    return 1 if got.alarms() else 0


def cmd_coverage(args) -> int:
    """What the whole set of documents reaches.

    ⚠ The exit status follows `review`'s: 0 unless a shape that cannot be
    right is present. An unwritten position is not one -- a conversion in
    progress looks exactly like that -- and alarming on it would be an alarm
    every unfinished piece of work trips. A position two documents write is
    one, whatever the platform turns out to be.
    """
    pack = _pack(args)
    prose = load_prose([pathlib.Path(p) for p in args.prose]) if args.prose else None
    got = run_coverage(pack, args.binding, prose)
    print(f"output positions {len(got.positions)}"
          f" · written by some document {got.covered}"
          f" · unwritten {len(got.unwritten)}"
          + (f" · unasked {len(got.unasked)}" if got.unasked else ""))
    for address in got.unwritten:
        print(f"  unwritten: {address}"
              + ("  (unasked: the specification does not name it and no case expects it)"
                 if address in got.unasked else ""))
    if got.undeclared:
        print(f"positions a binding writes that the model does not declare"
              f" {len(got.undeclared)} — `check` names them against theirs")
    for alarm in got.alarms():
        print(f"  ALARM: {alarm}")
    return 1 if got.alarms() else 0


def cmd_scaffold(args) -> int:
    """Write the half of a binding the interface model already decides."""
    pack = _pack(args)
    out = pathlib.Path(args.out)
    write_scaffold(pack, args.document, out, args.activation, args.kind)
    also = f" and the {args.kind} {out.parent / args.document}" if args.kind else ""
    print(f"wrote {out}{also}: {sum(len(e.fields) for e in pack.model.outputs())} output "
          f"position(s) and {sum(len(e.fields) for e in pack.model.entries if e.role != 'output')} "
          f"input position(s) from the model; run `check` next")
    return 0


def cmd_compare(args) -> int:
    """Several drafts of one specification, compared at every level.

    Exit 0 whenever the comparison ran: how far drafts part is data for the
    owner, not a failure of any of them."""
    report = run_compare([pathlib.Path(d) for d in args.document], args.codegen,
                         drives=args.drives, steps=args.steps)
    print(compare_summary(report))
    if args.out:
        pathlib.Path(args.out).write_text(
            json.dumps(report, indent=2, ensure_ascii=False, default=str) + "\n",
            encoding="utf-8")
    return 0


def cmd_revise_gate(args) -> int:
    """A revision held to the reach of what changed, by a reviser the caller names.

    The design is judged, and while it is outside the reach of what the specification changed the
    reviser command is run to put it right, up to `--rounds` times; the design is edited IN PLACE,
    so give a copy. Exit 0 only when the design as it stands is within reach; 1 when it is not (the
    rounds used up, or a revision that left the violations as they were); 2 when nothing could be
    judged, or the reviser failed its round."""
    try:
        delta = json.loads(pathlib.Path(args.delta).read_text(encoding="utf-8"))
    except ValueError as error:
        raise GateError(f"--delta is not JSON: {error}") from error
    record, root = pathlib.Path(args.record).resolve(), pathlib.Path(args.root).resolve()
    design = pathlib.Path(args.design).resolve()
    codegen = pathlib.Path(args.codegen) if args.codegen else None
    # The record is read once and every judgment is of the copy: a reviser that is told where the
    # record lies must not be able to move what the design is judged against.
    with tempfile.TemporaryDirectory() as directory:
        baseline = keep_baseline(record, pathlib.Path(directory))
        outcome = hold_revision(
            lambda: judge_revision(delta, baseline.copy, root, design, codegen=codegen),
            guarded(command_reviser(args.reviser, design, timeout_s=args.reviser_timeout), baseline),
            rounds=args.rounds)
    for held in outcome.rounds:
        listed = ", ".join(f"{id_} ({kind})" for id_, kind, _places in held.violations) or "none"
        print(f"round {held.number}: {held.verdict}; violations: {listed}")
    print(f"{outcome.status}: {outcome.reason}")
    if args.out:
        pathlib.Path(args.out).write_text(
            json.dumps(outcome.as_json(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    if outcome.within_reach:
        return 0
    return 1 if outcome.status in (OUTSIDE_REACH, STALLED) else 2


def cmd_decisions(args) -> int:
    """A draft held to the owner's decision record.

    Exit 1 when the draft guessed where no answer licenses it, or the
    product could not read it; a new question alone is not a failure, since
    the specification may have a gap nobody saw."""
    report, refused = hold_decisions(pathlib.Path(args.document),
                                     pathlib.Path(args.decisions), args.codegen)
    answer = json.loads(report or refused)
    if "findings" in answer:
        print(decisions_summary(answer))
    else:
        # The product's own refusal of the document, which names it.
        print(refused, end="")
    if args.out:
        pathlib.Path(args.out).write_text(report or refused, encoding="utf-8")
    return 1 if refused else 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(prog="sce_author", description=__doc__)
    sub = ap.add_subparsers(dest="command", required=True)

    def with_pack(p):
        p.add_argument("--pack", required=True, help="directory holding the interface model and conventions")
        return p

    b = with_pack(sub.add_parser("brief", help="assemble one page for whoever writes the document"))
    b.add_argument("--prose", required=True, nargs="+", help="one or more specification files")
    b.add_argument("--out", required=True, help="where to write the brief")
    b.set_defaults(fn=cmd_brief)

    pic = sub.add_parser("picture", help="write a picture the specification marks `[picture: NAME]` to a file")
    pic.add_argument("--prose", required=True, nargs="+", help="one or more specification files")
    pic.add_argument("--name", required=True, help="the NAME in the mark, e.g. image3.png")
    pic.add_argument("--out", required=True, help="where to write the picture")
    pic.add_argument("--json", action="store_true",
                     help="print its size, its format (null for one a client cannot show) and its SHA-256, as JSON")
    pic.set_defaults(fn=cmd_picture)

    ren = sub.add_parser(
        "render-pictures",
        help="draw the images of a PDF as PNG files, to be kept beside it as its renditions (needs the pdf pack)")
    ren.add_argument("--document", required=True, help="the PDF")
    ren.add_argument("--out-dir", required=True,
                     help="where to write the pictures and `renditions.json`, which lists them with their hashes")
    ren.set_defaults(fn=cmd_render_pictures)

    ext = sub.add_parser(
        "extract", help="read one file into blocks, and say what could not be carried (no pack needed)")
    ext.add_argument("file", help="the document: .md, .txt, .html, .docx, or a .pdf with a text layer")
    ext.add_argument("--out", required=True, help="where to write the extraction (JSON)")
    ext.set_defaults(fn=cmd_extract)

    conv = sub.add_parser(
        "convert", help="write the Markdown of an extraction by rule, and check it (no model, no pack)")
    conv.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    conv.add_argument("--out", required=True, help="where to write the conversion (JSON)")
    conv.add_argument("--markdown", help="also write the Markdown itself here")
    conv.add_argument("--scope", help="the scope (JSON) `make-scope` wrote: convert and check only that part")
    conv.set_defaults(fn=cmd_convert)

    out = sub.add_parser(
        "outline", help="list the parts of an extraction an owner can choose between (no pack, no model)")
    out.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    out.add_argument("--by", choices=OUTLINES,
                     help="divide by headings, pages or not at all (default: headings, else pages, else whole)")
    out.add_argument("--json", action="store_true", help="print the outline as JSON for a host")
    out.set_defaults(fn=cmd_outline)

    rn = sub.add_parser(
        "render-notes",
        help="say what could not be carried in a language, from each entry's code and facts (no pack, no model)")
    rn.add_argument("--notes", required=True,
                    help="a JSON list of entries (an extraction's or a report's `not_carried`)")
    rn.add_argument("--lexicon", required=True, choices=notes_languages,
                    help="the language the sentences are written in")
    rn.set_defaults(fn=cmd_render_notes)

    mks = sub.add_parser(
        "make-scope", help="write the scope of the sections an owner chose")
    mks.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    mks.add_argument("--section", action="append",
                     help="the id of a section `outline` listed, taken as its own paragraphs (repeat for each)")
    mks.add_argument("--tree", action="append",
                     help="the id of a section taken with every section under it (repeat for each)")
    mks.add_argument("--all", action="store_true", help="take every section, as a choice to leave blocks out of")
    mks.add_argument("--leave-out", action="append", dest="leave_out",
                     help="the id of a block of a chosen section not to take, such as a table, figure or "
                          "attachment `outline` listed as an item (repeat for each)")
    mks.add_argument("--by", choices=OUTLINES, help="the division `outline` was asked for, when it was asked")
    mks.add_argument("--out", required=True, help="where to write the scope (JSON)")
    mks.add_argument("--json", action="store_true", help="print the counts and whether it is every block, as JSON")
    mks.set_defaults(fn=cmd_make_scope)

    mkl = sub.add_parser(
        "make-ledger", help="write the ledger of a plan: the parts of the outline each destination was given")
    mkl.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    mkl.add_argument("--plan", required=True, help="the plan (JSON): for each destination, the part ids it was given")
    mkl.add_argument("--by", choices=OUTLINES, help="the division `outline` was asked for, when it was asked")
    mkl.add_argument("--out", required=True, help="where to write the ledger (JSON)")
    mkl.add_argument("--json", action="store_true", help="print the ledger's revision and its check, as JSON")
    mkl.set_defaults(fn=cmd_make_ledger)

    cdl = sub.add_parser(
        "check-ledger", help="count where the paragraphs went: unplaced, contradicting and shared (no model)")
    cdl.add_argument("--extraction", required=True, help="the extraction the ledger names")
    cdl.add_argument("--ledger", required=True, help="the ledger (JSON) to count")
    cdl.add_argument("--json", action="store_true", help="print the counts and ids as JSON")
    cdl.set_defaults(fn=cmd_check_ledger)

    lsc = sub.add_parser(
        "ledger-scopes", help="write the scope each spec of a complete ledger is imported under")
    lsc.add_argument("--extraction", required=True, help="the extraction the ledger names")
    lsc.add_argument("--ledger", required=True, help="the ledger (JSON); it has to be complete")
    lsc.add_argument("--out-dir", required=True, help="where to write one scope file for each spec")
    lsc.set_defaults(fn=cmd_ledger_scopes)

    cal = sub.add_parser(
        "carry-ledger",
        help="carry the ledger of a document to its revision by what is the same, and say what is asked (no model)")
    cal.add_argument("--old-extraction", required=True, help="the extraction the old ledger names")
    cal.add_argument("--old-ledger", required=True, help="the ledger (JSON) of the old document")
    cal.add_argument("--new-extraction", required=True, help="the extraction of the revised document")
    cal.add_argument("--out", required=True, help="where to write the draft ledger (JSON) of the revised document")
    cal.add_argument("--json", action="store_true", help="print what was carried and what is asked, as JSON")
    cal.set_defaults(fn=cmd_carry_ledger)

    pdv = sub.add_parser(
        "prepare-division",
        help="write what a model is told to draw a division of a document among specs (no model)")
    pdv.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    pdv.add_argument("--destinations", required=True,
                     help="the destinations (JSON) the owner has: `{\"v\": 1, \"destinations\": [{key, kind, title}]}`")
    pdv.add_argument("--by", choices=OUTLINES, help="the division `outline` was asked for, when it was asked")
    pdv.add_argument("--out-dir", required=True, help="where to write `prompt.txt` and `parts.json`")
    pdv.set_defaults(fn=cmd_prepare_division)

    fdv = sub.add_parser(
        "finish-division", help="read a model's answer to a prepared division as suggestions (no model)")
    fdv.add_argument("--extraction", required=True, help="the extraction the division was prepared from")
    fdv.add_argument("--dir", required=True, help="the folder `prepare-division` wrote, with `answer.txt` beside the prompt")
    fdv.set_defaults(fn=cmd_finish_division)

    chk = sub.add_parser(
        "check-conversion", help="count what a conversion matched, left out and invented, against its original")
    chk.add_argument("--extraction", required=True, help="the extraction the conversion names")
    chk.add_argument("--conversion", required=True, help="the conversion (JSON) to hold to it")
    chk.add_argument("--json", action="store_true", help="print the counts and ids as JSON")
    chk.add_argument("--scope", help="the scope (JSON) the import was made under: count only that part")
    chk.set_defaults(fn=cmd_check_conversion)

    prep = sub.add_parser(
        "prepare-conversion", help="cut an extraction into the units a model converts, and write what it is told")
    prep.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    prep.add_argument("--out-dir", required=True, help="where to write one prompt for each unit")
    prep.add_argument("--characters", type=int, default=UNIT_CHARACTERS,
                      help="how many characters of words a unit holds (a model with a small context wants fewer)")
    prep.add_argument("--scope", help="the scope (JSON) `make-scope` wrote: only that part is given to a model")
    prep.set_defaults(fn=cmd_prepare_conversion)

    fin = sub.add_parser(
        "finish-conversion", help="put a model's answers to the units together as a conversion")
    fin.add_argument("--extraction", required=True, help="the extraction the units were cut from")
    fin.add_argument("--dir", required=True, help="the folder `prepare-conversion` wrote, with an answer beside each prompt")
    fin.add_argument("--converter-name", required=True, help="what to call the model that answered")
    fin.add_argument("--converter-version", default="1", help="its version, as it reports one")
    fin.add_argument("--out", required=True, help="where to write the conversion (JSON)")
    fin.add_argument("--json", action="store_true", help="print which units were converted and which failed, as JSON")
    fin.set_defaults(fn=cmd_finish_conversion)

    prr = sub.add_parser(
        "prepare-reading",
        help="take the pictures figure blocks mark out of a document, and write what a model that can see is told")
    prr.add_argument("--extraction", required=True, help="the extraction (JSON) `extract` wrote")
    prr.add_argument("--document", required=True, help="the document the extraction was read from")
    prr.add_argument("--block", action="append", required=True,
                     help="the id of a figure block whose pictures are to be read (repeat for each)")
    prr.add_argument("--scope", help="the scope (JSON) `make-scope` wrote: a figure outside it is not handed over")
    prr.add_argument("--out-dir", required=True, help="where to write each picture and what a model is told")
    prr.add_argument("--json", action="store_true", help="print how many pictures are to be read and which not, as JSON")
    prr.set_defaults(fn=cmd_prepare_reading)

    fir = sub.add_parser(
        "finish-reading", help="add what a model read of each picture to the conversion, after its mark")
    fir.add_argument("--extraction", required=True, help="the extraction the pictures were taken from")
    fir.add_argument("--conversion", required=True, help="the conversion by rule the readings are added to")
    fir.add_argument("--dir", required=True, help="the folder `prepare-reading` wrote, with an answer beside each picture")
    fir.add_argument("--runtime", required=True, help="the runtime that read (claude-code, ...)")
    fir.add_argument("--runtime-version", default="", help="its version, as it reports one")
    fir.add_argument("--model", default="", help="the model it reported")
    fir.add_argument("--out", required=True, help="where to write the conversion with the readings (JSON)")
    fir.add_argument("--json", action="store_true", help="print which pictures were read and which failed, as JSON")
    fir.set_defaults(fn=cmd_finish_reading)

    asm = sub.add_parser(
        "assemble", help="apply the owner's decisions to a conversion: the text a review ends in")
    asm.add_argument("--extraction", required=True, help="the extraction the conversion names")
    asm.add_argument("--conversion", required=True, help="the conversion (JSON) the decisions are about")
    asm.add_argument("--decisions", required=True, help="the proposal's decisions: the JSON list `read_proposal` gives")
    asm.add_argument("--current", help="the spec's text now, to say what the assembled text changes in it")
    asm.add_argument("--out", required=True, help="where to write the text, exactly as it is to be saved")
    asm.add_argument("--json", action="store_true", help="print the ids, the counts and the changes as JSON")
    asm.set_defaults(fn=cmd_assemble)

    q = with_pack(sub.add_parser("questions", help="what the specification does not answer"))
    q.add_argument("--prose", required=True, nargs="+")
    q.add_argument("--out", help="write every question as NDJSON (the screen shows counts only)")
    q.set_defaults(fn=cmd_questions)

    cp = with_pack(sub.add_parser(
        "check-pack", help="list everything wrong with a pack at once, not just the first"))
    cp.set_defaults(fn=cmd_check_pack)

    r = with_pack(sub.add_parser(
        "review", help="measure the pack itself, which every other command trusts"))
    r.add_argument("--prose", required=True, nargs="+")
    r.set_defaults(fn=cmd_review)

    # ⚠ No `--pack`. Rendering consults the pack for nothing, and asking for
    # one would let a bad pack refuse a request that never needed it.
    s = sub.add_parser(
        "pseudo", help="show the written document the way a person reads it")
    s.add_argument("--binding", required=True,
                   help="the binding file, which names its own document")
    s.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    s.add_argument("--deploy",
                   help="a deployment descriptor; every line it decides is "
                        "shown too, each marked with a leading '!'")
    # ⚠ No `choices=`. What a shape or a lexicon may be called is the
    # product's registry to answer, and a list here would refuse a name the
    # product accepts the day one is registered. An unknown name comes back
    # as the product's own refusal, which names the real set.
    s.add_argument("--shape",
                   help="how lines and nesting are written: 'indent' (the "
                        "default) or 'endmark' (indentation plus closing "
                        "markers); layout only, never content")
    s.add_argument("--lexicon",
                   help="what the grammar's own words are called: 'en' (the "
                        "default) or 'ko'; what the document wrote is never "
                        "translated")
    s.set_defaults(fn=cmd_pseudo)

    # ⚠ No `--pack`: the views read the document and, optionally, what its owner wrote about it. The
    # words a reader sees come from `--labels` and `--scenarios` and from nowhere else.
    w = sub.add_parser(
        "views", help="show a statechart as a table, a timeline and a simulator, for a reader who is not a programmer")
    w.add_argument("--document", required=True, help="the statechart")
    w.add_argument("--out", required=True, help="the directory the pages are written into")
    w.add_argument("--scenarios",
                   help="a scenario set (sce-scenario-set.v1): each runnable example becomes a timeline, under the "
                        "specification's own sentence")
    w.add_argument("--labels",
                   help="plain words for the document's names (JSON: title, states, events, outputs, lanes, data); "
                        "what is left out is shown as the document wrote it")
    w.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    w.add_argument("--lang", choices=("ko", "en"), default="ko", help="the words the pages use for themselves")
    w.add_argument("--show-internal", action="store_true",
                   help="show what the machine says to itself (`<raise>`) in the tables; left out by default, "
                        "because it is in every cell and buries the one fact each cell is for")
    w.add_argument("--no-simulator", action="store_true",
                   help="skip the simulator, which walks the engine through every situation and can take minutes")
    w.set_defaults(fn=cmd_views)

    # ⚠ No `--pack` either: the drafts are compared with one another and
    # with nothing outside them.
    m = sub.add_parser(
        "compare", help="several drafts of one specification: where they agree and part")
    m.add_argument("--document", required=True, action="append",
                   help="a draft; give two or more")
    m.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    m.add_argument("--drives", type=int, default=DRIVES,
                   help=f"seeded random drives per draft (default {DRIVES}); "
                        f"the behaviour verdict states it")
    m.add_argument("--steps", type=int, default=STEPS,
                   help=f"steps per drive (default {STEPS})")
    m.add_argument("--out", help="write the whole report as JSON as well")
    m.set_defaults(fn=cmd_compare)

    # ⚠ No `--pack`: the record is the owner's answers, and a draft is held
    # to them whatever pack it was written with, or none.
    d = sub.add_parser(
        "decisions", help="hold a draft to the owner's decision record")
    d.add_argument("--document", required=True, help="the draft")
    d.add_argument("--decisions", required=True,
                   help="the owner's decision record (decisions.v1 JSON)")
    d.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    d.add_argument("--out", help="write the whole report as JSON as well")
    d.set_defaults(fn=cmd_decisions)

    # ⚠ No `--pack` either, and no default for the reviser: which client revises, and with what
    # words, is the caller's. The command is run without a shell and told of the design and of the
    # request by environment (`SCE_REVISION_DESIGN`, `SCE_REVISION_REQUEST`).
    g = sub.add_parser(
        "revise-gate",
        help="hold a revised design to the reach of what the specification changed, revising it "
             "again until it is within reach or the rounds are used up")
    g.add_argument("--record", required=True, help="the acceptance record scxml_accept wrote")
    g.add_argument("--root", required=True, help="the directory the record's paths are relative to")
    g.add_argument("--delta", required=True,
                   help="the `delta` of the revised requirement list, as JSON")
    g.add_argument("--design", required=True,
                   help="the revised draft; the reviser edits it in place, so give a copy")
    g.add_argument("--reviser", required=True,
                   help="the command that revises the design it is told of")
    g.add_argument("--rounds", type=int, default=DEFAULT_ROUNDS,
                   help=f"how many times the reviser may be asked (default {DEFAULT_ROUNDS}, "
                        f"at most {MAX_ROUNDS})")
    g.add_argument("--reviser-timeout", type=int, default=REVISER_TIMEOUT_S,
                   help=f"seconds one round may take (default {REVISER_TIMEOUT_S})")
    g.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    g.add_argument("--out", help="write how the gate ended, as JSON as well")
    g.set_defaults(fn=cmd_revise_gate)

    c = with_pack(sub.add_parser("check", help="judge a written document against the model"))
    c.add_argument("--binding", required=True, help="the binding file, which names its own document")
    c.add_argument("--prose", nargs="+",
                   help="the specification: also refuse a precondition it states that nothing reads")
    c.add_argument("--decisions",
                   help="the owner's decision record: a mark citing one of its ids may sit on any element")
    c.add_argument("--profile",
                   help="the owner's authoring profile: a mark citing one of its house rules may sit on any element")
    c.add_argument("--codegen",
                   help="the product's code generator, whose own refusals of the document (its "
                        "structure and its design-time lints) are added to these "
                        "(default: SCE_CODEGEN, else the one in this tree)")
    c.set_defaults(fn=cmd_check)

    v = with_pack(sub.add_parser(
        "verify", help="run the pack's examples against the written document"))
    v.add_argument("--binding", required=True,
                   help="the binding file, which names its own document")
    v.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    v.add_argument("--backend", default="python",
                   help="which lowering of the document to DRIVE. The product "
                        "emits six; this drives the one it can import, and "
                        "refuses the rest rather than reporting on a program "
                        "nobody started")
    v.add_argument("--explain", action="append", default=[], metavar="TEXT",
                   help="also SHOW every case whose name contains TEXT: the inputs the "
                        "binding handed the document, every value it returned that round, "
                        "and what it kept. Judging is unchanged. May be repeated")
    v.set_defaults(fn=cmd_verify)

    g = with_pack(sub.add_parser(
        "gaps", help="where the specification is silent, and what the run found there"))
    g.add_argument("--binding", required=True,
                   help="the binding file, which names its own document")
    g.add_argument("--codegen",
                   help="the product's code generator (default: the one in this tree)")
    g.add_argument("--backend", default="python", help="as for verify")
    g.add_argument("--prose", nargs="+",
                   help="the specification: say where each gap sits in it, and "
                        "add what it leaves open before anything is run")
    g.add_argument("--out", help="write every gap as NDJSON as well")
    g.add_argument("--ask-out",
                   help="write only what to ask the authors -- one question per "
                        "document guess that lists no candidates, failing or "
                        "not -- with nothing a case holds, so it can be handed over")
    g.add_argument("--counterfactual", action="store_true",
                   help="change each binding guess a failure implicates to every "
                        "alternative it has and run the cases again, to find "
                        "which guesses the failures actually rest on")
    g.add_argument("--max-runs", type=int, default=MAX_RUNS,
                   help=f"runs --counterfactual may spend (default {MAX_RUNS}); "
                        f"what it could not try is reported")
    g.add_argument("--decisions",
                   help="the owner's decision record: a mark citing one of its ids is the owner's "
                        "answer applied, not a guess")
    g.add_argument("--profile",
                   help="the owner's authoring profile: a mark citing one of its house rules is "
                        "the owner's answer applied, not a guess")
    g.set_defaults(fn=cmd_gaps)

    o = with_pack(sub.add_parser(
        "coverage", help="what the set of documents reaches, and what it does not"))
    o.add_argument("--binding", required=True, nargs="+",
                   help="every binding in the subject matter")
    o.add_argument("--prose", nargs="+",
                   help="the specification files: with them, an unwritten position that the "
                        "specification does not name and no case expects is reported as unasked")
    o.set_defaults(fn=cmd_coverage)

    f = with_pack(sub.add_parser(
        "scaffold", help="write a binding skeleton from the interface model"))
    f.add_argument("--document", required=True,
                   help="the document the binding will name, as the binding writes it")
    f.add_argument("--out", required=True,
                   help="the binding file to create; refused if it exists")
    # ⚠ No default. When the host runs the document is a fact about the
    # deployment, and a default here would be a guess made on its behalf.
    f.add_argument("--activation", choices=("on-change", "periodic"),
                   help="when the host runs the document, if it is known")
    # ⚠ No default either: whether the component is a transform is a reading
    # of the specification.
    f.add_argument("--kind", choices=KINDS,
                   help="also write the document's skeleton, as this kind, beside the binding")
    f.set_defaults(fn=cmd_scaffold)

    args = ap.parse_args(argv)
    # ⚠ ONE catch, and it is the base type rather than a list. Catching
    # `PackError` alone let a document that could not be read reach the caller
    # as a traceback, which is the same failure in a different file. The
    # OSError arm covers writing, which no refusal type owns: a brief whose
    # `--out` cannot be opened is the caller's mistake, not a defect.
    try:
        return args.fn(args)
    except AuthoringError as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 2
    except OSError as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
