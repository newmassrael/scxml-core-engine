"""
Cross-Implementation Test Vectors for EcmaScriptToLuaTransformer

Validates the Python transformer against shared test vectors.
C++ and Kotlin implementations should run the same vectors
to guarantee synchronized transformation output.

Vectors file: tests/ecmascript_to_lua_vectors.json
"""

import json
from pathlib import Path

import pytest

from ecmascript_to_lua import EcmaScriptToLuaTransformer, ExpressionContext

VECTORS_PATH = Path(__file__).parent / 'ecmascript_to_lua_vectors.json'
VECTORS = json.loads(VECTORS_PATH.read_text())


@pytest.mark.parametrize(
    "case",
    VECTORS["expressions"],
    ids=lambda c: f"{c['context']}:{c['input'][:50]}"
)
def test_expression_vector(case):
    """Verify expression transformation matches shared vector."""
    t = EcmaScriptToLuaTransformer()
    ctx = ExpressionContext[case["context"]]
    result = t.transform(case["input"], ctx)
    assert result == case["expected"], (
        f"Input: {case['input']!r} ({case['context']})\n"
        f"Expected: {case['expected']!r}\n"
        f"Got:      {result!r}"
    )


@pytest.mark.parametrize(
    "case",
    VECTORS["scripts"],
    ids=lambda c: c["input"][:50]
)
def test_script_vector(case):
    """Verify script transformation matches shared vector."""
    t = EcmaScriptToLuaTransformer()
    result = t.transform_script(case["input"])
    assert result == case["expected"], (
        f"Input: {case['input']!r}\n"
        f"Expected: {case['expected']!r}\n"
        f"Got:      {result!r}"
    )
