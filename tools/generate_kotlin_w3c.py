#!/usr/bin/env python3
"""
Kotlin W3C SCXML Test Generator

Generates Kotlin state machine code and JUnit5 test classes for all
W3C SCXML conformance tests (pure static AND script-engine hybrid).

Usage:
    python3 tools/generate_kotlin_w3c.py [--test TEST_ID] [--clean]

Output:
    sce-kotlin-tests/src/main/kotlin/com/sce/generated/testXXX/testXXXSm.kt
    sce-kotlin-tests/src/test/kotlin/com/sce/w3c/TestXXX.kt
"""

from __future__ import annotations

import argparse
import filecmp
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# Project root (parent of tools/)
PROJECT_ROOT = Path(__file__).resolve().parent.parent
RESOURCES_DIR = PROJECT_ROOT / 'resources'
CODEGEN_SCRIPT = PROJECT_ROOT / 'tools' / 'codegen' / 'codegen.py'
TESTS_MODULE = PROJECT_ROOT / 'sce-kotlin-tests'
SM_OUTPUT_BASE = TESTS_MODULE / 'src' / 'main' / 'kotlin' / 'com' / 'sce' / 'generated'
TEST_OUTPUT_DIR = TESTS_MODULE / 'src' / 'test' / 'kotlin' / 'com' / 'sce' / 'w3c'

# C++ CMakeLists.txt — source of truth for test IDs and types
CMAKE_FILE = PROJECT_ROOT / 'tests' / 'CMakeLists.txt'

# Tests excluded from Kotlin codegen (structural issues only, not script engine)
CODEGEN_EXCLUDE: set = set()


def write_if_changed(path: Path, content: str) -> bool:
    """Write file only if content differs. Returns True if written."""
    if path.exists() and path.read_text() == content:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    return True


def parse_cmake_tests() -> dict:
    """
    Parse test registrations from C++ CMakeLists.txt.

    Returns dict: test_id -> {'type': 'SIMPLE'|'SCHEDULED'|'HTTP', 'comment': str}
    """
    tests = {}
    pattern = re.compile(
        r'sce_generate_static_w3c_test\((\S+)\s+\$\{STATIC_W3C_OUTPUT_DIR\}'
        r'(?:\s+TYPE\s+(\w+))?\)'
        r'\s*#\s*(.*)'
    )

    with open(CMAKE_FILE) as f:
        for line in f:
            m = pattern.search(line)
            if m:
                test_id = m.group(1)
                test_type = m.group(2) or 'SIMPLE'
                comment = m.group(3).strip()
                tests[test_id] = {
                    'type': test_type,
                    'comment': comment,
                }

    return tests


def read_metadata(test_id: str) -> dict:
    """Read metadata.txt for a test, resolving variant directories."""
    num_prefix = re.match(r'(\d+)', test_id)
    if not num_prefix:
        return {}
    resource_dir = RESOURCES_DIR / num_prefix.group(1)
    meta_file = resource_dir / 'metadata.txt'
    if not meta_file.exists():
        return {}

    metadata = {}
    with open(meta_file) as f:
        for line in f:
            line = line.strip()
            if ':' in line:
                key, _, value = line.partition(':')
                metadata[key.strip()] = value.strip()
    return metadata


def find_scxml(test_id: str) -> Path | None:
    """Find the SCXML file for a test ID."""
    num_prefix = re.match(r'(\d+)', test_id)
    if not num_prefix:
        return None
    resource_dir = RESOURCES_DIR / num_prefix.group(1)
    scxml = resource_dir / f'test{test_id}.scxml'
    if scxml.exists():
        return scxml
    return None


def to_pascal_case(name: str) -> str:
    """Convert test ID to PascalCase class name."""
    parts = re.split(r'[_\-]', name)
    return ''.join(p[0].upper() + p[1:] if p else '' for p in parts)


