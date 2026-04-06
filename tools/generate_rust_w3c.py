#!/usr/bin/env python3
"""
Rust W3C SCXML Test Generator

Generates Rust state machine code and per-test integration tests for all
W3C SCXML conformance tests that can be statically generated.

Usage:
    python3 tools/generate_rust_w3c.py [--test TEST_ID] [--clean] [--list]

Output:
    sce-rust-tests/src/generated/testNNN/testNNN_sm.rs   (generated SM)
    sce-rust-tests/src/generated/testNNN/mod.rs           (module re-export)
    sce-rust-tests/src/generated/mod.rs                   (root module declarations)
    sce-rust-tests/tests/test_NNN.rs                      (integration test)
"""

from __future__ import annotations

import argparse
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
TESTS_CRATE = PROJECT_ROOT / 'sce-rust-tests'
SM_OUTPUT_BASE = TESTS_CRATE / 'src' / 'generated'
TEST_OUTPUT_DIR = TESTS_CRATE / 'tests'

# C++ CMakeLists.txt -- source of truth for test IDs and types
CMAKE_FILE = PROJECT_ROOT / 'tests' / 'CMakeLists.txt'

# Tests excluded from Rust codegen (structural issues only)
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


def copy_if_changed(src_dir: Path, dst_dir: Path):
    """Copy files from src to dst only if content differs."""
    dst_dir.mkdir(parents=True, exist_ok=True)
    for src_file in src_dir.iterdir():
        if src_file.is_file():
            content = src_file.read_text()
            write_if_changed(dst_dir / src_file.name, content)


