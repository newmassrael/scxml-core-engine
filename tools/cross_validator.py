#!/usr/bin/env python3
"""
C++/Kotlin Cross-Validator for SCXML Code Generation

Validates production readiness of Kotlin codegen by comparing structural
parity with C++ codegen across all W3C SCXML tests.

Comparison levels:
  1. Codegen success: Both generators succeed on same SCXML
  2. Structural parity: Same states, events, initial state, features
  3. Test result parity: Both pass/fail identically (Kotlin JUnit XML)

Usage:
  python3 tools/cross_validator.py                  # Full validation
  python3 tools/cross_validator.py --test 144       # Single test
  python3 tools/cross_validator.py --json            # JSON output
  python3 tools/cross_validator.py --verbose         # Show all, not just failures
"""

import argparse
import contextlib
import io
import json
import os
import re
import sys
import tempfile
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

# Add codegen to path
TOOLS_DIR = Path(__file__).resolve().parent
CODEGEN_DIR = TOOLS_DIR / "codegen"
sys.path.insert(0, str(CODEGEN_DIR))

from generators.kotlin_generator import KotlinCodeGenerator
from generators.cpp_generator import CppCodeGenerator

PROJECT_ROOT = TOOLS_DIR.parent
RESOURCES_DIR = PROJECT_ROOT / "resources"
KOTLIN_TEST_RESULTS_DIR = (
    PROJECT_ROOT / "sce-kotlin-tests" / "build" / "test-results" / "test"
)


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class CodegenInfo:
    """Extracted structural information from generated code."""
    success: bool = False
    error: str = ""
    states: List[str] = field(default_factory=list)
    events: List[str] = field(default_factory=list)
    initial_state: str = ""
    needs_script_engine: bool = False
    needs_http_send: bool = False
    has_parallel: bool = False
    has_history: bool = False


@dataclass
class TestComparison:
    """Comparison result for a single test."""
    test_id: int
    cpp: CodegenInfo = field(default_factory=CodegenInfo)
    kotlin: CodegenInfo = field(default_factory=CodegenInfo)
    kotlin_test_result: str = "UNKNOWN"  # PASS, FAIL, SKIP, ERROR, UNKNOWN
    # Parity checks
    both_generate: bool = False
    states_match: bool = False
    events_match: bool = False
    initial_match: bool = False
    features_match: bool = False
    # Differences
    state_diff: str = ""
    event_diff: str = ""
    feature_diff: str = ""


@dataclass
class ValidationReport:
    """Full cross-validation report."""
    total_tests: int = 0
    both_generate_ok: int = 0
    cpp_only: int = 0
    kotlin_only: int = 0
    neither_generates: int = 0
    states_match: int = 0
    events_match: int = 0
    initial_match: int = 0
    features_match: int = 0
    full_parity: int = 0
    kotlin_pass: int = 0
    kotlin_fail: int = 0
    kotlin_skip: int = 0
    kotlin_unknown: int = 0
    failures: List[Dict] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Code parsing: extract structure from generated output
# ---------------------------------------------------------------------------

# C++ infrastructure events not present in Kotlin sealed interfaces.
# C++ enumerates all possible events (including error/lifecycle) in enum for type safety.
# Kotlin handles infrastructure events via string matching at runtime.
CPP_INFRA_EVENTS = {"NONE", "Wildcard"}

# C++ infrastructure event prefixes that Kotlin handles differently
CPP_INFRA_EVENT_PREFIXES = (
    "Error_",       # W3C SCXML error events (error.execution, error.communication)
    "Error",        # Generic error event
    "Cancel_invoke",  # Invoke lifecycle
    "Done_invoke",    # Invoke lifecycle
    "Done_state_",    # Parallel state completion (done.state.X)
    "HTTP_",          # HTTP infrastructure events
)


def _strip_cpp_line_comments(text: str) -> str:
    """Remove C++ // comments from each line before parsing enums."""
    lines = text.split("\n")
    return "\n".join(line.split("//")[0] for line in lines)


