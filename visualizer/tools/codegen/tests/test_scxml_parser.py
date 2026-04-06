"""
Unit tests for scxml_parser.py

Covers: In() predicates, delay parsing, string literals, named context transforms,
script engine detection, and end-to-end SCXML parsing.
"""

import sys
from pathlib import Path

import pytest

# Add codegen directory to path so imports work
sys.path.insert(0, str(Path(__file__).parent.parent))

from scxml_parser import SCXMLParser, SCXMLModel, State, Transition


# ---------------------------------------------------------------------------
# Fixture: fresh parser instance
# ---------------------------------------------------------------------------

@pytest.fixture
def parser():
    """Create a fresh SCXMLParser with an empty model for utility method testing."""
    p = SCXMLParser()
    p.model = SCXMLModel(name="test")
    return p


# ===========================================================================
# _is_pure_in_predicate
# ===========================================================================

class TestIsPureInPredicate:
    """W3C SCXML 5.9.2: Detect expressions that are ONLY In() predicates."""

    def test_single_in(self, parser):
        assert parser._is_pure_in_predicate("In('s1')") is True

    def test_double_and(self, parser):
        assert parser._is_pure_in_predicate("In('s1') && In('s2')") is True

    def test_double_or(self, parser):
        assert parser._is_pure_in_predicate("In('s1') || In('s2')") is True

    def test_complex_combination(self, parser):
        assert parser._is_pure_in_predicate("(In('a') && In('b')) || In('c')") is True

    def test_xml_entity_and(self, parser):
        """W3C SCXML B.1: XML entity escaping — &amp;&amp; is decoded to &&."""
        assert parser._is_pure_in_predicate("In('s1') &amp;&amp; In('s2')") is True

    def test_mixed_with_typeof(self, parser):
        """Mixed In() + ECMAScript → not pure."""
        assert parser._is_pure_in_predicate("In('s1') && typeof x !== 'undefined'") is False

    def test_variable_argument(self, parser):
        """In(variable) instead of In('literal') → not pure."""
        assert parser._is_pure_in_predicate("In(stateName)") is False

    def test_empty_string(self, parser):
        assert parser._is_pure_in_predicate("") is False

    def test_no_in_at_all(self, parser):
        assert parser._is_pure_in_predicate("x > 5") is False

    def test_in_with_event_access(self, parser):
        assert parser._is_pure_in_predicate("In('s1') && _event.name == 'go'") is False


# ===========================================================================
# _convert_in_to_cpp / _convert_in_to_kotlin
# ===========================================================================

class TestConvertInPredicates:
    """W3C SCXML 5.9.2: Transform In() calls to language-specific isStateActive()."""

    def test_cpp_single(self, parser):
        assert parser._convert_in_to_cpp("In('s1')") == 'this->isStateActive("s1")'

    def test_cpp_double(self, parser):
        result = parser._convert_in_to_cpp("In('a') && In('b')")
        assert result == 'this->isStateActive("a") && this->isStateActive("b")'

    def test_cpp_xml_entities(self, parser):
        result = parser._convert_in_to_cpp("In('s1') &amp;&amp; In('s2')")
        assert result == 'this->isStateActive("s1") && this->isStateActive("s2")'

    def test_kotlin_single(self, parser):
        assert parser._convert_in_to_kotlin("In('s1')") == 'isStateActive("s1")'

    def test_kotlin_double_or(self, parser):
        result = parser._convert_in_to_kotlin("In('a') || In('b')")
        assert result == 'isStateActive("a") || isStateActive("b")'


# ===========================================================================
# _parse_delay_to_ms
# ===========================================================================

class TestParseDelayToMs:
    """W3C SCXML 6.2.5: CSS2 time format parsing."""

    def test_seconds(self, parser):
        assert parser._parse_delay_to_ms("1s") == 1000

    def test_fractional_seconds(self, parser):
        assert parser._parse_delay_to_ms("1.5s") == 1500

    def test_milliseconds(self, parser):
        assert parser._parse_delay_to_ms("200ms") == 200

    def test_bare_number_as_seconds(self, parser):
        assert parser._parse_delay_to_ms(".5") == 500

    def test_bare_integer(self, parser):
        assert parser._parse_delay_to_ms("2") == 2000

    def test_empty_string(self, parser):
        assert parser._parse_delay_to_ms("") == 0

    def test_whitespace(self, parser):
        assert parser._parse_delay_to_ms("  1s  ") == 1000

    def test_zero_ms(self, parser):
        assert parser._parse_delay_to_ms("0ms") == 0


# ===========================================================================
# _is_static_string_literal / _extract_static_string_literal
# ===========================================================================

class TestStaticStringLiterals:
    """W3C SCXML B.2: ECMAScript string literal detection for pure-static optimization."""

    def test_single_quoted(self, parser):
        assert parser._is_static_string_literal("'hello'") is True

    def test_double_quoted(self, parser):
        assert parser._is_static_string_literal('"world"') is True

    def test_variable(self, parser):
        assert parser._is_static_string_literal("varName") is False

    def test_empty_string(self, parser):
        assert parser._is_static_string_literal("") is False

    def test_string_with_escape(self, parser):
        """Strings with backslash escapes are NOT static (too complex for parse-time eval)."""
        assert parser._is_static_string_literal(r"'hello\nworld'") is False

    def test_extract_single(self, parser):
        assert parser._extract_static_string_literal("'test'") == "test"

    def test_extract_double(self, parser):
        assert parser._extract_static_string_literal('"value"') == "value"

    def test_extract_with_spaces(self, parser):
        assert parser._extract_static_string_literal("  'trimmed'  ") == "trimmed"


# ===========================================================================
# _requires_script_engine
# ===========================================================================

