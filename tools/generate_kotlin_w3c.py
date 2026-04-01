#!/usr/bin/env python3
"""
Kotlin W3C SCXML Test Generator

Generates Kotlin state machine code and JUnit5 test classes for all
W3C SCXML conformance tests that can be statically compiled to Kotlin.

Usage:
    python3 tools/generate_kotlin_w3c.py [--test TEST_ID] [--clean]

Output:
    sce-kotlin-tests/src/main/kotlin/com/sce/generated/testXXX/testXXXSm.kt
    sce-kotlin-tests/src/test/kotlin/com/sce/w3c/TestXXX.kt
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
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

# Tests with known Kotlin codegen bugs (to be fixed in templates)
# 364: space-separated initial states in compound state
# 409, 411: empty event name -> "Empty" reference unresolved
CODEGEN_EXCLUDE = {'364', '409', '411'}


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


def generate_sm(test_id: str, scxml_path: Path) -> str:
    """
    Run Kotlin codegen for a single test.

    Returns:
        'generated' — SM file written successfully
        'script_engine' — needs script engine, skipped
        'failed' — codegen error
    """
    output_dir = SM_OUTPUT_BASE / f'test{test_id}'
    output_dir.mkdir(parents=True, exist_ok=True)

    result = subprocess.run(
        [
            sys.executable, str(CODEGEN_SCRIPT),
            str(scxml_path),
            '-o', str(output_dir),
            '--language', 'kotlin',
        ],
        capture_output=True,
        text=True,
        env={**os.environ, 'SPDLOG_LEVEL': 'warn'},
    )

    stdout = result.stdout
    if 'Needs ScriptEngine: True' in stdout:
        shutil.rmtree(output_dir, ignore_errors=True)
        return 'script_engine'
    if 'Generated:' in stdout:
        return 'generated'
    return 'failed'


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

    # Find which states are final (markFinalStateReached() or isInFinalState = true)
    final_states = set()
    for m in re.finditer(
        r'is \w+State\.(\w+)\s*->\s*\{[\s\S]*?(?:markFinalStateReached\(\)|isInFinalState\s*=\s*true)',
        content,
    ):
        final_states.add(m.group(1))

    # Priority: Pass > Final > first non-Fail final state
    if 'Pass' in final_states:
        return 'Pass'
    if 'Final' in final_states:
        return 'Final'
    non_fail = [s for s in final_states if s.lower() != 'fail']
    if len(non_fail) == 1:
        return non_fail[0]

    return None


def generate_test_class(test_id: str, metadata: dict, test_type: str) -> bool:
    """Generate a JUnit5 test class for a W3C test. Returns False if pass state not detected."""
    pass_state = detect_pass_state(test_id)
    if not pass_state:
        return False

    sm_class = f'Test{to_pascal_case(test_id)}'
    sm_package = f'test{test_id}'
    specnum = metadata.get('specnum', '')
    description = metadata.get('description', '')

    if test_type == 'SCHEDULED':
        timeout_override = '    override val timeoutMs: Long = 10_000L\n'
    else:
        timeout_override = ''

    content = (
        f"// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)\n"
        f"package com.sce.w3c\n"
        f"\n"
        f"import com.sce.generated.{sm_package}.{sm_class}Event\n"
        f"import com.sce.generated.{sm_package}.{sm_class}State\n"
        f"import com.sce.generated.{sm_package}.{sm_class}StateMachine\n"
        f"import org.junit.jupiter.api.DisplayName\n"
        f"\n"
        f"// W3C SCXML {specnum}: {description}\n"
        f"@DisplayName(\"Test {test_id} -- W3C SCXML {specnum}\")\n"
        f"class Test{to_pascal_case(test_id)} : W3CTestBase<{sm_class}State, {sm_class}Event>() {{\n"
        f"    override fun createStateMachine() = {sm_class}StateMachine()\n"
        f"    override val expectedPassState: {sm_class}State = {sm_class}State.{pass_state}\n"
        f"{timeout_override}"
        f"}}\n"
    )

    test_file = TEST_OUTPUT_DIR / f'Test{to_pascal_case(test_id)}.kt'
    test_file.parent.mkdir(parents=True, exist_ok=True)
    with open(test_file, 'w') as f:
        f.write(content)
    return True


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

    generated = []
    skipped_script = []
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

        # Single codegen call: generates output AND detects script engine
        result = generate_sm(test_id, scxml)

        if result == 'script_engine':
            skipped_script.append(test_id)
            continue
        elif result == 'failed':
            failed.append((test_id, 'codegen failed'))
            continue

        metadata = read_metadata(test_id)
        test_type = cmake_tests.get(test_id, {}).get('type', 'SIMPLE')

        if generate_test_class(test_id, metadata, test_type):
            generated.append(test_id)
        else:
            failed.append((test_id, 'pass state not detected'))

    # Summary
    print(f"\n{'='*60}")
    print(f"Kotlin W3C Test Generation Summary")
    print(f"{'='*60}")
    print(f"  Generated (pure static):    {len(generated)}")
    print(f"  Skipped (script engine):    {len(skipped_script)}")
    print(f"  Skipped (other):            {len(skipped_other)}")
    print(f"  Failed:                     {len(failed)}")
    print(f"  Total:                      {len(test_ids)}")

    if skipped_other:
        print(f"\nSkipped (other):")
        for tid, reason in skipped_other:
            print(f"  {tid}: {reason}")

    if failed:
        print(f"\nFailed tests:")
        for tid, reason in failed:
            print(f"  {tid}: {reason}")

    if generated:
        print(f"\nGenerated SM classes: {SM_OUTPUT_BASE}")
        print(f"Generated test classes: {TEST_OUTPUT_DIR}")
        print(f"\nGenerated test IDs: {' '.join(generated)}")


if __name__ == '__main__':
    main()