def extract_cpp_info(header_content: str, inline_content: str) -> CodegenInfo:
    """Extract structural info from generated C++ header + inline."""
    info = CodegenInfo(success=True)

    # Strip line comments before parsing enums to avoid comma confusion
    header_no_comments = _strip_cpp_line_comments(header_content)

    # States: enum class State : uint8_t { Fail, Pass, S0, S1 };
    state_match = re.search(
        r"enum\s+class\s+State\s*:\s*uint8_t\s*\{([^}]+)\}", header_no_comments
    )
    if state_match:
        raw = state_match.group(1)
        info.states = sorted(
            s.strip() for s in raw.split(",") if s.strip()
        )

    # Events: enum class Event : uint8_t { NONE, Bar, Foo, Wildcard };
    event_match = re.search(
        r"enum\s+class\s+Event\s*:\s*uint8_t\s*\{([^}]+)\}", header_no_comments
    )
    if event_match:
        raw = event_match.group(1)
        all_events = [e.strip() for e in raw.split(",") if e.strip()]
        # Filter infrastructure events (exact match + prefix match)
        info.events = sorted(
            e for e in all_events
            if e not in CPP_INFRA_EVENTS
            and not any(e.startswith(p) for p in CPP_INFRA_EVENT_PREFIXES)
        )

    # Initial state: static constexpr initialState() { return State::S0; }
    init_match = re.search(
        r"initialState\(\)[^{]*\{[^}]*return\s+State::(\w+)", header_content
    )
    if not init_match:
        init_match = re.search(r"initialState_\s*=\s*State::(\w+)", header_content)
    if init_match:
        info.initial_state = init_match.group(1)

    # Script engine
    if "NEEDS_SCRIPT_ENGINE = true" in header_content:
        info.needs_script_engine = True

    # Parallel states
    if "HAS_PARALLEL_STATES = true" in header_content:
        info.has_parallel = True

    # HTTP send
    if "performHttpSend(" in inline_content or "performHttpSend(" in header_content:
        info.needs_http_send = True

    # History: check for actual usage, not just #include
    if ("historyDefault" in inline_content or "saveHistory" in inline_content or
            "restoreHistory" in inline_content or "HistoryType::" in header_content):
        info.has_history = True

    return info


def _normalize_event_name(name: str) -> str:
    """
    Normalize event name to canonical form for cross-language comparison.

    Both C++ and Kotlin represent SCXML dot-separated events differently:
      - C++: underscore-separated enum (Foo_zoo, In_s11p112)
      - Kotlin: sealed interface hierarchy (Foo.Zoo, InS11p112)

    Canonical form: lowercase, no separators (foozoo, ins11p112)
    """
    # Remove .Self suffix (Kotlin uses .Self for exact match of branch events)
    name = re.sub(r"\.Self$", "", name)
    # Convert dots and underscores to nothing, then lowercase
    return re.sub(r"[._]", "", name).lower()


