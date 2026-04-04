#!/usr/bin/env python3
"""
N-Way Cross-Validator for SCXML Code Generation

Auto-detects all registered language generators and validates that they
produce structurally equivalent output for all W3C SCXML tests.

Design:
  - SCXMLModel is the canonical reference (language-agnostic)
  - Generators are auto-discovered via generators.supported_languages()
  - Any new language added to the registry is automatically validated

Usage:
  python3 tools/cross_validator.py                  # All languages, all tests
  python3 tools/cross_validator.py --test 144       # Single test
  python3 tools/cross_validator.py --lang cpp,kotlin # Specific languages
  python3 tools/cross_validator.py --json           # JSON output
  python3 tools/cross_validator.py -v               # Verbose
"""

import argparse
import contextlib
import io
import json
import re
import sys
import tempfile
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Dict, List, Optional, Set

# Add codegen to path
TOOLS_DIR = Path(__file__).resolve().parent
CODEGEN_DIR = TOOLS_DIR / "codegen"
sys.path.insert(0, str(CODEGEN_DIR))

from generators import get_generator, supported_languages
from scxml_parser import SCXMLParser

PROJECT_ROOT = TOOLS_DIR.parent
RESOURCES_DIR = PROJECT_ROOT / "resources"

# JUnit XML test result directories per language
_TEST_RESULT_DIRS = {
    "kotlin": PROJECT_ROOT / "sce-kotlin-tests" / "build" / "test-results" / "test",
}


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class CanonicalInfo:
    """Canonical structural info extracted from SCXMLModel (language-agnostic)."""
    states: List[str] = field(default_factory=list)
    events: List[str] = field(default_factory=list)
    initial_state: str = ""
    needs_script_engine: bool = False
    needs_http_send: bool = False
    has_parallel: bool = False
    has_history: bool = False


@dataclass
class LangResult:
    """Code generation result for a single language on a single test."""
    success: bool = False
    error: str = ""


@dataclass
class TestComparison:
    """Comparison result for a single test across all languages."""
    test_id: str  # e.g. "144", "403a"
    canonical: CanonicalInfo = field(default_factory=CanonicalInfo)
    results: Dict[str, LangResult] = field(default_factory=dict)
    test_results: Dict[str, str] = field(default_factory=dict)  # lang -> PASS/FAIL/SKIP
    all_generate: bool = False


@dataclass
class ValidationReport:
    """Full cross-validation report."""
    languages: List[str] = field(default_factory=list)
    total_tests: int = 0
    all_generate_ok: int = 0
    per_language: Dict[str, Dict] = field(default_factory=dict)
    generation_mismatches: List[Dict] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Canonical model extraction
# ---------------------------------------------------------------------------

# Infrastructure events added by generators, not from SCXML source
_INFRA_EVENT_PREFIXES = (
    "error.", "cancel.invoke", "done.invoke", "done.state.", "Wildcard",
)


def _is_infra_event(event_name: str) -> bool:
    """Check if an event is infrastructure (not user-defined in SCXML)."""
    return any(event_name.startswith(p) or event_name == p
               for p in _INFRA_EVENT_PREFIXES)


def extract_canonical(scxml_path: str) -> CanonicalInfo:
    """
    Extract canonical structural info from SCXML via SCXMLParser.

    This is the single source of truth -- all languages should agree
    with this model if their generation succeeds.
    """
    parser = SCXMLParser()
    model = parser.parse_file(scxml_path)

    info = CanonicalInfo()

    # States: capitalize first letter to match generator output convention
    info.states = sorted(
        s_id[0].upper() + s_id[1:] if s_id else s_id
        for s_id in model.states.keys()
    )

    # Events: filter infrastructure events, normalize to capitalized form
    user_events = [e for e in model.events if not _is_infra_event(e)]
    info.events = sorted(user_events)

    # Initial state
    if model.initial:
        init = model.initial
        info.initial_state = init[0].upper() + init[1:] if init else ""

    # Feature flags
    info.needs_script_engine = model.needs_script_engine
    info.needs_http_send = model.needs_http_send
    info.has_parallel = model.has_parallel_states
    info.has_history = model.has_history_states

    return info


