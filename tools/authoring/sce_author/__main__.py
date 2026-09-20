"""Command line for the authoring core.

Every input is named on the command line. The core opens no file it was not
given, which is the structural half of the domain-free guarantee: a subject
matter has no route in except through a pack the caller chose.
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys

from .brief import write as write_brief
from .check import check
from .coverage import coverage as run_coverage
from .errors import AuthoringError
from .pack import load_pack
from .prose import load_prose
from .pseudo import render as render_pseudo
from .questions import ask
from .review import review as run_review
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


def cmd_check(args) -> int:
    pack = _pack(args)
    findings = check(pack, pathlib.Path(args.binding))
    for f in findings:
        print(f"  {f}")
    print(f"  {len(findings)} refusal(s)")
    return 1 if findings else 0


def cmd_verify(args) -> int:
    pack = _pack(args)
    result = run_verify(pack, pathlib.Path(args.binding),
                        pathlib.Path(args.codegen) if args.codegen else None,
                        args.backend)
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
        if case.unchecked:
            print(f"  ----  {case.name}: nothing written at "
                  f"{', '.join(case.unchecked)}")
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
    return 1 if result.failed else 0


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
    got = run_coverage(pack, args.binding)
    print(f"output positions {len(got.positions)}"
          f" · written by some document {got.covered}"
          f" · unwritten {len(got.unwritten)}")
    for address in got.unwritten:
        print(f"  unwritten: {address}")
    if got.undeclared:
        print(f"positions a binding writes that the model does not declare"
              f" {len(got.undeclared)} — `check` names them against theirs")
    for alarm in got.alarms():
        print(f"  ALARM: {alarm}")
    return 1 if got.alarms() else 0


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

    q = with_pack(sub.add_parser("questions", help="what the specification does not answer"))
    q.add_argument("--prose", required=True, nargs="+")
    q.add_argument("--out", help="write every question as NDJSON (the screen shows counts only)")
    q.set_defaults(fn=cmd_questions)

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
                        "default) or 'endmark'; layout only, never content")
    s.add_argument("--lexicon",
                   help="what the grammar's own words are called: 'en' (the "
                        "default) or 'ko'; what the document wrote is never "
                        "translated")
    s.set_defaults(fn=cmd_pseudo)

    c = with_pack(sub.add_parser("check", help="judge a written document against the model"))
    c.add_argument("--binding", required=True, help="the binding file, which names its own document")
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
    v.set_defaults(fn=cmd_verify)

    o = with_pack(sub.add_parser(
        "coverage", help="what the set of documents reaches, and what it does not"))
    o.add_argument("--binding", required=True, nargs="+",
                   help="every binding in the subject matter")
    o.set_defaults(fn=cmd_coverage)

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