def extract_kotlin_info(kt_content: str) -> CodegenInfo:
    """Extract structural info from generated Kotlin file."""
    info = CodegenInfo(success=True)

    # States: sealed interface TestXXXState : State { data object Foo : TestXXXState }
    state_objects = re.findall(r"data\s+object\s+(\w+)\s*:\s*\w+State", kt_content)
    info.states = sorted(state_objects)

    # Events: Capture both direct data objects and nested sealed interface hierarchies.
    # Kotlin uses sealed interface hierarchy for prefix matching:
    #   sealed interface Foo : TestXXXEvent {
    #       data object Self : Foo   // exact "foo" event
    #       data object Zoo : Foo    // "foo.zoo" event
    #   }
    #   data object Bar : TestXXXEvent  // flat event

    # Step 1: Find direct event data objects (flat events)
    direct_events = re.findall(r"data\s+object\s+(\w+)\s*:\s*\w+Event", kt_content)

    # Step 2: Find sealed interface branches and their children
    # Pattern: sealed interface Foo : TestXXXEvent { ... data object Self : Foo ... data object Zoo : Foo ... }
    branch_pattern = re.compile(
        r"sealed\s+interface\s+(\w+)\s*:\s*\w+Event\s*\{", re.MULTILINE
    )
    branch_events = []
    for m in branch_pattern.finditer(kt_content):
        branch_name = m.group(1)
        # Find child data objects of this branch
        # They're declared as: data object XXX : BranchName
        children = re.findall(
            rf"data\s+object\s+(\w+)\s*:\s*{re.escape(branch_name)}\b", kt_content
        )
        for child in children:
            if child == "Self":
                branch_events.append(branch_name)  # Foo.Self → Foo
            else:
                branch_events.append(f"{branch_name}_{child}")  # Foo.Zoo → Foo_Zoo

    all_events = set(direct_events + branch_events)
    # Filter infrastructure events (same as C++ filtering)
    info.events = sorted(
        e for e in all_events
        if e not in CPP_INFRA_EVENTS
        and not any(_normalize_event_name(e).startswith(p.lower().replace("_", ""))
                    for p in CPP_INFRA_EVENT_PREFIXES)
    )

    # Initial state: override val initialState = TestXXXState.S0
    init_match = re.search(r"override\s+val\s+initialState.*?=\s*\w+State\.(\w+)", kt_content)
    if init_match:
        info.initial_state = init_match.group(1)

    # Script engine
    if "ensureScriptEngine()" in kt_content:
        info.needs_script_engine = True

    # Parallel: check for isParallelState returning true
    if re.search(r"isParallelState\(.*?\).*?true", kt_content, re.DOTALL):
        info.has_parallel = True

    # HTTP send
    if "performHttpSend(" in kt_content:
        info.needs_http_send = True

    # History: check for actual history save/restore, not just imports
    if re.search(r"saveHistory\(|restoreHistory\(|historyDefault", kt_content):
        info.has_history = True

    return info


# ---------------------------------------------------------------------------
# Kotlin JUnit XML result parsing
# ---------------------------------------------------------------------------

def parse_kotlin_results() -> Dict[int, str]:
    """Parse Kotlin JUnit XML test results. Returns {test_id: 'PASS'|'FAIL'|'SKIP'|'ERROR'}."""
    results = {}
    if not KOTLIN_TEST_RESULTS_DIR.exists():
        return results

    for xml_file in KOTLIN_TEST_RESULTS_DIR.glob("TEST-com.sce.w3c.Test*.xml"):
        # Extract test ID from filename: TEST-com.sce.w3c.Test144.xml -> 144
        match = re.search(r"Test(\d+[a-z]?)\.xml$", xml_file.name)
        if not match:
            continue
        test_id_str = match.group(1)
        # Handle variant tests like 403a, 403b
        try:
            test_id = int(test_id_str)
        except ValueError:
            # Variant test (e.g., "403a") — use base number
            test_id = int(re.match(r"(\d+)", test_id_str).group(1))

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
# Core comparison logic
# ---------------------------------------------------------------------------