def generate_child_sms(test_id: str, scxml_path: Path, output_dir: Path) -> int:
    """
    Generate child state machines for static invoke tests.

    Finds child SCXML files extracted by the parser (_child0, _machineName, sub1, etc.)
    and generates Kotlin SMs in the same package as the parent.
    Skips children not referenced by the parent SM (hybrid invokes use ScxmlRuntimeInterpreter).

    Returns number of children generated.
    """
    resource_dir = scxml_path.parent
    parent_stem = scxml_path.stem  # e.g., "test191"
    parent_package = f'test{test_id}'
    count = 0

    # Find child SCXML files: test{num}_*.scxml and test{num}sub*.scxml
    num_prefix = re.match(r'(\d+)', test_id)
    if not num_prefix:
        return 0
    num = num_prefix.group(1)

    # Read parent SM to check which child classes are actually referenced
    # Hybrid invoke tests use ScxmlRuntimeInterpreter (no static child class reference)
    parent_sm_file = output_dir / f'{parent_stem}Sm.kt'
    parent_sm_content = parent_sm_file.read_text() if parent_sm_file.exists() else ''

    for child_scxml in resource_dir.glob('*.scxml'):
        child_name = child_scxml.stem
        # Skip the parent itself
        if child_name == parent_stem:
            continue
        # Match: test191_child0, test191_machineName, test226sub1, etc.
        if not (child_name.startswith(f'test{num}_') or
                child_name.startswith(f'test{num}sub')):
            continue

        # Skip if parent SM doesn't reference this child class (hybrid invoke — runtime loaded)
        child_class = to_pascal_case(child_name)
        if f'{child_class}StateMachine' not in parent_sm_content:
            continue

        # Generate child SM (script engine or pure static — both supported now)
        child_result = subprocess.run(
            [
                sys.executable, str(CODEGEN_SCRIPT),
                str(child_scxml),
                '-o', str(output_dir),
                '--language', 'kotlin',
            ],
            capture_output=True,
            text=True,
            env={**os.environ, 'SPDLOG_LEVEL': 'warn'},
        )

        if 'Generated:' not in child_result.stdout:
            # Codegen failed for structural reasons — generate a no-op stub
            child_sm_file = output_dir / f'{child_name}Sm.kt'
            child_class = to_pascal_case(child_name)
            stub_content = (
                f"// GENERATED STUB — child codegen failed (no-op)\n"
                f"package com.sce.generated.{parent_package}\n\n"
                f"import com.sce.runtime.*\n\n"
                f"sealed interface {child_class}State : State {{\n"
                f"    data object Initial : {child_class}State\n"
                f"}}\n"
                f"sealed interface {child_class}Event : Event\n\n"
                f"class {child_class}StateMachine(\n"
                f"    scriptEngine: ScxmlScriptEngine? = null\n"
                f") : StateMachineEngine<{child_class}State, {child_class}Event>(scriptEngine) {{\n"
                f"    override val initialState = {child_class}State.Initial\n"
                f"    override fun processEvent(state: {child_class}State, event: {child_class}Event) = TransitionResult.Ignored as TransitionResult<{child_class}State>\n"
                f"    override fun onEntry(state: {child_class}State) {{}}\n"
                f"    override fun onExit(state: {child_class}State) {{}}\n"
                f"    override fun executeTransitionActions(source: {child_class}State, event: {child_class}Event?) {{}}\n"
                f"}}\n"
            )
            write_if_changed(child_sm_file, stub_content)
            count += 1
            continue

        # Fix package name: child SM must be in same package as parent
        child_sm_file = output_dir / f'{child_name}Sm.kt'
        if child_sm_file.exists():
            content = child_sm_file.read_text()
            # Replace child's own package with parent's package
            child_package = child_name.lower()
            new_content = content.replace(
                f'package com.sce.generated.{child_package}',
                f'package com.sce.generated.{parent_package}'
            )
            write_if_changed(child_sm_file, new_content)
            count += 1

    return count