def generate_sm(test_id: str, scxml_path: Path) -> tuple[str, bool]:
    """
    Run Rust codegen for a single test AND its child SMs (invoke children).
    Uses a temp dir to avoid touching timestamps on unchanged files.

    Returns:
        ('generated', needs_script_engine) -- SM file written successfully
        ('failed', False) -- codegen error
    """
    output_dir = SM_OUTPUT_BASE / f'test{test_id}'

    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        result = subprocess.run(
            [
                sys.executable, str(CODEGEN_SCRIPT),
                str(scxml_path),
                '-o', str(tmp_dir),
                '--language', 'rust',
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

    # W3C SCXML 6.4: Generate child SMs for invoke elements
    _generate_invoke_children(test_id, scxml_path)

    return 'generated', needs_script


def _generate_invoke_children(test_id: str, parent_scxml: Path):
    """
    W3C SCXML 6.4: Detect and generate child state machines for <invoke> elements.

    Scans parent SCXML model for static_invokes and generates each child SM
    into the same test module directory (e.g., test252/test252_child0_sm.rs).
    """
    # Parse parent model to find invoke children
    sys_path_backup = sys.path[:]
    sys.path.insert(0, str(PROJECT_ROOT / 'tools' / 'codegen'))
    try:
        from scxml_parser import SCXMLParser
        parser = SCXMLParser()
        model = parser.parse_file(str(parent_scxml))
    except Exception:
        return
    finally:
        sys.path[:] = sys_path_backup

    parent_dir = parent_scxml.parent
    output_dir = SM_OUTPUT_BASE / f'test{test_id}'

    # W3C SCXML 6.4: Generate children for hybrid invokes (Rust AOT pre-generation)
    if model.hybrid_invokes:
        _generate_hybrid_invoke_children(test_id, parent_scxml, model, output_dir)

    if not model.static_invokes:
        return

    for invoke_info in model.static_invokes:
        child_name = invoke_info.get('child_name', '')
        child_src = invoke_info.get('src', '')
        if not child_name or not child_src:
            continue

        # Strip file: prefix (W3C SCXML 6.4.3)
        if child_src.startswith('file:'):
            child_src = child_src[len('file:'):]

        child_scxml = parent_dir / child_src
        if not child_scxml.exists():
            print(f"  Warning: child SCXML not found: {child_scxml}")
            continue

        with tempfile.TemporaryDirectory() as tmp:
            tmp_dir = Path(tmp)
            result = subprocess.run(
                [
                    sys.executable, str(CODEGEN_SCRIPT),
                    str(child_scxml),
                    '-o', str(tmp_dir),
                    '--language', 'rust',
                ],
                capture_output=True,
                text=True,
                env={**os.environ, 'SPDLOG_LEVEL': 'warn'},
            )

            if 'Generated:' not in result.stdout:
                print(f"  Warning: child codegen failed for {child_name}")
                continue

            copy_if_changed(tmp_dir, output_dir)


def _generate_hybrid_invoke_children(test_id: str, parent_scxml: Path,
                                     model, output_dir: Path):
    """
    W3C SCXML 6.4: Generate child SMs for hybrid invoke elements (srcexpr/contentexpr).

    Rust AOT backend pre-generates children at codegen time since there is no
    runtime SCXML interpreter. For srcexpr: finds child SCXML in resource dir.
    For contentexpr: generates a trivial child that immediately reaches final state.
    """
    parent_dir = parent_scxml.parent

    for invoke_info in model.hybrid_invokes:
        child_name = invoke_info.get('child_name', '')
        if not child_name:
            continue

        srcexpr = invoke_info.get('srcexpr', '')
        contentexpr = invoke_info.get('contentexpr', '')

        child_scxml = None

        if srcexpr:
            # For srcexpr: scan resource directory for child SCXML files
            # Convention: any .scxml file that isn't the parent is a potential child
            for f in sorted(parent_dir.glob('*.scxml')):
                if f.name != parent_scxml.name:
                    child_scxml = f
                    break

        if contentexpr or (srcexpr and child_scxml is None):
            # For contentexpr or when srcexpr child not found: generate trivial child
            # W3C SCXML: Trivial child immediately reaches final state (done.invoke)
            trivial_content = (
                '<?xml version="1.0"?>\n'
                '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
                f'name="{child_name}" initial="final" version="1.0">\n'
                '  <final id="final"/>\n'
                '</scxml>\n'
            )
            child_scxml = parent_dir / f'{child_name}.scxml'
            child_scxml.write_text(trivial_content)

        if child_scxml is None or not child_scxml.exists():
            print(f"  Warning: hybrid child SCXML not found for {child_name}")
            continue

        with tempfile.TemporaryDirectory() as tmp:
            tmp_dir = Path(tmp)
            result = subprocess.run(
                [
                    sys.executable, str(CODEGEN_SCRIPT),
                    str(child_scxml),
                    '-o', str(tmp_dir),
                    '--language', 'rust',
                ],
                capture_output=True,
                text=True,
                env={**os.environ, 'SPDLOG_LEVEL': 'warn'},
            )

            if 'Generated:' not in result.stdout:
                print(f"  Warning: hybrid child codegen failed for {child_name}")
                continue

            # Rename the generated file to match the expected child_name
            # The codegen names the file after the SCXML input, but the parent
            # references the convention-based child_name.
            actual_stem = child_scxml.stem  # e.g., "test216sub1"
            actual_file = tmp_dir / f'{actual_stem}_sm.rs'
            expected_file = tmp_dir / f'{child_name}_sm.rs'
            if actual_file.exists() and actual_file != expected_file:
                # Read, transform internal names, and write with new name
                content = actual_file.read_text()
                # Replace the Pascal/snake case names: Test216Sub1 -> Test216Hybrid0
                actual_pascal = to_pascal_case(actual_stem)
                expected_pascal = to_pascal_case(child_name)
                content = content.replace(actual_pascal, expected_pascal)
                content = content.replace(actual_stem, child_name)
                expected_file.write_text(content)
                actual_file.unlink()

            copy_if_changed(tmp_dir, output_dir)

        # Clean up generated trivial SCXML if we created it
        if contentexpr or (srcexpr and not any(parent_dir.glob(f'*sub*.scxml'))):
            generated_file = parent_dir / f'{child_name}.scxml'
            if generated_file.exists() and child_name.startswith(f'test{test_id}_hybrid'):
                generated_file.unlink()


def detect_pass_state(test_id: str) -> str | None:
    """
    Detect the pass state name from the generated Rust SM code.

    W3C convention: "pass"/"fail" final states. Some tests use "final" only.
    Returns PascalCase state variant name or None if not detectable.
    """
    sm_file = SM_OUTPUT_BASE / f'test{test_id}' / f'test{test_id}_sm.rs'
    if not sm_file.exists():
        return None

    content = sm_file.read_text()

    # Find which states are final from the is_final_state() match arms
    # Pattern: `SomeState::Variant => true,` inside is_final_state
    final_states = set()
    in_is_final = False
    for line in content.splitlines():
        stripped = line.strip()
        if 'fn is_final_state' in line:
            in_is_final = True
            continue
        if in_is_final:
            if stripped.startswith('}'):
                # End of match block or function
                in_is_final = False
                continue
            m = re.match(r'\w+State::(\w+)\s*=>\s*true', stripped)
            if m:
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


def generate_test_module(test_id: str) -> bool:
    """
    Generate the per-test mod.rs that re-exports the generated SM.
    Includes child SM modules for invoke support (W3C SCXML 6.4).

    Creates: sce-rust-tests/src/generated/testNNN/mod.rs
    """
    output_dir = SM_OUTPUT_BASE / f'test{test_id}'

    # Discover child SM files (e.g., test252_child0_sm.rs)
    child_modules = []
    if output_dir.exists():
        for f in sorted(output_dir.iterdir()):
            if f.name.endswith('_sm.rs') and f.name != f'test{test_id}_sm.rs':
                mod_name = f.stem  # e.g., "test252_child0_sm"
                child_modules.append(mod_name)

    lines = [
        f"// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)",
        f"",
        f"#[allow(dead_code, unused_variables, unused_imports, clippy::all)]",
        f"mod test{test_id}_sm;",
        f"pub use test{test_id}_sm::*;",
    ]

    # W3C SCXML 6.4: Child SM modules for invoke support
    for child_mod in child_modules:
        lines.append(f"")
        lines.append(f"#[allow(dead_code, unused_variables, unused_imports, clippy::all)]")
        lines.append(f"pub mod {child_mod};")

    lines.append("")  # trailing newline

    mod_content = "\n".join(lines)
    mod_file = output_dir / 'mod.rs'
    return write_if_changed(mod_file, mod_content)


def generate_root_mod(valid_test_ids: list[str]):
    """
    Generate the root generated/mod.rs with all test module declarations.

    Creates: sce-rust-tests/src/generated/mod.rs
    """
    sorted_ids = sorted(valid_test_ids, key=lambda x: (len(x), x))

    lines = [
        "// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)",
        "//! Generated W3C SCXML conformance test state machines.",
        "//!",
        "//! This module is populated by `python3 tools/generate_rust_w3c.py` from the",
        "//! W3C test corpus under `resources/*.scxml`.",
        "//!",
        f"//! Contains {len(sorted_ids)} generated test modules.",
        "",
    ]

    for test_id in sorted_ids:
        lines.append(f"pub mod test{test_id};")

    lines.append("")  # trailing newline

    content = "\n".join(lines)
    write_if_changed(SM_OUTPUT_BASE / 'mod.rs', content)


def generate_integration_test(test_id: str, pass_state: str | None,
                              test_type: str, needs_script: bool = False) -> bool:
    """
    Generate a Rust integration test file for a W3C test.

    Creates: sce-rust-tests/tests/test_NNN.rs
    """
    machine_name = f"Test{to_pascal_case(test_id)}"
    mod_name = f"test{test_id}"

    # W3C SCXML 6.2: SCHEDULED tests need longer timeout for delayed send/invoke
    timeout_secs = 5 if test_type == 'SCHEDULED' else 3

    # Build assertions
    if pass_state:
        final_assertion = (
            f"    assert_eq!(\n"
            f"        engine.get_current_state(),\n"
            f"        sce_rust_tests::generated::{mod_name}::{machine_name}State::{pass_state},\n"
            f"        \"Test {test_id} reached wrong final state\"\n"
            f"    );\n"
        )
    else:
        final_assertion = (
            f"    assert!(\n"
            f"        sce_rust_runtime::StatePolicy::is_final_state(engine.get_current_state()),\n"
            f"        \"Test {test_id} did not reach final state\"\n"
            f"    );\n"
        )

    # Script engine registration for tests that need it
    script_init = ""
    if needs_script:
        script_init = "    let _ = sce_rust_lua::register();\n"

    # W3C SCXML C.2: HTTP tests need loopback server simulation
    http_init = ""
    if test_type == 'HTTP':
        http_init = "    engine.enable_http_loopback();\n"

    content = (
        f"// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)\n"
        f"use std::time::Duration;\n"
        f"\n"
        f"#[test]\n"
        f"fn test_{test_id}() {{\n"
        f"{script_init}"
        f"    let policy = sce_rust_tests::generated::{mod_name}::{machine_name}Policy::new();\n"
        f"    let mut engine = sce_rust_runtime::Engine::new(policy);\n"
        f"{http_init}"
        f"    engine.initialize();\n"
        f"    let completed = engine.run_until_completion(\n"
        f"        Duration::from_secs({timeout_secs}),\n"
        f"        Duration::from_millis(10),\n"
        f"    );\n"
        f"    assert!(completed, \"Test {test_id} timed out\");\n"
        f"{final_assertion}"
        f"}}\n"
    )

    test_file = TEST_OUTPUT_DIR / f'test_{test_id}.rs'
    return write_if_changed(test_file, content)


def clean_stale(valid_test_ids: set[str]) -> int:
    """
    Remove generated files for tests no longer in the registry.

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

    # Clean stale integration tests: TEST_OUTPUT_DIR/test_NNN.rs
    if TEST_OUTPUT_DIR.exists():
        for f in TEST_OUTPUT_DIR.glob('test_*.rs'):
            # Extract test ID: "test_144.rs" -> "144"
            stem = f.stem  # e.g., "test_144"
            if not stem.startswith('test_'):
                continue
            file_test_id = stem[len('test_'):]
            if file_test_id not in valid_test_ids:
                f.unlink()
                print(f"  Removed stale test: {f.name}")
                removed += 1

    return removed


def clean_generated():
    """Remove all generated files."""
    # Clean SM directories (but preserve mod.rs placeholder)
    if SM_OUTPUT_BASE.exists():
        for d in SM_OUTPUT_BASE.iterdir():
            if d.is_dir() and d.name.startswith('test'):
                shutil.rmtree(d)
        print(f"Cleaned SM directories in: {SM_OUTPUT_BASE}")

    # Clean integration test files
    if TEST_OUTPUT_DIR.exists():
        for f in TEST_OUTPUT_DIR.glob('test_*.rs'):
            f.unlink()
        print(f"Cleaned test files in: {TEST_OUTPUT_DIR}")

    # Reset root mod.rs to empty
    empty_mod = (
        "// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial\n"
        "// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael\n"
        "\n"
        "//! Generated W3C SCXML conformance test state machines.\n"
        "//!\n"
        "//! This module is populated by `python3 tools/generate_rust_w3c.py` from the\n"
        "//! W3C test corpus under `resources/*.scxml`. Phase 1 ships empty -- Phase 2\n"
        "//! begins adding per-test submodules.\n"
    )
    write_if_changed(SM_OUTPUT_BASE / 'mod.rs', empty_mod)
    print(f"Reset: {SM_OUTPUT_BASE / 'mod.rs'}")


def main():
    parser = argparse.ArgumentParser(description='Generate Rust W3C SCXML tests')
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
            skipped_other.append((test_id, 'known codegen exclusion'))
            continue

        # Run Rust codegen
        result, needs_script = generate_sm(test_id, scxml)

        if result == 'failed':
            failed.append((test_id, 'codegen failed'))
            continue

        # Generate per-test mod.rs
        generate_test_module(test_id)

        # Detect pass state from generated SM
        pass_state = detect_pass_state(test_id)
        if not pass_state:
            # Still generate the test, but use is_final_state() only
            pass

        # Generate integration test file
        test_type = cmake_tests.get(test_id, {}).get('type', 'SIMPLE')
        generate_integration_test(test_id, pass_state, test_type, needs_script=needs_script)

        if needs_script:
            generated_script.append(test_id)
        else:
            generated_static.append(test_id)

    # Generate root mod.rs with all module declarations
    all_generated = generated_static + generated_script
    if all_generated or not args.test:
        if args.test:
            # Single test mode: read existing mod.rs and merge
            existing_ids = set()
            root_mod = SM_OUTPUT_BASE / 'mod.rs'
            if root_mod.exists():
                for line in root_mod.read_text().splitlines():
                    m = re.match(r'pub mod test(\w+);', line)
                    if m:
                        existing_ids.add(m.group(1))
            existing_ids.update(all_generated)
            generate_root_mod(sorted(existing_ids, key=lambda x: (len(x), x)))
        else:
            generate_root_mod(all_generated)

    # Clean stale files (full generation mode only)
    stale_removed = 0
    if not args.test:
        valid_ids = set(generated_static) | set(generated_script)
        if valid_ids:
            stale_removed = clean_stale(valid_ids)

    # Ensure tests/ directory exists
    TEST_OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Summary
    total_generated = len(generated_static) + len(generated_script)
    print(f"\n{'='*60}")
    print(f"Rust W3C Test Generation Summary")
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
        print(f"\nGenerated SM modules: {SM_OUTPUT_BASE}")
        print(f"Generated integration tests: {TEST_OUTPUT_DIR}")
        print(f"\nGenerated test IDs (static): {' '.join(generated_static)}")
        if generated_script:
            print(f"Generated test IDs (script): {' '.join(generated_script)}")


if __name__ == '__main__':
    main()
