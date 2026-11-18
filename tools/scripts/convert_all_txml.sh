#!/bin/bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

# Convert all TXML files to SCXML for visualizer deployment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RESOURCES_DIR="$PROJECT_ROOT/resources"
TXML_CONVERTER="$PROJECT_ROOT/build/tools/txml_converter/txml-converter"

# Check if txml-converter exists
if [ ! -f "$TXML_CONVERTER" ]; then
    echo "Error: txml-converter not found at $TXML_CONVERTER"
    echo "Please build the project first: cd build && cmake --build ."
    exit 1
fi

echo "Converting all TXML files to SCXML..."
echo "Project root: $PROJECT_ROOT"
echo "Resources dir: $RESOURCES_DIR"

converted=0
skipped=0
failed=0

cd "$RESOURCES_DIR"

# A directory is one IRP test number. Its fixtures are `test<number>.txml` and,
# for a test the IRP ships as variants, `test<number><letter>.txml` beside it
# (resources/403/test403a.txml is fixture 403a): a variant has no directory of
# its own (docs/SCE_W3C_TEST_REGISTRATION.md).
for dir in */; do
    test_num=${dir%/}
    for txml_file in "$dir"test"${test_num}".txml "$dir"test"${test_num}"[a-z].txml; do
        [ -f "$txml_file" ] || continue
        fixture=$(basename "$txml_file" .txml)
        fixture_id=${fixture#test}
        scxml_file="$dir$fixture.scxml"
        if [ -f "$scxml_file" ]; then
            echo "[$fixture_id] SCXML already exists, skipping"
            skipped=$((skipped + 1))
        else
            echo "[$fixture_id] Converting TXML -> SCXML"
            if "$TXML_CONVERTER" "$txml_file" "$scxml_file" 2>/dev/null; then
                converted=$((converted + 1))
            else
                echo "[$fixture_id] ERROR: Conversion failed"
                failed=$((failed + 1))
            fi
        fi
    done
done

echo ""
echo "Summary:"
echo "  Converted: $converted"
echo "  Skipped (already exists): $skipped"
echo "  Failed: $failed"
echo ""

if [ $failed -eq 0 ]; then
    echo "✓ All TXML files successfully converted"
    exit 0
else
    echo "✗ Some conversions failed"
    exit 1
fi