# ---------------------------------------------------------------------------
# JUnit XML test result parsing
# ---------------------------------------------------------------------------

def parse_test_results(language: str) -> Dict[str, str]:
    """Parse JUnit XML test results for a language. Returns {test_id: status}."""
    results_dir = _TEST_RESULT_DIRS.get(language)
    if not results_dir or not results_dir.exists():
        return {}

    results = {}
    for xml_file in results_dir.glob("TEST-*.xml"):
        match = re.search(r"Test(\d+[a-z]?)\.xml$", xml_file.name)
        if not match:
            continue
        test_id = match.group(1)  # e.g. "144", "403a"

        try:
            tree = ET.parse(xml_file)
            root = tree.getroot()
            failures = int(root.get("failures", "0"))
            errors = int(root.get("errors", "0"))
            skipped = int(root.get("skipped", "0"))

            if skipped > 0:
                results[test_id] = "SKIP"
            elif failures > 0:
                results[test_id] = "FAIL"
            elif errors > 0:
                results[test_id] = "ERROR"
            else:
                results[test_id] = "PASS"
        except ET.ParseError:
            results[test_id] = "ERROR"

    return results


# ---------------------------------------------------------------------------
# Core comparison
# ---------------------------------------------------------------------------

def _find_scxml(test_id: str) -> Optional[Path]:
    """Locate SCXML file for a test ID (handles variants like 403a)."""
    # Standard: resources/144/test144.scxml
    p = RESOURCES_DIR / test_id / f"test{test_id}.scxml"
    if p.exists():
        return p
    # Variant: resources/403/test403a.scxml (numeric prefix directory)
    base_num = re.match(r"(\d+)", test_id)
    if base_num:
        p = RESOURCES_DIR / base_num.group(1) / f"test{test_id}.scxml"
        if p.exists():
            return p
    return None


def compare_test(test_id: str, generators: dict, tmp_dir: Path,
                 test_results: Dict[str, Dict[str, str]]) -> TestComparison:
    """Run all generators on a test and compare success/failure."""
    comp = TestComparison(test_id=test_id)

    scxml_path = _find_scxml(test_id)
    if not scxml_path:
        return comp

    # Extract canonical model
    try:
        comp.canonical = extract_canonical(str(scxml_path))
    except Exception:
        pass

    # Test results per language
    for lang, lang_results in test_results.items():
        comp.test_results[lang] = lang_results.get(test_id, "UNKNOWN")

    # Generate with each language
    _devnull = io.StringIO()
    stem = scxml_path.stem  # e.g. "test403a"
    for lang, gen in generators.items():
        lang_dir = tmp_dir / f"{lang}_{test_id}"
        lang_dir.mkdir(exist_ok=True)
        result = LangResult()
        try:
            with contextlib.redirect_stdout(_devnull):
                ok = gen.generate(str(scxml_path.resolve()), str(lang_dir))
            if ok:
                result.success = True
            else:
                result.error = "Generation returned False"
        except Exception as e:
            result.error = str(e)
        comp.results[lang] = result

    # Check if all languages agree
    successes = {lang for lang, r in comp.results.items() if r.success}
    failures = {lang for lang, r in comp.results.items() if not r.success}
    comp.all_generate = len(failures) == 0

    return comp


# ---------------------------------------------------------------------------
# Discovery
# ---------------------------------------------------------------------------

def discover_test_ids() -> List[str]:
    """Discover all test IDs from resources directory, including variants (e.g. 403a)."""
    test_ids: Set[str] = set()
    if not RESOURCES_DIR.exists():
        return []
    for entry in RESOURCES_DIR.iterdir():
        if not entry.is_dir():
            continue
        # Find all test*.scxml in this directory
        for scxml in entry.glob("test*.scxml"):
            # Extract test ID from filename: test144.scxml -> "144", test403a.scxml -> "403a"
            m = re.match(r"test(\d+[a-z]?)\.scxml$", scxml.name)
            if m:
                test_ids.add(m.group(1))
    # Sort: numeric first, then variants (144, 145, ..., 403a, 403b, 403c, ...)
    return sorted(test_ids, key=lambda x: (int(re.match(r"(\d+)", x).group(1)), x))


# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

def build_report(comparisons: List[TestComparison],
                 languages: List[str]) -> ValidationReport:
    """Aggregate comparisons into a validation report."""
    report = ValidationReport(
        languages=languages,
        total_tests=len(comparisons),
    )

    # Per-language stats
    for lang in languages:
        report.per_language[lang] = {
            "generate_ok": 0,
            "generate_fail": 0,
            "test_pass": 0,
            "test_fail": 0,
            "test_skip": 0,
            "test_unknown": 0,
        }

    for comp in comparisons:
        if comp.all_generate:
            report.all_generate_ok += 1

        # Per-language generation stats
        for lang in languages:
            r = comp.results.get(lang)
            if r and r.success:
                report.per_language[lang]["generate_ok"] += 1
            else:
                report.per_language[lang]["generate_fail"] += 1

            # Test results
            tr = comp.test_results.get(lang, "UNKNOWN")
            if tr == "PASS":
                report.per_language[lang]["test_pass"] += 1
            elif tr == "FAIL":
                report.per_language[lang]["test_fail"] += 1
            elif tr == "SKIP":
                report.per_language[lang]["test_skip"] += 1
            else:
                report.per_language[lang]["test_unknown"] += 1

        # Generation mismatches: some languages succeed, others fail
        successes = [l for l in languages if comp.results.get(l, LangResult()).success]
        failures = [l for l in languages if not comp.results.get(l, LangResult()).success]
        if successes and failures:
            errors = {
                l: comp.results[l].error
                for l in failures if comp.results.get(l)
            }
            report.generation_mismatches.append({
                "test_id": comp.test_id,
                "succeed": successes,
                "fail": failures,
                "errors": errors,
            })

    return report