class TestRequiresScriptEngine:
    """Detect expressions that need JSEngine vs. static C++ codegen."""

    def test_simple_number(self, parser):
        assert parser._requires_script_engine("42") is False

    def test_simple_identifier(self, parser):
        assert parser._requires_script_engine("x") is False

    def test_typeof(self, parser):
        assert parser._requires_script_engine("typeof x !== 'undefined'") is True

    def test_event_access(self, parser):
        assert parser._requires_script_engine("_event.data") is True

    def test_comparison(self, parser):
        assert parser._requires_script_engine("x == 1") is True

    def test_pure_in_predicate(self, parser):
        """Pure In() should NOT require script engine."""
        assert parser._requires_script_engine("In('s1')") is False
        assert parser.model.uses_in_predicate is True

    def test_mixed_in_with_ecmascript(self, parser):
        assert parser._requires_script_engine("In('s1') && x > 0") is True

    def test_empty(self, parser):
        assert parser._requires_script_engine("") is False

    def test_underscore_identifier(self, parser):
        """System-reserved _variables need JSEngine."""
        assert parser._requires_script_engine("_name") is True

    def test_string_literal(self, parser):
        """String literals need JSEngine for proper boolean conversion."""
        assert parser._requires_script_engine("'foo'") is True

    def test_cpp_reserved_word(self, parser):
        assert parser._requires_script_engine("return") is True


# ===========================================================================
# _id_to_camel_case (static method)
# ===========================================================================

class TestIdToCamelCase:

    def test_snake_case(self):
        assert SCXMLParser._id_to_camel_case("my_var") == "myVar"

    def test_kebab_case(self):
        assert SCXMLParser._id_to_camel_case("my-var") == "myVar"

    def test_dot_separated(self):
        assert SCXMLParser._id_to_camel_case("my.var") == "myVar"

    def test_already_camel(self):
        assert SCXMLParser._id_to_camel_case("myVar") == "myVar"

    def test_empty(self):
        assert SCXMLParser._id_to_camel_case("") == ""

    def test_single_part(self):
        assert SCXMLParser._id_to_camel_case("hardware") == "hardware"


# ===========================================================================
# _transform_cpp_code_with_named_contexts / _transform_kt_code_with_named_contexts
# ===========================================================================

class TestNamedContextTransforms:

    def test_cpp_declared_context(self, parser):
        result = parser._transform_cpp_code_with_named_contexts(
            "hardware.powerOff()", {"hardware"}
        )
        assert result == "this->hardware_->powerOff()"

    def test_cpp_undeclared_unchanged(self, parser):
        result = parser._transform_cpp_code_with_named_contexts(
            "undeclared.method()", {"hardware"}
        )
        assert result == "undeclared.method()"

    def test_cpp_preserves_string_literals(self, parser):
        result = parser._transform_cpp_code_with_named_contexts(
            '"hardware.method()"', {"hardware"}
        )
        assert result == '"hardware.method()"'

    def test_kt_snake_case_context(self, parser):
        result = parser._transform_kt_code_with_named_contexts(
            "my_hardware.powerOff()", {"my_hardware"}
        )
        assert result == "myHardware.powerOff()"

    def test_kt_undeclared_unchanged(self, parser):
        result = parser._transform_kt_code_with_named_contexts(
            "undeclared.method()", {"my_hardware"}
        )
        assert result == "undeclared.method()"


# ===========================================================================
# End-to-end: parse_file with real SCXML
# ===========================================================================

class TestParseFile:
    """Integration tests using actual W3C test SCXML files."""

    RESOURCES = Path(__file__).parent.parent.parent.parent / "resources"

    def test_parse_test144(self):
        """test144: simple raise events, no script engine needed."""
        scxml = self.RESOURCES / "144" / "test144.scxml"
        if not scxml.exists():
            pytest.skip("test144.scxml not found")

        parser = SCXMLParser()
        model = parser.parse_file(str(scxml))

        assert model.name == "test144"
        assert model.initial == "s0"
        assert "s0" in model.states
        assert "s1" in model.states
        assert "pass" in model.states
        assert "fail" in model.states
        assert model.states["pass"].is_final is True
        assert model.states["fail"].is_final is True

        # s0 has two transitions: event="foo" and event="*"
        s0 = model.states["s0"]
        assert len(s0.transitions) == 2
        assert s0.transitions[0].event == "foo"
        assert s0.transitions[0].target == "s1"
        assert s0.transitions[1].event == "*"
        assert s0.transitions[1].target == "fail"

        # Events detected
        assert "foo" in model.events
        assert "bar" in model.events

    def test_parse_test201_http_send(self):
        """test201: BasicHTTP send — parser extracts send_type and target."""
        scxml = self.RESOURCES / "201" / "test201.scxml"
        if not scxml.exists():
            pytest.skip("test201.scxml not found")

        parser = SCXMLParser()
        model = parser.parse_file(str(scxml))

        assert model.name == "test201"
        assert "event1" in model.events
        assert "timeout" in model.events

        # s0 entry has two send actions
        s0 = model.states["s0"]
        assert len(s0.on_entry_blocks) == 1
        actions = s0.on_entry_blocks[0]
        assert len(actions) == 2
        assert actions[0]["type"] == "send"
        assert actions[0]["send_type"] == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
        assert actions[0]["target"] == "http://localhost:8080/test"
        assert actions[1]["type"] == "send"
        assert actions[1]["event"] == "timeout"

    def test_parse_detects_parallel(self):
        """test310: parallel states."""
        scxml = self.RESOURCES / "310" / "test310.scxml"
        if not scxml.exists():
            pytest.skip("test310.scxml not found")

        parser = SCXMLParser()
        model = parser.parse_file(str(scxml))

        assert model.has_parallel_states is True