def copy_if_changed(src_dir: Path, dst_dir: Path):
    """Copy files from src to dst only if content differs. Preserves timestamps for unchanged files."""
    dst_dir.mkdir(parents=True, exist_ok=True)
    for src_file in src_dir.iterdir():
        if src_file.is_file():
            dst_file = dst_dir / src_file.name
            content = src_file.read_text()
            write_if_changed(dst_file, content)


def generate_sm(test_id: str, scxml_path: Path) -> tuple[str, bool]:
    """
    Run Kotlin codegen for a single test.
    Uses a temp dir to avoid touching timestamps on unchanged files.

    Returns:
        ('generated', needs_script_engine) — SM file written successfully
        ('failed', False) — codegen error
    """
    output_dir = SM_OUTPUT_BASE / f'test{test_id}'

    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        result = subprocess.run(
            [
                sys.executable, str(CODEGEN_SCRIPT),
                str(scxml_path),
                '-o', str(tmp_dir),
                '--language', 'kotlin',
            ],
            capture_output=True,
            text=True,
            env={**os.environ, 'SPDLOG_LEVEL': 'warn'},
        )

        stdout = result.stdout
        needs_script = 'Needs ScriptEngine: True' in stdout

        if 'Generated:' not in stdout:
            return 'failed', False

        # Copy only changed files to preserve timestamps
        copy_if_changed(tmp_dir, output_dir)

    # Generate child SMs for invoke tests
    generate_child_sms(test_id, scxml_path, output_dir)
    return 'generated', needs_script


def detect_pass_state(test_id: str) -> str | None:
    """
    Detect the pass state name from the generated Kotlin SM code.

    W3C convention: "pass"/"fail" final states. Some tests use "final" only.
    Returns PascalCase state name or None if not detectable.
    """
    sm_file = SM_OUTPUT_BASE / f'test{test_id}' / f'test{test_id}Sm.kt'
    if not sm_file.exists():
        return None

    content = sm_file.read_text()

    # Find which states are final (markFinalStateReached() in onEntry)
    # Strategy: scan line-by-line through onEntry for state -> markFinalStateReached pairs.
    # This avoids fragile nested-brace regex parsing.
    final_states = set()
    in_on_entry = False
    current_state = None
    brace_depth = 0
    for line in content.splitlines():
        stripped = line.strip()
        if not in_on_entry:
            if 'override fun onEntry(' in line:
                in_on_entry = True
                # Count braces on the declaration line itself (includes opening {)
                brace_depth = stripped.count('{') - stripped.count('}')
            continue
        brace_depth += stripped.count('{') - stripped.count('}')
        if brace_depth <= 0:
            in_on_entry = False
            current_state = None
            continue
        m = re.match(r'is \w+State\.(\w+)\s*->', stripped)
        if m:
            current_state = m.group(1)
        if current_state and 'markFinalStateReached()' in stripped:
            final_states.add(current_state)

    # Priority: Pass > Final > first non-Fail final state
    if 'Pass' in final_states:
        return 'Pass'
    if 'Final' in final_states:
        return 'Final'
    non_fail = [s for s in final_states if s.lower() != 'fail']
    if len(non_fail) == 1:
        return non_fail[0]

    return None


def sm_uses_http_send(test_id: str) -> bool:
    """Check if the generated SM actually uses performHttpSend()."""
    sm_file = SM_OUTPUT_BASE / f'test{test_id}' / f'test{test_id}Sm.kt'
    if not sm_file.exists():
        return False
    return 'performHttpSend(' in sm_file.read_text()