def print_report(report: ValidationReport, comparisons: List[TestComparison],
                 verbose: bool = False):
    """Print human-readable report to stdout."""
    langs = report.languages
    total = report.total_tests

    print("=" * 70)
    print(f"  SCXML Cross-Validation Report ({', '.join(langs)})")
    print("=" * 70)
    print()

    # Per-language generation summary
    print("--- Code Generation ---")
    print(f"  {'Language':<12} {'OK':>6} {'Fail':>6} {'Rate':>8}")
    for lang in langs:
        s = report.per_language[lang]
        ok = s["generate_ok"]
        fail = s["generate_fail"]
        rate = f"{ok / total * 100:.1f}%" if total > 0 else "N/A"
        print(f"  {lang:<12} {ok:>6} {fail:>6} {rate:>8}")
    print()

    # Cross-language parity
    print("--- Cross-Language Parity ---")
    print(f"  Total SCXML tests:     {total}")
    print(f"  All languages agree:   {report.all_generate_ok}/{total}")
    mismatches = len(report.generation_mismatches)
    if mismatches > 0:
        print(f"  Generation mismatches: {mismatches}")
    print()

    # Per-language test results (if available)
    has_test_results = any(
        s["test_pass"] + s["test_fail"] + s["test_skip"] > 0
        for s in report.per_language.values()
    )
    if has_test_results:
        print("--- Test Results ---")
        print(f"  {'Language':<12} {'PASS':>6} {'FAIL':>6} {'SKIP':>6}")
        for lang in langs:
            s = report.per_language[lang]
            tested = s["test_pass"] + s["test_fail"] + s["test_skip"]
            if tested > 0:
                print(f"  {lang:<12} {s['test_pass']:>6} {s['test_fail']:>6} {s['test_skip']:>6}")
        print()

    # Verdict
    all_ok = report.all_generate_ok == total
    any_test_fail = any(
        s["test_fail"] > 0 for s in report.per_language.values()
    )
    print("--- Verdict ---")
    if all_ok and not any_test_fail:
        print(f"  PRODUCTION READY: {total}/{total} full parity across {len(langs)} languages")
    elif mismatches <= total * 0.05 and not any_test_fail:
        pct = report.all_generate_ok / total * 100 if total > 0 else 0
        print(f"  NEAR PRODUCTION READY ({pct:.1f}% parity)")
    else:
        pct = report.all_generate_ok / total * 100 if total > 0 else 0
        print(f"  NEEDS WORK ({pct:.1f}% parity)")
    print()

    # Mismatches detail
    if report.generation_mismatches:
        print(f"--- Generation Mismatches ({mismatches}) ---")
        for m in report.generation_mismatches:
            print(f"\n  Test {m['test_id']}")
            print(f"    Succeed: {', '.join(m['succeed'])}")
            print(f"    Fail:    {', '.join(m['fail'])}")
            for lang, err in m["errors"].items():
                print(f"      {lang}: {err}")
        print()

    # Verbose: all tests
    if verbose:
        print("--- All Tests ---")
        for c in comparisons:
            parts = []
            for lang in langs:
                r = c.results.get(lang, LangResult())
                tr = c.test_results.get(lang, "")
                status = "OK" if r.success else "FAIL"
                tr_str = f"/{tr}" if tr and tr != "UNKNOWN" else ""
                parts.append(f"{lang}={status}{tr_str}")
            print(f"  Test {c.test_id:4d}  {' | '.join(parts)}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="N-way cross-validator for SCXML code generation"
    )
    parser.add_argument("--test", type=str, help="Validate single test ID (e.g. 144 or 403a)")
    parser.add_argument("--lang", type=str,
                        help="Comma-separated languages (default: all registered)")
    parser.add_argument("--json", action="store_true", help="Output JSON report")
    parser.add_argument("--verbose", "-v", action="store_true",
                        help="Show all tests, not just failures")
    parser.add_argument("--output", "-o", type=str,
                        help="Write JSON report to file")
    args = parser.parse_args()

    # Discover languages
    if args.lang:
        languages = [l.strip() for l in args.lang.split(",")]
    else:
        languages = supported_languages()

    # Create generators
    generators = {}
    for lang in languages:
        try:
            generators[lang] = get_generator(lang)
        except ValueError as e:
            print(f"Warning: {e}", file=sys.stderr)

    if not generators:
        print("No valid generators found.", file=sys.stderr)
        sys.exit(1)

    print(f"  Languages: {', '.join(generators.keys())}", file=sys.stderr)

    # Discover tests
    if args.test:
        test_ids = [args.test]
    else:
        test_ids = discover_test_ids()

    if not test_ids:
        print("No SCXML tests found in resources/", file=sys.stderr)
        sys.exit(1)

    # Parse test results for all languages
    test_results = {}
    for lang in generators:
        test_results[lang] = parse_test_results(lang)

    # Run comparisons
    comparisons = []
    with tempfile.TemporaryDirectory(prefix="cross_val_") as tmp_dir:
        tmp_path = Path(tmp_dir)
        for i, test_id in enumerate(test_ids):
            if not args.json and not args.test:
                print(f"\r  Validating: {i + 1}/{len(test_ids)} (test{test_id})",
                      end="", flush=True, file=sys.stderr)
            comp = compare_test(test_id, generators, tmp_path, test_results)
            comparisons.append(comp)

    if not args.json and not args.test:
        print("", file=sys.stderr)

    # Build report
    langs = list(generators.keys())
    report = build_report(comparisons, langs)

    if args.json or args.output:
        report_dict = asdict(report)
        json_str = json.dumps(report_dict, indent=2, ensure_ascii=False)
        if args.output:
            Path(args.output).write_text(json_str)
            print(f"Report written to {args.output}", file=sys.stderr)
        else:
            print(json_str)
    else:
        print_report(report, comparisons, verbose=args.verbose)

    # Exit code: 0 if full parity, 1 otherwise
    any_fail = any(
        s["test_fail"] > 0 for s in report.per_language.values()
    )
    if report.all_generate_ok == report.total_tests and not any_fail:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()
