#!/bin/bash
# Generate AOT test headers without native build
# Reads test numbers from CMakeLists.txt and generates headers using Python codegen

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="${1:-build/tests/w3c_static_generated}"

echo "=== AOT Test Header Generation ==="
echo "Script directory: $SCRIPT_DIR"
echo "Project root: $PROJECT_ROOT"
echo "Output directory: $OUTPUT_DIR"
echo ""

# Verify required files exist
if [ ! -f "$PROJECT_ROOT/tests/CMakeLists.txt" ]; then
    echo "Error: tests/CMakeLists.txt not found at $PROJECT_ROOT/tests/CMakeLists.txt"
    exit 1
fi

if [ ! -f "$PROJECT_ROOT/tools/codegen/codegen.py" ]; then
    echo "Error: codegen.py not found at $PROJECT_ROOT/tools/codegen/codegen.py"
    exit 1
fi

# Create output directory (absolute path)
OUTPUT_DIR="$PROJECT_ROOT/$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
echo "Output directory (absolute): $OUTPUT_DIR"
echo ""

# Extract test numbers from CMakeLists.txt
# Method 1: W3C_AOT_TESTS list
AOT_TESTS=$(grep -A 100 'set(W3C_AOT_TESTS' "$PROJECT_ROOT/tests/CMakeLists.txt" | \
            sed -n '/set(W3C_AOT_TESTS/,/^)/p' | \
            grep -v 'set(W3C_AOT_TESTS' | \
            grep -v '^)' | \
            tr ' ' '\n' | \
            grep -E '^[0-9]+')

# Method 2: sce_generate_static_w3c_test() calls
GENERATED_TESTS=$(grep 'sce_generate_static_w3c_test(' "$PROJECT_ROOT/tests/CMakeLists.txt" | \
                  sed 's/.*sce_generate_static_w3c_test(\([0-9]*\).*/\1/')

# Combine and deduplicate
TEST_NUMBERS=$(echo -e "$AOT_TESTS\n$GENERATED_TESTS" | \
               grep -E '^[0-9]+$' | \
               sort -n -u)

TOTAL=$(echo "$TEST_NUMBERS" | wc -l)
COUNT=0
SUCCESS=0
FAILED=0

echo "Found $TOTAL AOT tests to generate"
echo ""

for TEST_NUM in $TEST_NUMBERS; do
    COUNT=$((COUNT + 1))
    TEST_DIR="$PROJECT_ROOT/resources/$TEST_NUM"

    if [ ! -d "$TEST_DIR" ]; then
        echo "[$COUNT/$TOTAL] Skip test$TEST_NUM - directory not found"
        FAILED=$((FAILED + 1))
        continue
    fi

    echo -n "[$COUNT/$TOTAL] Generating test$TEST_NUM... "

    # Generate ALL SCXML files in test directory
    SCXML_COUNT=0
    for SCXML_FILE in "$TEST_DIR"/*.scxml; do
        if [ ! -f "$SCXML_FILE" ]; then
            continue
        fi

        FILENAME=$(basename "$SCXML_FILE" .scxml)

        # Skip standalone machineName.scxml (it's a child file)
        if [ "$FILENAME" = "machineName" ]; then
            continue
        fi

        # Determine if this is a child/sub state machine
        IS_CHILD=false
        if [[ "$FILENAME" =~ _child[0-9]+$ ]] || [[ "$FILENAME" =~ sub[0-9]+$ ]]; then
            IS_CHILD=true
        fi

        # Generate header
        if [ "$IS_CHILD" = true ]; then
            python3 "$PROJECT_ROOT/tools/codegen/codegen.py" \
                "$SCXML_FILE" \
                -o "$OUTPUT_DIR" \
                --as-child >/dev/null 2>&1 && SCXML_COUNT=$((SCXML_COUNT + 1))
        else
            python3 "$PROJECT_ROOT/tools/codegen/codegen.py" \
                "$SCXML_FILE" \
                -o "$OUTPUT_DIR" >/dev/null 2>&1 && SCXML_COUNT=$((SCXML_COUNT + 1))
        fi
    done

    if [ $SCXML_COUNT -gt 0 ]; then
        echo "✓ ($SCXML_COUNT files)"
        SUCCESS=$((SUCCESS + 1))
    else
        echo "✗ (no SCXML files)"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "=== Generation Summary ==="
echo "Total tests: $TOTAL"
echo "Success: $SUCCESS"
echo "Failed/Skipped: $FAILED"
echo ""
echo "Generated files in: $OUTPUT_DIR"

HEADER_COUNT=$(ls -1 "$OUTPUT_DIR"/*.h 2>/dev/null | wc -l)
echo "Total headers: $HEADER_COUNT"

if [ "$HEADER_COUNT" -eq 0 ]; then
    echo ""
    echo "ERROR: No headers were generated!"
    exit 1
fi

echo ""
echo "✓ AOT header generation complete!"