def generate_test_class(test_id: str, metadata: dict, test_type: str,
                        needs_script_engine: bool) -> bool:
    """Generate a JUnit5 test class for a W3C test. Returns False if pass state not detected."""
    pass_state = detect_pass_state(test_id)
    if not pass_state:
        return False

    sm_class = f'Test{to_pascal_case(test_id)}'
    sm_package = f'test{test_id}'
    specnum = metadata.get('specnum', '')
    description = metadata.get('description', '')

    # W3C SCXML C.2: HTTP tests use W3CHttpTestBase only when SM actually uses performHttpSend()
    # Some TYPE HTTP tests (e.g., 201) dispatch events internally without real HTTP
    is_http = test_type == 'HTTP' and sm_uses_http_send(test_id)
    base_class = 'W3CHttpTestBase' if is_http else 'W3CTestBase'

    # W3C SCXML 6.2: SCHEDULED tests need longer timeout for delayed send/invoke
    # W3CTestBase default is 2s, SCHEDULED tests get 5s (matches C++ ScheduledAotTest pattern)
    timeout_override = ''
    if test_type == 'SCHEDULED':
        timeout_override = f"    override val timeoutMs: Long = 5000L\n"

    # Build imports
    imports = (
        f"import com.sce.generated.{sm_package}.{sm_class}Event\n"
        f"import com.sce.generated.{sm_package}.{sm_class}State\n"
        f"import com.sce.generated.{sm_package}.{sm_class}StateMachine\n"
    )
    imports += f"import org.junit.jupiter.api.DisplayName\n"

    # Script engine injection for hybrid tests
    # Uses W3CTestBase.createEngine() — engine selected via system property, no hardcoded import
    if needs_script_engine:
        create_sm = f"    override fun createStateMachine() = {sm_class}StateMachine(createEngine())\n"
    else:
        create_sm = f"    override fun createStateMachine() = {sm_class}StateMachine()\n"

    content = (
        f"// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)\n"
        f"package com.sce.w3c\n"
        f"\n"
        f"{imports}"
        f"\n"
        f"// W3C SCXML {specnum}: {description}\n"
        f"@DisplayName(\"Test {test_id} -- W3C SCXML {specnum}\")\n"
        f"class Test{to_pascal_case(test_id)} : {base_class}<{sm_class}State, {sm_class}Event>() {{\n"
        f"{create_sm}"
        f"    override val expectedPassState: {sm_class}State = {sm_class}State.{pass_state}\n"
        f"{timeout_override}"
        f"}}\n"
    )

    test_file = TEST_OUTPUT_DIR / f'Test{to_pascal_case(test_id)}.kt'
    write_if_changed(test_file, content)
    return True


def clean_stale(valid_test_ids: set[str]) -> int:
    """
    Remove generated files for tests no longer in the CMake registry.

    Scans SM output directories and test class files, removing any that
    don't correspond to a valid (successfully generated) test ID.

    Returns number of stale entries removed.
    """
    removed = 0

    # Clean stale SM directories: SM_OUTPUT_BASE/testXXX/
    if SM_OUTPUT_BASE.exists():
        for d in SM_OUTPUT_BASE.iterdir():
            if not d.is_dir() or not d.name.startswith('test'):
                continue
            dir_test_id = d.name[len('test'):]
            if dir_test_id not in valid_test_ids:
                shutil.rmtree(d)
                print(f"  Removed stale SM dir: {d.name}")
                removed += 1

    # Clean stale test classes: TEST_OUTPUT_DIR/TestXxx.kt
    if TEST_OUTPUT_DIR.exists():
        for f in TEST_OUTPUT_DIR.glob('Test*.kt'):
            if f.name == 'W3CTestBase.kt':
                continue
            # Extract test ID: "Test144.kt" -> "144", "Test553B.kt" -> "553b" (case-insensitive match)
            stem = f.stem  # e.g., "Test144"
            file_test_id = stem[len('Test'):]
            # Match against valid IDs (case-sensitive as stored in cmake_tests)
            if file_test_id not in valid_test_ids and file_test_id.lower() not in {t.lower() for t in valid_test_ids}:
                f.unlink()
                print(f"  Removed stale test: {f.name}")
                removed += 1

    return removed