def compare_test(test_id: int, cpp_gen: CppCodeGenerator,
                 kt_gen: KotlinCodeGenerator, tmp_dir: Path,
                 kotlin_results: Dict[int, str]) -> TestComparison:
    """Run both generators on a test and compare outputs."""
    comp = TestComparison(test_id=test_id)
    comp.kotlin_test_result = kotlin_results.get(test_id, "UNKNOWN")

    scxml_path = RESOURCES_DIR / str(test_id) / f"test{test_id}.scxml"
    if not scxml_path.exists():
        comp.cpp.error = "SCXML not found"
        comp.kotlin.error = "SCXML not found"
        return comp

    # Suppress codegen stdout noise
    _devnull = io.StringIO()

    # Generate C++
    cpp_dir = tmp_dir / f"cpp_{test_id}"
    cpp_dir.mkdir(exist_ok=True)
    try:
        with contextlib.redirect_stdout(_devnull):
            cpp_ok = cpp_gen.generate(str(scxml_path), str(cpp_dir))
        if cpp_ok:
            header_path = cpp_dir / f"test{test_id}_sm.h"
            inline_path = cpp_dir / f"test{test_id}_sm.inl"
            if header_path.exists():
                header_text = header_path.read_text()
                inline_text = inline_path.read_text() if inline_path.exists() else ""
                comp.cpp = extract_cpp_info(header_text, inline_text)
            else:
                comp.cpp.error = "Header not generated"
        else:
            comp.cpp.error = "Generation returned False"
    except Exception as e:
        comp.cpp.error = str(e)

    # Generate Kotlin
    kt_dir = tmp_dir / f"kt_{test_id}"
    kt_dir.mkdir(exist_ok=True)
    try:
        with contextlib.redirect_stdout(_devnull):
            kt_ok = kt_gen.generate(str(scxml_path), str(kt_dir))
        if kt_ok:
            kt_path = kt_dir / f"test{test_id}Sm.kt"
            if kt_path.exists():
                comp.kotlin = extract_kotlin_info(kt_path.read_text())
            else:
                comp.kotlin.error = "Kotlin file not generated"
        else:
            comp.kotlin.error = "Generation returned False"
    except Exception as e:
        comp.kotlin.error = str(e)

    # Parity checks
    comp.both_generate = comp.cpp.success and comp.kotlin.success

    if comp.both_generate:
        # States comparison
        if comp.cpp.states == comp.kotlin.states:
            comp.states_match = True
        else:
            cpp_set = set(comp.cpp.states)
            kt_set = set(comp.kotlin.states)
            missing_in_kt = cpp_set - kt_set
            extra_in_kt = kt_set - cpp_set
            parts = []
            if missing_in_kt:
                parts.append(f"missing in Kotlin: {sorted(missing_in_kt)}")
            if extra_in_kt:
                parts.append(f"extra in Kotlin: {sorted(extra_in_kt)}")
            comp.state_diff = "; ".join(parts)

        # Events comparison: use normalized names for cross-language parity
        cpp_norm = {_normalize_event_name(e) for e in comp.cpp.events}
        kt_norm = {_normalize_event_name(e) for e in comp.kotlin.events}
        if cpp_norm == kt_norm:
            comp.events_match = True
        else:
            missing_in_kt = cpp_norm - kt_norm
            extra_in_kt = kt_norm - cpp_norm
            parts = []
            if missing_in_kt:
                # Show original names for clarity
                missing_orig = sorted(
                    e for e in comp.cpp.events
                    if _normalize_event_name(e) in missing_in_kt
                )
                parts.append(f"missing in Kotlin: {missing_orig}")
            if extra_in_kt:
                extra_orig = sorted(
                    e for e in comp.kotlin.events
                    if _normalize_event_name(e) in extra_in_kt
                )
                parts.append(f"extra in Kotlin: {extra_orig}")
            comp.event_diff = "; ".join(parts)

        # Initial state
        comp.initial_match = comp.cpp.initial_state == comp.kotlin.initial_state

        # Feature flags
        feature_diffs = []
        if comp.cpp.needs_script_engine != comp.kotlin.needs_script_engine:
            feature_diffs.append(
                f"script_engine: C++={comp.cpp.needs_script_engine}, Kotlin={comp.kotlin.needs_script_engine}"
            )
        if comp.cpp.needs_http_send != comp.kotlin.needs_http_send:
            feature_diffs.append(
                f"http_send: C++={comp.cpp.needs_http_send}, Kotlin={comp.kotlin.needs_http_send}"
            )
        if comp.cpp.has_parallel != comp.kotlin.has_parallel:
            feature_diffs.append(
                f"parallel: C++={comp.cpp.has_parallel}, Kotlin={comp.kotlin.has_parallel}"
            )
        if comp.cpp.has_history != comp.kotlin.has_history:
            feature_diffs.append(
                f"history: C++={comp.cpp.has_history}, Kotlin={comp.kotlin.has_history}"
            )

        comp.features_match = len(feature_diffs) == 0
        comp.feature_diff = "; ".join(feature_diffs)

    return comp


# ---------------------------------------------------------------------------
# Discovery
# ---------------------------------------------------------------------------

def discover_test_ids() -> List[int]:
    """Discover all test IDs from resources directory."""
    test_ids = []
    if not RESOURCES_DIR.exists():
        return test_ids
    for entry in RESOURCES_DIR.iterdir():
        if entry.is_dir() and entry.name.isdigit():
            test_id = int(entry.name)
            scxml = entry / f"test{test_id}.scxml"
            if scxml.exists():
                test_ids.append(test_id)
    return sorted(test_ids)


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------

