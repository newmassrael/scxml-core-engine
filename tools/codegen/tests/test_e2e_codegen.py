"""
End-to-end code generation tests: SCXML -> generated code -> pattern verification.

Verifies that template changes produce correct output by running the full
codegen pipeline on real W3C SCXML test files and checking structural invariants
in the generated code. Catches regressions that unit tests on individual
utility functions cannot detect.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from generators.kotlin_generator import KotlinCodeGenerator
from generators.cpp_generator import CppCodeGenerator

# Project root for locating SCXML resources
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
RESOURCES = PROJECT_ROOT / "resources"


# ===========================================================================
# Shared helpers
# ===========================================================================

def _generate_kotlin(tmp_path_factory, test_num):
    """Generate Kotlin code for a W3C test and return the output content."""
    scxml_path = RESOURCES / str(test_num) / f"test{test_num}.scxml"
    assert scxml_path.exists(), f"SCXML file not found: {scxml_path}"
    out = tmp_path_factory.mktemp(f"kt_{test_num}")
    gen = KotlinCodeGenerator()
    success = gen.generate(str(scxml_path), str(out))
    assert success, f"Kotlin generation failed for test{test_num}"
    output = out / f"test{test_num}Sm.kt"
    assert output.exists(), f"Expected output not found: {output}"
    return output.read_text()


def _generate_cpp(tmp_path_factory, test_num):
    """Generate C++ code for a W3C test and return header + inline content."""
    scxml_path = RESOURCES / str(test_num) / f"test{test_num}.scxml"
    assert scxml_path.exists(), f"SCXML file not found: {scxml_path}"
    out = tmp_path_factory.mktemp(f"cpp_{test_num}")
    gen = CppCodeGenerator()
    success = gen.generate(str(scxml_path), str(out))
    assert success, f"C++ generation failed for test{test_num}"
    header = out / f"test{test_num}_sm.h"
    inline = out / f"test{test_num}_sm.inl"
    assert header.exists(), f"Expected header not found: {header}"
    assert inline.exists(), f"Expected inline not found: {inline}"
    return header.read_text(), inline.read_text()


# ===========================================================================
# Kotlin: Simple state machine structure (test144 — raise + transition)
# ===========================================================================

class TestKotlinSimpleStateMachine:

    @pytest.fixture(scope="class")
    def content(self, tmp_path_factory):
        return _generate_kotlin(tmp_path_factory, 144)

    def test_generation_succeeds(self, content):
        assert len(content) > 0

    def test_state_sealed_interface(self, content):
        assert "sealed interface Test144State" in content
        assert "data object S0" in content
        assert "data object S1" in content
        assert "data object Pass" in content
        assert "data object Fail" in content

    def test_event_sealed_interface(self, content):
        assert "sealed interface Test144Event" in content

    def test_raise_internal(self, content):
        assert "raiseInternal(" in content

    def test_no_script_engine_for_pure_static(self, content):
        assert "ensureScriptEngine" not in content


# ===========================================================================
# Kotlin: BasicHTTP content send (test520 — <content> element)
# ===========================================================================

class TestKotlinBasicHttpContent:

    @pytest.fixture(scope="class")
    def content(self, tmp_path_factory):
        return _generate_kotlin(tmp_path_factory, 520)

    def test_generation_succeeds(self, content):
        assert len(content) > 0

    def test_perform_http_send_call(self, content):
        assert "performHttpSend(" in content

    def test_http_target(self, content):
        assert '"http://localhost:8080/test"' in content

    def test_content_passed(self, content):
        assert "this is some content" in content

    def test_no_schedule_http_send(self, content):
        """test520 has no delay on HTTP send."""
        assert "scheduleHttpSend(" not in content


# ===========================================================================
# Kotlin: BasicHTTP with params (test518 — namelist evaluation)
# ===========================================================================

class TestKotlinBasicHttpParams:

    @pytest.fixture(scope="class")
    def content(self, tmp_path_factory):
        return _generate_kotlin(tmp_path_factory, 518)

    def test_generation_succeeds(self, content):
        assert len(content) > 0

    def test_perform_http_send_with_params(self, content):
        assert "performHttpSend(" in content
        assert "httpParams" in content

    def test_namelist_evaluation(self, content):
        assert "hasVariable(" in content
        assert "getVariable(" in content

    def test_script_engine_required(self, content):
        assert "ensureScriptEngine()" in content


# ===========================================================================
# Kotlin: BasicHTTP static params (test531)
# ===========================================================================

class TestKotlinBasicHttpStaticParams:

    @pytest.fixture(scope="class")
    def content(self, tmp_path_factory):
        return _generate_kotlin(tmp_path_factory, 531)

    def test_generation_succeeds(self, content):
        assert len(content) > 0

    def test_static_params_in_map(self, content):
        assert "httpParams" in content
        assert "listOf(" in content

    def test_perform_http_send(self, content):
        assert "performHttpSend(" in content


# ===========================================================================
# Kotlin: BasicHTTP event-only send (test534)
# ===========================================================================

class TestKotlinBasicHttpEventOnly:

    @pytest.fixture(scope="class")
    def content(self, tmp_path_factory):
        return _generate_kotlin(tmp_path_factory, 534)

    def test_generation_succeeds(self, content):
        assert len(content) > 0

    def test_perform_http_send_with_event(self, content):
        assert "performHttpSend(" in content
        assert "emptyMap()" in content


# ===========================================================================
# C++: Simple state machine structure (test144)
# ===========================================================================

class TestCppSimpleStateMachine:

    @pytest.fixture(scope="class")
    def generated(self, tmp_path_factory):
        return _generate_cpp(tmp_path_factory, 144)

    def test_generation_succeeds(self, generated):
        header, inline = generated
        assert len(header) > 0
        assert len(inline) > 0

    def test_state_enum(self, generated):
        header, _ = generated
        assert "enum class State" in header
        assert "S0" in header
        assert "S1" in header
        assert "Pass" in header
        assert "Fail" in header

    def test_event_enum(self, generated):
        header, _ = generated
        assert "enum class Event" in header


# ===========================================================================
# C++: BasicHTTP send (test520 — content)
# ===========================================================================

class TestCppBasicHttpContent:

    @pytest.fixture(scope="class")
    def generated(self, tmp_path_factory):
        return _generate_cpp(tmp_path_factory, 520)

    def test_generation_succeeds(self, generated):
        header, _ = generated
        assert len(header) > 0

    def test_perform_http_send(self, generated):
        _, inline = generated
        assert "performHttpSend(" in inline

    def test_http_target(self, generated):
        _, inline = generated
        assert '"http://localhost:8080/test"' in inline


# ===========================================================================
# Cross-language: Both generators produce output for the same SCXML
# ===========================================================================

class TestCrossLanguageParity:

    @pytest.mark.parametrize("test_num", [144, 520, 518, 531, 534])
    def test_both_languages_succeed(self, tmp_path, test_num):
        """Both C++ and Kotlin generation should succeed for all BasicHTTP tests."""
        scxml_path = RESOURCES / str(test_num) / f"test{test_num}.scxml"
        if not scxml_path.exists():
            pytest.skip(f"SCXML file not found: {scxml_path}")
        kt_dir = tmp_path / "kt"
        cpp_dir = tmp_path / "cpp"
        kt_dir.mkdir()
        cpp_dir.mkdir()
        kt_ok = KotlinCodeGenerator().generate(str(scxml_path), str(kt_dir))
        cpp_ok = CppCodeGenerator().generate(str(scxml_path), str(cpp_dir))
        assert kt_ok, f"Kotlin generation failed for test{test_num}"
        assert cpp_ok, f"C++ generation failed for test{test_num}"

    @pytest.mark.parametrize("test_num", [518, 520, 531, 534])
    def test_basic_http_parity(self, tmp_path, test_num):
        """Both languages should generate performHttpSend for BasicHTTP tests."""
        scxml_path = RESOURCES / str(test_num) / f"test{test_num}.scxml"
        if not scxml_path.exists():
            pytest.skip(f"SCXML file not found: {scxml_path}")

        kt_dir = tmp_path / "kt"
        cpp_dir = tmp_path / "cpp"
        kt_dir.mkdir()
        cpp_dir.mkdir()

        KotlinCodeGenerator().generate(str(scxml_path), str(kt_dir))
        CppCodeGenerator().generate(str(scxml_path), str(cpp_dir))

        kt_content = (kt_dir / f"test{test_num}Sm.kt").read_text()
        cpp_inline = (cpp_dir / f"test{test_num}_sm.inl").read_text()

        assert "performHttpSend(" in kt_content, f"Kotlin missing performHttpSend for test{test_num}"
        assert "performHttpSend(" in cpp_inline, f"C++ missing performHttpSend for test{test_num}"
