"""
Unit tests for generators/cpp_generator.py

Covers: state name capitalization, C++ string escaping.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from generators.cpp_generator import CppCodeGenerator


@pytest.fixture
def gen():
    """Create a CppCodeGenerator instance (templates not loaded, just for filter testing)."""
    # __init__ tries to load templates; bypass by constructing minimally
    g = object.__new__(CppCodeGenerator)
    return g


# ===========================================================================
# _capitalize_state
# ===========================================================================

class TestCapitalizeState:

    def test_lowercase(self, gen):
        assert gen._capitalize_state("s0") == "S0"

    def test_word(self, gen):
        assert gen._capitalize_state("stopped") == "Stopped"

    def test_pass_reserved(self, gen):
        assert gen._capitalize_state("pass") == "Pass"

    def test_fail_reserved(self, gen):
        assert gen._capitalize_state("fail") == "Fail"

    def test_pass_uppercase(self, gen):
        assert gen._capitalize_state("PASS") == "Pass"

    def test_empty(self, gen):
        """W3C SCXML C.2: Empty event for content-only send."""
        assert gen._capitalize_state("") == "Empty"

    def test_already_capitalized(self, gen):
        assert gen._capitalize_state("Start") == "Start"


# ===========================================================================
# _escape_cpp_string
# ===========================================================================

class TestEscapeCppString:

    def test_plain(self, gen):
        assert gen._escape_cpp_string("hello") == "hello"

    def test_quotes(self, gen):
        assert gen._escape_cpp_string('say "hi"') == 'say \\"hi\\"'

    def test_backslash(self, gen):
        assert gen._escape_cpp_string("a\\b") == "a\\\\b"

    def test_newline(self, gen):
        assert gen._escape_cpp_string("a\nb") == "a\\nb"

    def test_tab(self, gen):
        assert gen._escape_cpp_string("a\tb") == "a\\tb"

    def test_carriage_return(self, gen):
        assert gen._escape_cpp_string("a\rb") == "a\\rb"

    def test_empty(self, gen):
        assert gen._escape_cpp_string("") == ""
