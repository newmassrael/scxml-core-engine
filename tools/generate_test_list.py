#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""
Generate web/visualizer/test-list.js from the W3C conformance registry.

Zero Duplication: tests/w3c/conformance/fixtures.json is the single source
of truth for which W3C tests this repository runs, and this script renders
the visualizer's navigation list from it.

It used to read `set(W3C_AOT_TESTS ...)` out of tests/CMakeLists.txt. That
variable is initialised empty and accumulated by the registration macro at
CMake *configure* time, so the literal in the file holds nothing: the
extraction matched `set(W3C_AOT_TESTS "")`, returned zero tests, and would
have rewritten test-list.js with an empty array — silently emptying the
visualizer's navigation. Reading the registry removes both the parse of a
build script as data and the failure mode.

Usage:
    python3 tools/generate_test_list.py [registry_file] [output_file]

Default:
    python3 tools/generate_test_list.py tests/w3c/conformance/fixtures.json \
        web/visualizer/test-list.js
"""

import json
import re
import sys
from pathlib import Path

# A registry that parsed to fewer than this many tests is a broken read,
# not a shrunken suite. Without the floor the failure is silent: the file
# is rewritten with a short (or empty) array and the visualizer simply
# stops offering most tests.
MIN_TESTS = 150


def extract_w3c_aot_tests(registry_file):
    """
    Read the registered test ids from the W3C conformance registry.

    Args:
        registry_file: Path to tests/w3c/conformance/fixtures.json

    Returns:
        List of test numbers (as strings, e.g., ['144', '403a', '403b'])
    """
    with open(registry_file, 'r', encoding='utf-8') as f:
        registry = json.load(f)

    fixtures = registry.get('fixtures')
    if not isinstance(fixtures, list):
        raise ValueError(f"{registry_file} declares no `fixtures` array")

    tests = [f['id'] for f in fixtures]
    if len(tests) < MIN_TESTS:
        raise ValueError(
            f"{registry_file} yielded {len(tests)} test(s); expected at least "
            f"{MIN_TESTS}. Refusing to rewrite the visualizer list from what "
            f"looks like a broken read."
        )
    return tests


def format_js_array(tests):
    """
    Format test list as JavaScript array

    Args:
        tests: List of test numbers

    Returns:
        Formatted JavaScript array string with proper line breaks
    """
    # Group tests by hundreds for readability
    grouped = []
    current_group = []
    current_hundred = None

    for test in tests:
        # Extract numeric part for grouping
        numeric_part = int(re.match(r'^(\d+)', test).group(1))
        hundred = numeric_part // 100

        if current_hundred is None:
            current_hundred = hundred

        if hundred != current_hundred:
            grouped.append(current_group)
            current_group = []
            current_hundred = hundred

        # Add quotes for alphanumeric tests (403a, 403b, etc.)
        if re.match(r'^\d+[a-z]$', test):
            current_group.append(f"'{test}'")
        else:
            current_group.append(test)

    if current_group:
        grouped.append(current_group)

    # Format as multi-line array
    lines = []
    for group in grouped:
        lines.append('    ' + ', '.join(group) + ',')

    # Remove trailing comma from last line
    if lines:
        lines[-1] = lines[-1].rstrip(',')

    return '\n'.join(lines)


def generate_test_list_js(tests):
    """
    Generate complete test-list.js content

    Args:
        tests: List of test numbers

    Returns:
        Complete JavaScript file content
    """
    js_array = format_js_array(tests)

    return f"""// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * W3C SCXML Test List
 *
 * ⚠️ AUTO-GENERATED - DO NOT EDIT MANUALLY
 * Source: tests/w3c/conformance/fixtures.json
 * Generator: tools/generate_test_list.py
 *
 * Zero Duplication: this file is generated from the W3C conformance
 * registry. To update the test list, edit the registry and run:
 *     python3 tools/generate_test_list.py
 * Or rebuild with CMake (automatic generation via custom command).
 */

const W3C_TEST_LIST = [
{js_array}
];

