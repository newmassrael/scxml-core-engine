"""
Unit tests for generators/kotlin_generator.py

Covers: name conversion filters, event tree building, Kotlin type mapping,
string escaping, and event class name generation.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from generators.kotlin_generator import KotlinCodeGenerator


# ===========================================================================
# _to_pascal_case
# ===========================================================================

class TestToPascalCase:

    def test_lowercase(self):
        assert KotlinCodeGenerator._to_pascal_case("stopped") == "Stopped"

    def test_snake_case(self):
        assert KotlinCodeGenerator._to_pascal_case("my_state") == "MyState"

    def test_hyphen_case(self):
        assert KotlinCodeGenerator._to_pascal_case("my-state") == "MyState"

    def test_dot_separated(self):
        assert KotlinCodeGenerator._to_pascal_case("error.execution") == "ErrorExecution"

    def test_short_id(self):
        assert KotlinCodeGenerator._to_pascal_case("s0") == "S0"

    def test_already_pascal(self):
        assert KotlinCodeGenerator._to_pascal_case("Pass") == "Pass"

    def test_empty(self):
        assert KotlinCodeGenerator._to_pascal_case("") == "Empty"

    def test_reserved_words(self):
        assert KotlinCodeGenerator._to_pascal_case("pass") == "Pass"
        assert KotlinCodeGenerator._to_pascal_case("fail") == "Fail"


# ===========================================================================
# _to_camel_case
# ===========================================================================

class TestToCamelCase:

    def test_snake_case(self):
        assert KotlinCodeGenerator._to_camel_case("track_index") == "trackIndex"

    def test_already_camel(self):
        assert KotlinCodeGenerator._to_camel_case("myVar") == "myVar"

    def test_multi_part(self):
        assert KotlinCodeGenerator._to_camel_case("a_b_c") == "aBC"

    def test_empty(self):
        assert KotlinCodeGenerator._to_camel_case("") == ""

    def test_single(self):
        assert KotlinCodeGenerator._to_camel_case("name") == "name"


# ===========================================================================
# _to_kotlin_type
# ===========================================================================

class TestToKotlinType:

    def test_int(self):
        assert KotlinCodeGenerator._to_kotlin_type("int") == "Int"

    def test_string(self):
        assert KotlinCodeGenerator._to_kotlin_type("string") == "String"

    def test_bool(self):
        assert KotlinCodeGenerator._to_kotlin_type("bool") == "Boolean"

    def test_runtime(self):
        assert KotlinCodeGenerator._to_kotlin_type("runtime") == "Any"

    def test_unknown(self):
        assert KotlinCodeGenerator._to_kotlin_type("unknown") == "Any"


# ===========================================================================
# _to_kotlin_default
# ===========================================================================

class TestToKotlinDefault:

    def test_int_with_value(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "int", "expr": "5"}) == "5"

    def test_int_no_value(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "int", "expr": ""}) == "0"

    def test_string_quoted(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "string", "expr": '"hello"'}) == '"hello"'

    def test_string_no_value(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "string", "expr": ""}) == '""'

    def test_bool_true(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "bool", "expr": "true"}) == "true"

    def test_bool_invalid(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "bool", "expr": "maybe"}) == "false"

    def test_runtime(self):
        assert KotlinCodeGenerator._to_kotlin_default({"type": "runtime"}) == "null"


# ===========================================================================
# _escape_kotlin_string
# ===========================================================================

class TestEscapeKotlinString:

    def test_plain(self):
        assert KotlinCodeGenerator._escape_kotlin_string("hello") == "hello"

    def test_quotes(self):
        assert KotlinCodeGenerator._escape_kotlin_string('say "hi"') == 'say \\"hi\\"'

    def test_dollar(self):
        assert KotlinCodeGenerator._escape_kotlin_string("$var") == "\\$var"

    def test_backslash(self):
        assert KotlinCodeGenerator._escape_kotlin_string("a\\b") == "a\\\\b"

    def test_newline(self):
        assert KotlinCodeGenerator._escape_kotlin_string("a\nb") == "a\\nb"

    def test_empty(self):
        assert KotlinCodeGenerator._escape_kotlin_string("") == ""


# ===========================================================================
# _to_kotlin_string_expr
# ===========================================================================

class TestToKotlinStringExpr:

    def test_single_quoted(self):
        assert KotlinCodeGenerator._to_kotlin_string_expr("'hello'") == '"hello"'

    def test_single_with_inner_double(self):
        result = KotlinCodeGenerator._to_kotlin_string_expr("'say \"hi\"'")
        assert result == '"say \\"hi\\""'

    def test_variable_passthrough(self):
        assert KotlinCodeGenerator._to_kotlin_string_expr("varName") == "varName"

    def test_empty(self):
        assert KotlinCodeGenerator._to_kotlin_string_expr("") == '""'

    def test_dollar_in_string(self):
        assert KotlinCodeGenerator._to_kotlin_string_expr("'price $5'") == '"price \\$5"'


# ===========================================================================
# _to_event_class_name
# ===========================================================================

class TestToEventClassName:

    def test_simple(self):
        assert KotlinCodeGenerator._to_event_class_name("play") == "Play"

    def test_underscore(self):
        assert KotlinCodeGenerator._to_event_class_name("switch_on") == "SwitchOn"

    def test_dotted(self):
        assert KotlinCodeGenerator._to_event_class_name("error.execution") == "Error.Execution"

    def test_deep_dotted(self):
        assert KotlinCodeGenerator._to_event_class_name("done.state.s1") == "Done.State.S1"

    def test_empty(self):
        assert KotlinCodeGenerator._to_event_class_name("") == "Empty"


# ===========================================================================
# _to_state_class_name
# ===========================================================================

class TestToStateClassName:

    def test_simple(self):
        assert KotlinCodeGenerator._to_state_class_name("stopped") == "Stopped"

    def test_short_id(self):
        assert KotlinCodeGenerator._to_state_class_name("s1") == "S1"

    def test_underscore(self):
        assert KotlinCodeGenerator._to_state_class_name("my_state") == "MyState"

    def test_empty(self):
        assert KotlinCodeGenerator._to_state_class_name("") == "Empty"


# ===========================================================================
# _build_event_tree
# ===========================================================================

class TestBuildEventTree:

    def test_flat_events(self):
        tree = KotlinCodeGenerator._build_event_tree({"play", "pause", "stop"})
        assert tree["play"]["_leaf"] is True
        assert tree["pause"]["_leaf"] is True
        assert tree["stop"]["_leaf"] is True

    def test_hierarchical_events(self):
        tree = KotlinCodeGenerator._build_event_tree(
            {"error.execution", "error.communication"}
        )
        assert "error" in tree
        assert tree["error"]["_leaf"] is False
        assert tree["error"]["execution"]["_leaf"] is True
        assert tree["error"]["communication"]["_leaf"] is True

    def test_branch_and_leaf(self):
        """Event that is both a leaf and a parent (e.g., 'error' + 'error.execution')."""
        tree = KotlinCodeGenerator._build_event_tree(
            {"error", "error.execution"}
        )
        assert tree["error"]["_leaf"] is True  # leaf: 'error' event exists
        assert tree["error"]["execution"]["_leaf"] is True  # child exists too

    def test_empty(self):
        tree = KotlinCodeGenerator._build_event_tree(set())
        assert tree == {}


# ===========================================================================
# _collect_leaf_events
# ===========================================================================

class TestCollectLeafEvents:

    def test_flat(self):
        tree = KotlinCodeGenerator._build_event_tree({"a", "b", "c"})
        leaves = sorted(KotlinCodeGenerator._collect_leaf_events(tree))
        assert leaves == ["a", "b", "c"]

    def test_hierarchical(self):
        tree = KotlinCodeGenerator._build_event_tree({"error.execution", "error.communication", "done"})
        leaves = sorted(KotlinCodeGenerator._collect_leaf_events(tree))
        assert leaves == ["done", "error.communication", "error.execution"]

    def test_branch_leaf(self):
        """Branch event that is also a leaf should appear in result."""
        tree = KotlinCodeGenerator._build_event_tree({"error", "error.execution"})
        leaves = sorted(KotlinCodeGenerator._collect_leaf_events(tree))
        assert "error" in leaves
        assert "error.execution" in leaves


# ===========================================================================
# _collect_branch_events
# ===========================================================================

class TestCollectBranchEvents:

    def test_no_branches(self):
        tree = KotlinCodeGenerator._build_event_tree({"play", "pause"})
        branches = KotlinCodeGenerator._collect_branch_events(tree)
        assert branches == set()

    def test_pure_hierarchy(self):
        """Parent that is NOT a leaf → not a branch event."""
        tree = KotlinCodeGenerator._build_event_tree({"error.execution"})
        branches = KotlinCodeGenerator._collect_branch_events(tree)
        assert branches == set()

    def test_branch_and_leaf(self):
        """'error' is both leaf and parent → needs .Self suffix."""
        tree = KotlinCodeGenerator._build_event_tree({"error", "error.execution"})
        branches = KotlinCodeGenerator._collect_branch_events(tree)
        assert "error" in branches

    def test_deep_branch(self):
        tree = KotlinCodeGenerator._build_event_tree(
            {"done", "done.state", "done.state.s1"}
        )
        branches = KotlinCodeGenerator._collect_branch_events(tree)
        assert "done" in branches
        assert "done.state" in branches


# ===========================================================================
# _render_event_tree
# ===========================================================================

class TestRenderEventTree:

    def test_leaf_only(self):
        tree = KotlinCodeGenerator._build_event_tree({"play"})
        rendered = KotlinCodeGenerator._render_event_tree(tree, "TestEvent")
        assert "data object Play : TestEvent" in rendered

    def test_sealed_interface(self):
        tree = KotlinCodeGenerator._build_event_tree({"error.execution"})
        rendered = KotlinCodeGenerator._render_event_tree(tree, "TestEvent")
        assert "sealed interface Error : TestEvent" in rendered
        assert "data object Execution : Error" in rendered

    def test_self_suffix(self):
        """Branch+leaf gets Self data object."""
        tree = KotlinCodeGenerator._build_event_tree({"error", "error.execution"})
        rendered = KotlinCodeGenerator._render_event_tree(tree, "TestEvent")
        assert "data object Self : Error" in rendered
