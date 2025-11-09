#!/bin/bash
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

for dir in */; do
    test_num=${dir%/}
    txml_file="$dir/test${test_num}.txml"
    scxml_file="$dir/test${test_num}.scxml"
    
    if [ -f "$txml_file" ]; then
        if [ -f "$scxml_file" ]; then
            echo "[$test_num] SCXML already exists, skipping"
            skipped=$((skipped + 1))
        else
            echo "[$test_num] Converting TXML -> SCXML"
            if "$TXML_CONVERTER" "$txml_file" "$scxml_file" 2>/dev/null; then
                converted=$((converted + 1))
            else
                echo "[$test_num] ERROR: Conversion failed"
                failed=$((failed + 1))
            fi
        fi
    fi
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
