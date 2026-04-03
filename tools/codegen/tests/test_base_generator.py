"""
Unit tests for generators/base.py

Covers: variable classification, static generation eligibility check.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from scxml_parser import SCXMLModel, State
from generators.kotlin_generator import KotlinCodeGenerator


@pytest.fixture
def gen():
    """Create a KotlinCodeGenerator as concrete BaseCodeGenerator subclass."""
    return KotlinCodeGenerator()


# ===========================================================================
# _classify_variables
# ===========================================================================

class TestClassifyVariables:

    def test_integer(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": "42"}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "int"

    def test_zero(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": "0"}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "int"

    def test_string(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": '"hello"'}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "string"

    def test_bool_true(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": "true"}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "bool"

    def test_bool_false(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": "false"}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "bool"

    def test_runtime_expression(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": "someFunction()"}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "runtime"
        assert model.needs_script_engine is True

    def test_no_expr(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [{"id": "x", "expr": ""}]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "runtime"

    def test_mixed(self, gen):
        model = SCXMLModel(name="test")
        model.variables = [
            {"id": "count", "expr": "5"},
            {"id": "name", "expr": '"Alice"'},
            {"id": "flag", "expr": "true"},
            {"id": "computed", "expr": "x + 1"},
        ]
        gen._classify_variables(model)
        assert model.variables[0]["type"] == "int"
        assert model.variables[1]["type"] == "string"
        assert model.variables[2]["type"] == "bool"
        assert model.variables[3]["type"] == "runtime"


# ===========================================================================
# _can_generate_static
# ===========================================================================

class TestCanGenerateStatic:

    def test_valid_initial(self, gen):
        model = SCXMLModel(name="test", initial="s0")
        model.states = {"s0": State(id="s0"), "s1": State(id="s1")}
        assert gen._can_generate_static(model) is True

    def test_missing_initial(self, gen):
        model = SCXMLModel(name="test", initial="")
        model.states = {"s0": State(id="s0")}
        assert gen._can_generate_static(model) is False

    def test_initial_not_in_states(self, gen):
        model = SCXMLModel(name="test", initial="nonexistent")
        model.states = {"s0": State(id="s0")}
        assert gen._can_generate_static(model) is False

    def test_space_separated_initial(self, gen):
        """W3C SCXML 3.4: Parallel initial with multiple state targets."""
        model = SCXMLModel(name="test", initial="s0 s1")
        model.states = {"s0": State(id="s0"), "s1": State(id="s1")}
        assert gen._can_generate_static(model) is True

    def test_space_separated_partial_missing(self, gen):
        model = SCXMLModel(name="test", initial="s0 s2")
        model.states = {"s0": State(id="s0"), "s1": State(id="s1")}
        assert gen._can_generate_static(model) is False