def build_report(comparisons: List[TestComparison]) -> ValidationReport:
    """Aggregate comparisons into a validation report."""
    report = ValidationReport(total_tests=len(comparisons))

    for comp in comparisons:
        # Codegen parity
        if comp.both_generate:
            report.both_generate_ok += 1
        elif comp.cpp.success and not comp.kotlin.success:
            report.cpp_only += 1
        elif not comp.cpp.success and comp.kotlin.success:
            report.kotlin_only += 1
        else:
            report.neither_generates += 1

        # Structural parity (only for tests where both generate)
        if comp.both_generate:
            if comp.states_match:
                report.states_match += 1
            if comp.events_match:
                report.events_match += 1
            if comp.initial_match:
                report.initial_match += 1
            if comp.features_match:
                report.features_match += 1
            if (comp.states_match and comp.events_match and
                    comp.initial_match and comp.features_match):
                report.full_parity += 1

        # Kotlin test results
        if comp.kotlin_test_result == "PASS":
            report.kotlin_pass += 1
        elif comp.kotlin_test_result == "FAIL":
            report.kotlin_fail += 1
        elif comp.kotlin_test_result == "SKIP":
            report.kotlin_skip += 1
        else:
            report.kotlin_unknown += 1

        # Collect failures
        if comp.both_generate and not (
            comp.states_match and comp.events_match and
            comp.initial_match and comp.features_match
        ):
            failure = {
                "test_id": comp.test_id,
                "kotlin_test": comp.kotlin_test_result,
            }
            if not comp.states_match:
                failure["state_diff"] = comp.state_diff
            if not comp.events_match:
                failure["event_diff"] = comp.event_diff
            if not comp.initial_match:
                failure["initial_diff"] = (
                    f"C++={comp.cpp.initial_state}, Kotlin={comp.kotlin.initial_state}"
                )
            if not comp.features_match:
                failure["feature_diff"] = comp.feature_diff
            report.failures.append(failure)

    return report