def clean_generated():
    """Remove all generated files."""
    if SM_OUTPUT_BASE.exists():
        shutil.rmtree(SM_OUTPUT_BASE)
        print(f"Cleaned: {SM_OUTPUT_BASE}")

    # Remove generated test classes (but NOT W3CTestBase.kt)
    for f in TEST_OUTPUT_DIR.glob('Test*.kt'):
        f.unlink()
    print(f"Cleaned test classes in: {TEST_OUTPUT_DIR}")


def main():
    parser = argparse.ArgumentParser(description='Generate Kotlin W3C SCXML tests')
    parser.add_argument('--test', '-t', help='Generate single test by ID (e.g., 144)')
    parser.add_argument('--clean', action='store_true', help='Remove all generated files')
    parser.add_argument('--list', action='store_true', help='List all tests without generating')
    args = parser.parse_args()

    if args.clean:
        clean_generated()
        return

    # Parse C++ CMakeLists for test registry (source of truth)
    cmake_tests = parse_cmake_tests()
    print(f"C++ test registry: {len(cmake_tests)} tests")

    if args.list:
        for tid, info in sorted(cmake_tests.items(), key=lambda x: x[0]):
            scxml = find_scxml(tid)
            status = 'OK' if scxml else 'MISSING'
            print(f"  {tid:6s} [{info['type']:9s}] {status} -- {info['comment'][:70]}")
        return

    # Filter to single test if requested
    if args.test:
        test_ids = [args.test]
    else:
        test_ids = sorted(cmake_tests.keys(), key=lambda x: (len(x), x))

    generated_static = []
    generated_script = []
    skipped_other = []
    failed = []

    for test_id in test_ids:
        scxml = find_scxml(test_id)
        if not scxml:
            skipped_other.append((test_id, 'SCXML not found'))
            continue

        if test_id in CODEGEN_EXCLUDE:
            skipped_other.append((test_id, 'known codegen bug'))
            continue

        # Single codegen call: generates output for both static and script engine tests
        result, needs_script = generate_sm(test_id, scxml)

        if result == 'failed':
            failed.append((test_id, 'codegen failed'))
            continue

        metadata = read_metadata(test_id)
        test_type = cmake_tests.get(test_id, {}).get('type', 'SIMPLE')

        if generate_test_class(test_id, metadata, test_type, needs_script):
            if needs_script:
                generated_script.append(test_id)
            else:
                generated_static.append(test_id)
        else:
            failed.append((test_id, 'pass state not detected'))

    # Clean stale files (full generation mode only)
    stale_removed = 0
    if not args.test:
        valid_ids = set(generated_static) | set(generated_script)
        if valid_ids:
            stale_removed = clean_stale(valid_ids)

    # Summary
    total_generated = len(generated_static) + len(generated_script)
    print(f"\n{'='*60}")
    print(f"Kotlin W3C Test Generation Summary")
    print(f"{'='*60}")
    print(f"  Generated (pure static):    {len(generated_static)}")
    print(f"  Generated (script engine):  {len(generated_script)}")
    print(f"  Generated (total):          {total_generated}")
    print(f"  Skipped (other):            {len(skipped_other)}")
    print(f"  Failed:                     {len(failed)}")
    print(f"  Stale removed:              {stale_removed}")
    print(f"  Total:                      {len(test_ids)}")

    if skipped_other:
        print(f"\nSkipped (other):")
        for tid, reason in skipped_other:
            print(f"  {tid}: {reason}")

    if failed:
        print(f"\nFailed tests:")
        for tid, reason in failed:
            print(f"  {tid}: {reason}")

    if total_generated:
        print(f"\nGenerated SM classes: {SM_OUTPUT_BASE}")
        print(f"Generated test classes: {TEST_OUTPUT_DIR}")
        print(f"\nGenerated test IDs (static): {' '.join(generated_static)}")
        if generated_script:
            print(f"Generated test IDs (script): {' '.join(generated_script)}")


if __name__ == '__main__':
    main()