/**
 * Get current test number from URL hash
 */
function getCurrentTestNumber() {{
    const params = {{}};
    const hash = window.location.hash.substring(1);

    hash.split('&').forEach(param => {{
        const [key, value] = param.split('=');
        if (key && value) {{
            params[key] = decodeURIComponent(value);
        }}
    }});

    return params.test;
}}

/**
 * Navigate to test by number
 */
function navigateToTest(testNumber) {{
    window.location.hash = `test=${{testNumber}}`;
    window.location.reload();
}}

/**
 * Get previous test number
 */
function getPreviousTest() {{
    const currentTest = getCurrentTestNumber();
    if (!currentTest) return null;

    const currentIndex = W3C_TEST_LIST.indexOf(currentTest);
    if (currentIndex === -1) {{
        const currentNum = parseInt(currentTest);
        const numIndex = W3C_TEST_LIST.indexOf(currentNum);
        if (numIndex > 0) {{
            return W3C_TEST_LIST[numIndex - 1];
        }}
        return null;
    }}

    if (currentIndex > 0) {{
        return W3C_TEST_LIST[currentIndex - 1];
    }}

    return null;
}}

/**
 * Get next test number
 */
function getNextTest() {{
    const currentTest = getCurrentTestNumber();
    if (!currentTest) return null;

    const currentIndex = W3C_TEST_LIST.indexOf(currentTest);
    if (currentIndex === -1) {{
        const currentNum = parseInt(currentTest);
        const numIndex = W3C_TEST_LIST.indexOf(currentNum);
        if (numIndex >= 0 && numIndex < W3C_TEST_LIST.length - 1) {{
            return W3C_TEST_LIST[numIndex + 1];
        }}
        return null;
    }}

    if (currentIndex < W3C_TEST_LIST.length - 1) {{
        return W3C_TEST_LIST[currentIndex + 1];
    }}

    return null;
}}

/**
 * Initialize test navigation buttons
 */
function initializeTestNavigation() {{
    const btnPrevTest = document.getElementById('btn-prev-test');
    const btnNextTest = document.getElementById('btn-next-test');

    if (btnPrevTest) {{
        const prevTest = getPreviousTest();
        if (prevTest) {{
            btnPrevTest.disabled = false;
            btnPrevTest.addEventListener('click', () => {{
                navigateToTest(prevTest);
            }});
        }} else {{
            btnPrevTest.disabled = true;
        }}
    }}

    if (btnNextTest) {{
        const nextTest = getNextTest();
        if (nextTest) {{
            btnNextTest.disabled = false;
            btnNextTest.addEventListener('click', () => {{
                navigateToTest(nextTest);
            }});
        }} else {{
            btnNextTest.disabled = true;
        }}
    }}

    console.log(`[Test Navigation] Current: ${{getCurrentTestNumber()}}, Prev: ${{getPreviousTest()}}, Next: ${{getNextTest()}}`);
}}

// Initialize on DOMContentLoaded
window.addEventListener('DOMContentLoaded', initializeTestNavigation);
"""


def main():
    """Main entry point"""
    # Parse arguments
    registry_file = (
        Path(sys.argv[1]) if len(sys.argv) > 1
        else Path('tests/w3c/conformance/fixtures.json')
    )
    output_file = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('web/visualizer/test-list.js')

    # Validate input file exists
    if not registry_file.exists():
        print(f"Error: {registry_file} not found", file=sys.stderr)
        sys.exit(1)

    # Extract tests
    try:
        tests = extract_w3c_aot_tests(registry_file)
        print(f"Extracted {len(tests)} tests from {registry_file}")
    except Exception as e:
        print(f"Error extracting tests: {e}", file=sys.stderr)
        sys.exit(1)

    # Generate JavaScript
    js_content = generate_test_list_js(tests)

    # Write output
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(js_content)

    print(f"Generated {output_file} successfully")
    print(f"Test range: {tests[0]} - {tests[-1]}")


if __name__ == '__main__':
    main()