def print_report(report: ValidationReport, comparisons: List[TestComparison],
                 verbose: bool = False):
    """Print human-readable report to stdout."""
    gen = report.both_generate_ok
    total = report.total_tests

    print("=" * 70)
    print("  C++/Kotlin Cross-Validation Report")
    print("=" * 70)
    print()

    # Codegen summary
    print("--- Code Generation ---")
    print(f"  Total SCXML tests:     {total}")
    print(f"  Both generate OK:      {gen}")
    print(f"  C++ only:              {report.cpp_only}")
    print(f"  Kotlin only:           {report.kotlin_only}")
    print(f"  Neither generates:     {report.neither_generates}")
    print()

    # Structural parity (of those where both generate)
    print(f"--- Structural Parity (of {gen} shared tests) ---")
    print(f"  States match:          {report.states_match}/{gen}")
    print(f"  Events match:          {report.events_match}/{gen}")
    print(f"  Initial state match:   {report.initial_match}/{gen}")
    print(f"  Feature flags match:   {report.features_match}/{gen}")
    print(f"  Full parity:           {report.full_parity}/{gen}")
    print()

    # Kotlin test results
    kt_total = report.kotlin_pass + report.kotlin_fail + report.kotlin_skip
    print(f"--- Kotlin Test Results ({kt_total} with results) ---")
    print(f"  PASS:                  {report.kotlin_pass}")
    print(f"  FAIL:                  {report.kotlin_fail}")
    print(f"  SKIP:                  {report.kotlin_skip}")
    print(f"  No result:             {report.kotlin_unknown}")
    print()

    # Production readiness verdict
    parity_pct = (report.full_parity / gen * 100) if gen > 0 else 0
    print("--- Production Readiness ---")
    if parity_pct == 100 and report.kotlin_fail == 0:
        print(f"  VERDICT: PRODUCTION READY")
        print(f"  {gen}/{gen} tests have full structural parity")
        print(f"  {report.kotlin_pass}/{kt_total} Kotlin tests pass")
    elif parity_pct >= 95 and report.kotlin_fail == 0:
        print(f"  VERDICT: NEAR PRODUCTION READY ({parity_pct:.1f}% parity)")
        print(f"  {len(report.failures)} test(s) with structural differences")
    else:
        print(f"  VERDICT: NEEDS WORK ({parity_pct:.1f}% parity)")
        print(f"  {len(report.failures)} test(s) with structural differences")
        print(f"  {report.kotlin_fail} Kotlin test failure(s)")
    print()

    # Failures
    if report.failures:
        print(f"--- Structural Differences ({len(report.failures)}) ---")
        for f in report.failures:
            print(f"\n  Test {f['test_id']} (Kotlin: {f['kotlin_test']})")
            if "state_diff" in f:
                print(f"    States:   {f['state_diff']}")
            if "event_diff" in f:
                print(f"    Events:   {f['event_diff']}")
            if "initial_diff" in f:
                print(f"    Initial:  {f['initial_diff']}")
            if "feature_diff" in f:
                print(f"    Features: {f['feature_diff']}")
        print()

    # Codegen-only failures (one side fails)
    cpp_only_tests = [c for c in comparisons if c.cpp.success and not c.kotlin.success]
    kt_only_tests = [c for c in comparisons if not c.cpp.success and c.kotlin.success]

    if cpp_only_tests and verbose:
        print(f"--- C++ Only ({len(cpp_only_tests)} tests) ---")
        for c in cpp_only_tests[:10]:
            print(f"  Test {c.test_id}: Kotlin error: {c.kotlin.error}")
        if len(cpp_only_tests) > 10:
            print(f"  ... and {len(cpp_only_tests) - 10} more")
        print()

    if kt_only_tests and verbose:
        print(f"--- Kotlin Only ({len(kt_only_tests)} tests) ---")
        for c in kt_only_tests[:10]:
            print(f"  Test {c.test_id}: C++ error: {c.cpp.error}")
        if len(kt_only_tests) > 10:
            print(f"  ... and {len(kt_only_tests) - 10} more")
        print()

    if verbose:
        # All tests detail
        print(f"--- All Tests Detail ---")
        for c in comparisons:
            status = "OK" if (c.both_generate and c.states_match and
                              c.events_match and c.initial_match and
                              c.features_match) else "DIFF"
            gen_status = ("BOTH" if c.both_generate else
                          "CPP" if c.cpp.success else
                          "KT" if c.kotlin.success else "NONE")
            kt_test = c.kotlin_test_result
            print(f"  Test {c.test_id:4d}  gen={gen_status:4s}  "
                  f"parity={status:4s}  kotlin_test={kt_test}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="C++/Kotlin cross-validator for SCXML codegen"
    )
    parser.add_argument("--test", type=int, help="Validate single test ID")
    parser.add_argument("--json", action="store_true", help="Output JSON report")
    parser.add_argument("--verbose", "-v", action="store_true",
                        help="Show all tests, not just failures")
    parser.add_argument("--output", "-o", type=str,
                        help="Write JSON report to file")
    args = parser.parse_args()

    # Discover tests
    if args.test:
        test_ids = [args.test]
    else:
        test_ids = discover_test_ids()

    if not test_ids:
        print("No SCXML tests found in resources/", file=sys.stderr)
        sys.exit(1)

    # Parse Kotlin test results
    kotlin_results = parse_kotlin_results()

    # Initialize generators
    cpp_gen = CppCodeGenerator()
    kt_gen = KotlinCodeGenerator()

    # Run comparisons
    comparisons = []
    with tempfile.TemporaryDirectory(prefix="cross_val_") as tmp_dir:
        tmp_path = Path(tmp_dir)
        for i, test_id in enumerate(test_ids):
            if not args.json and not args.test:
                print(f"\r  Validating: {i + 1}/{len(test_ids)} (test{test_id})",
                      end="", flush=True, file=sys.stderr)
            comp = compare_test(test_id, cpp_gen, kt_gen, tmp_path, kotlin_results)
            comparisons.append(comp)

    if not args.json and not args.test:
        print("", file=sys.stderr)  # newline after progress

    # Build report
    report = build_report(comparisons)

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
    if report.full_parity == report.both_generate_ok and report.kotlin_fail == 0:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()
