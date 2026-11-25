# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Cases for ``interface_standard.py``: each reader on a tree it is built for.

The gate's verdict is only as good as four readers: the guard scanner, the
flag cleaner, the include graph that pairs a header with the flags it is
compiled with, and the classifier that tells a failure from a dependency this
machine lacks. Each is driven here on a small tree written into a temporary
directory, so a reader that went blind fails a case instead of reporting a
clean interface. No case needs a C++ compiler: the one case that runs jobs end
to end uses this Python interpreter as the "compiler".

Run with ``python3 -m unittest discover -s tools/interface-standard/tests``.
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import interface_standard as tool  # noqa: E402  (the path above is what finds it)


def write(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def track(root: Path) -> None:
    """Make ``root`` a git repository whose index holds every file now in it.

    The tool asks git which repository files exist, so a case that wants a file
    to count has to stage it, and one that wants a file ignored leaves it out.
    """
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    for argv in (["git", "init", "-q", str(root)], ["git", "-C", str(root), "add", "-A"]):
        subprocess.run(argv, check=True, env=env, stdout=subprocess.DEVNULL)


def run_main(argv):
    """``(status, stdout, stderr)`` of the tool's command line."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        status = tool.main(argv)
    return status, out.getvalue(), err.getvalue()


class StandardsAreReadByYear(unittest.TestCase):
    def test_spellings_of_one_standard_agree(self):
        self.assertEqual(tool.std_year("c++17"), 2017)
        self.assertEqual(tool.std_year("gnu++1z"), 2017)
        self.assertEqual(tool.std_year("c++20"), 2020)
        self.assertEqual(tool.std_year("c++2a"), 2020)

    def test_a_typo_is_refused_rather_than_compiled(self):
        for bad in ("c++18", "c17", "c++", "cpp17"):
            with self.assertRaises(ValueError, msg=bad):
                tool.std_year(bad)

    def test_a_compile_feature_names_its_year(self):
        self.assertEqual(tool.feature_year("cxx_std_20"), 2020)
        self.assertIsNone(tool.feature_year("cxx_constexpr"))


class GuardsRefuseAStandardSelectingDirective(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()

    def tearDown(self):
        self._tmp.cleanup()

    def build_scopes(self):
        write(self.root / "sce/include/core/A.h",
              "#pragma once\n"
              "#if __cpp_concepts >= 202002L\n"
              "#endif\n"
              "#if defined(FOO) && \\\n"
              "    __cplusplus >= 202002L\n"
              "#endif\n"
              "#endif // __cplusplus\n"
              "#if BAR /* __cplusplus */\n"
              "#endif\n"
              "#ifndef __cpp_lib_span\n"
              "#endif\n")
        # The guard the logging macros once carried is refused like any other
        # (docs/adr/0060), and the allowed one is still refused beside another
        # macro in the same file.
        write(self.root / "sce/include/core/LogMacros.h",
              "#pragma once\n"
              "#if __cpp_lib_format >= 201907L\n"
              "#endif\n")
        write(self.root / "sce/include/common/JsonText.h",
              "#pragma once\n"
              "#if defined(__cpp_lib_to_chars) && __cpp_lib_to_chars >= 201611L\n"
              "#endif\n"
              "#if __cpp_concepts\n"
              "#endif\n")
        write(self.root / "backends/cpp/forge-runtime/include/sce/forge/ok.h",
              "#pragma once\n#if SCE_FORGE_X\n#endif\n")
        templates = self.root / "tools/codegen/templates"
        write(templates / "state_machine.jinja2",
              "{% if x %}\n#if __cpp_concepts >= 202002L\n#endif\n{% endif %}\n")
        write(templates / "forge/cpp/span_user.h.jinja2", "#  ifdef __cpp_lib_span\n#endif\n")
        # Subtrees that emit another language are not read.
        write(templates / "c/state_machine.h.jinja2", "#ifdef __cplusplus\n#endif\n")
        write(templates / "forge/c/queue.h.jinja2", "#ifdef __cplusplus\n#endif\n")
        write(templates / "mesh_router/kotlin/x.jinja2", "#if __cplusplus\n#endif\n")

    def test_every_standard_directive_is_refused_and_named_by_line(self):
        self.build_scopes()
        report = tool.check_guards(self.root)
        refused = {(g.path, g.line) for g in report.refused}
        self.assertEqual(refused, {
            ("sce/include/core/A.h", 2),
            ("sce/include/core/A.h", 4),
            ("sce/include/core/A.h", 10),
            ("sce/include/core/LogMacros.h", 2),
            ("sce/include/common/JsonText.h", 4),
            ("tools/codegen/templates/state_machine.jinja2", 2),
            ("tools/codegen/templates/forge/cpp/span_user.h.jinja2", 1),
        })
        self.assertEqual([(g.path, g.line) for g in report.allowed],
                         [("sce/include/common/JsonText.h", 2)])

    def test_the_allowlist_holds_the_one_library_feature_guard(self):
        # The logging macros' format guard was the other entry, and it went with
        # the logging split it sat in (docs/adr/0060).
        self.assertEqual(list(tool.GUARD_ALLOWLIST),
                         [("sce/include/common/JsonText.h", "__cpp_lib_to_chars")])

    def test_the_other_languages_template_subtrees_are_not_read(self):
        self.build_scopes()
        report = tool.check_guards(self.root)
        # A.h, JsonText.h, LogMacros.h, ok.h, state_machine.jinja2, span_user.h.jinja2.
        self.assertEqual(report.files, 6)
        self.assertEqual(report.missing_scopes, [])

    def test_an_allowance_that_matches_nothing_is_reported(self):
        self.build_scopes()
        (self.root / "sce/include/common/JsonText.h").unlink()
        report = tool.check_guards(self.root)
        self.assertEqual(report.unused_allowances,
                         [("sce/include/common/JsonText.h", "__cpp_lib_to_chars")])

    def test_the_command_prints_each_refusal_as_file_and_line(self):
        self.build_scopes()
        status, out, _ = run_main(["guards", "--root", str(self.root), "--min-files", "1"])
        self.assertEqual(status, 1)
        self.assertIn("sce/include/core/A.h:2: #if __cpp_concepts >= 202002L", out.splitlines())

    def test_a_tree_holding_only_the_allowed_guards_passes(self):
        write(self.root / "sce/include/common/JsonText.h",
              "#if defined(__cpp_lib_to_chars) && __cpp_lib_to_chars >= 201611L\n#endif\n")
        write(self.root / "backends/cpp/forge-runtime/include/sce/forge/ok.h", "#pragma once\n")
        write(self.root / "tools/codegen/templates/state_machine.jinja2", "{{ x }}\n")
        status, _, _ = run_main(["guards", "--root", str(self.root), "--min-files", "1"])
        self.assertEqual(status, 0)
        # The same tree read by a scan that expects the real tree's size is blind.
        status, _, err = run_main(["guards", "--root", str(self.root), "--min-files", "100"])
        self.assertEqual(status, 1)
        self.assertIn("read only 3 file(s)", err)

    def test_a_missing_scope_is_reported_not_skipped(self):
        write(self.root / "sce/include/core/LogMacros.h", "#pragma once\n")
        report = tool.check_guards(self.root)
        self.assertEqual(report.missing_scopes,
                         ["backends/cpp/forge-runtime/include", "tools/codegen/templates"])


class GenerationOutputsAreReadFromTheBuildGraph(unittest.TestCase):
    def test_only_custom_command_headers_inside_the_tree_are_kept(self):
        text = (
            "tests/generated/a_sm.h: CUSTOM_COMMAND\n"
            "tests/generated/a_sm.h.json: CUSTOM_COMMAND\n"
            "sce/generated/json_builtins_lua.h: CUSTOM_COMMAND\n"
            "tests/w3c/test100.scxml: CUSTOM_COMMAND\n"
            "all: phony\n"
            "sce/libsce_runtime.a: CXX_STATIC_LIBRARY_LINKER__sce_runtime_\n"
            "/build/tests/fragment.inc: CUSTOM_COMMAND\n"
            "/src/sce/src/mesh/generated/envelope.h: CUSTOM_COMMAND\n"
        )
        self.assertEqual(tool.generation_outputs(text, Path("/build")), [
            "sce/generated/json_builtins_lua.h",
            "tests/fragment.inc",
            "tests/generated/a_sm.h",
        ])


class FlagsAreTheTranslationUnitsWithoutItsOutputs(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()
        self.source = str(self.root / "sce/src/a.cpp")
        self.entry = tool.Entry(
            directory=str(self.root / "build/sce"), file=self.source,
            args=["/usr/bin/c++", "-DX=1", f"-I{self.root}/sce/include",
                  "-isystem", f"{self.root}/third_party/x", "-g", "-std=gnu++20",
                  "-include", "common/DisableStdOut.h",
                  "-o", "CMakeFiles/t.dir/a.cpp.o", "-c", self.source, "-MD", "-MF", "a.d"],
            target="t", declared_in="sce", language="c++")

    def tearDown(self):
        self._tmp.cleanup()

    def test_a_header_alone_drops_the_source_and_the_forced_includes(self):
        self.assertEqual(
            tool.compile_flags(self.entry, keep_source=False, keep_forced_includes=False,
                               substitute_std=True),
            ["-DX=1", f"-I{self.root}/sce/include", "-isystem", f"{self.root}/third_party/x",
             "-g", "-std=@STD@"])

    def test_a_translation_unit_keeps_its_source_and_its_forced_includes(self):
        self.assertEqual(
            tool.compile_flags(self.entry, keep_source=True, keep_forced_includes=True,
                               substitute_std=True),
            ["-DX=1", f"-I{self.root}/sce/include", "-isystem", f"{self.root}/third_party/x",
             "-g", "-std=@STD@", "-include", "common/DisableStdOut.h", self.source])

    def test_a_command_with_no_standard_gets_the_placeholder(self):
        entry = tool.Entry(self.entry.directory, self.source, ["c++", "-c", self.source],
                           "t", "sce", "c++")
        self.assertEqual(tool.compile_flags(entry, keep_source=False, keep_forced_includes=False,
                                            substitute_std=True), ["-std=@STD@"])

    def test_a_c_unit_keeps_its_own_standard(self):
        entry = tool.Entry(self.entry.directory, self.source,
                           ["cc", "-std=c11", "-c", self.source], "t", "backends", "c")
        self.assertEqual(tool.compile_flags(entry, keep_source=True, keep_forced_includes=True,
                                            substitute_std=False), ["-std=c11", self.source])

    def test_the_search_path_follows_the_compilers_order(self):
        entry = tool.Entry("/b", "/s/a.cpp",
                           ["c++", "-iquote", "q", "-I", "/a", "-I/b2", "-isystem", "/s",
                            "-idirafter", "/d", "-include", "x.h"], "t", "", "c++")
        search = tool.search_path(entry)
        self.assertEqual(search.quote, ("/b/q", "/a", "/b2", "/s", "/d"))
        self.assertEqual(search.angle, ("/a", "/b2", "/s", "/d"))
        self.assertEqual(tool.forced_includes(entry), ["x.h"])

    def test_a_launcher_is_not_the_compiler(self):
        self.assertEqual(tool.strip_launcher(["/usr/bin/ccache", "/usr/bin/c++", "-c"]),
                         ["/usr/bin/c++", "-c"])
        self.assertEqual(tool.strip_launcher(["/usr/bin/c++", "-c"]), ["/usr/bin/c++", "-c"])


class TheDeclaringTargetIsReadFromTheObjectPath(unittest.TestCase):
    def test_targets_and_their_directories(self):
        build = Path("/b")
        self.assertEqual(tool.target_of("/b/sce/CMakeFiles/sce_runtime.dir/src/x.cpp.o", build),
                         ("sce_runtime", "sce"))
        self.assertEqual(tool.target_of("/b/tests/CMakeFiles/w3c.dir/a.cpp.o", build),
                         ("w3c", "tests"))
        self.assertEqual(tool.target_of("/b/CMakeFiles/top.dir/a.cpp.o", build), ("top", ""))

    def test_only_targets_declared_under_sce_are_the_library(self):
        def entry(declared):
            return tool.Entry("/b", "/s/a.cpp", ["c++"], "t", declared, "c++")
        self.assertTrue(entry("sce").is_library)
        self.assertTrue(entry("sce/src/mesh").is_library)
        self.assertFalse(entry("tests").is_library)
        self.assertFalse(entry("scenery").is_library)


class AnImplementationStandardIsReadFromItsDeclaration(unittest.TestCase):
    def test_private_and_public_features_raise_the_floor_and_interface_does_not(self):
        text = (
            "target_compile_features(sce_runtime PUBLIC cxx_std_17 PRIVATE cxx_std_20)\n"
            "target_compile_features(sce_base PUBLIC cxx_std_17)\n"
            "target_compile_features(sce_core INTERFACE cxx_std_20)\n"
            "# target_compile_features(commented PRIVATE cxx_std_20)\n"
            "target_compile_features(multi\n"
            "    PUBLIC\n"
            "        cxx_std_17\n"
            "    PRIVATE\n"
            "        cxx_std_23)\n"
            "target_compile_features(${name} PRIVATE cxx_std_20)\n"
        )
        self.assertEqual(tool.implementation_floors([text]),
                         {"sce_runtime": 2020, "sce_base": 2017, "multi": 2023})


class EachHeaderIsPairedWithTheFlagsItNeeds(unittest.TestCase):
    """The include graph and the assignment, on a tree with one of each case."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        base = Path(self._tmp.name).resolve()
        self.root = base / "root"
        self.build = base / "build"
        mark = "// SCE-GENERATED -- DO NOT EDIT\n// source-hash: 0\n"
        inc = self.root / "sce/include"
        self.a = write(inc / "core/A.h", '#pragma once\n#include "core/B.h"\n')
        self.b = write(inc / "core/B.h", "#pragma once\n")
        self.unreached = write(inc / "core/Unreached.h", "#pragma once\n")
        self.lonely = write(inc / "lonely/C.h", "#pragma once\n")
        self.machine = write(self.build / "tests/gen/m_sm.h", mark)
        self.child = write(self.build / "tests/gen/child_sm.h", mark)
        self.c_machine = write(self.build / "ctests/gen/c_sm.h", mark)
        lib_src = write(self.root / "sce/src/x.cpp", '#include "core/A.h"\n#include <vector>\n')
        test_src = write(self.root / "tests/t.cpp", '#include "m_sm.h"\n#include "core/A.h"\n')
        c_src = write(self.root / "backends/c/tests/u.c", '#include "c_sm.h"\n')
        track(self.root)
        self.lib = tool.Entry(str(self.build / "sce"), str(lib_src),
                              ["c++", f"-I{inc}", "-c", str(lib_src)],
                              "sce_runtime", "sce", "c++")
        self.test = tool.Entry(str(self.build / "tests"), str(test_src),
                               ["c++", f"-I{self.build}/tests/gen", f"-I{inc}",
                                "-c", str(test_src)],
                               "w3c", "tests", "c++")
        self.c_unit = tool.Entry(str(self.build / "ctests"), str(c_src),
                                 ["cc", f"-I{self.build}/ctests/gen", "-c", str(c_src)],
                                 "ct", "backends/c/tests", "c")
        self.entries = [self.test, self.c_unit, self.lib]

    def tearDown(self):
        self._tmp.cleanup()

    def graph(self):
        return tool.IncludeGraph(follow=(self.root, self.build),
                                 leaves=(self.root / "third_party",))

    def test_a_closure_reaches_through_headers(self):
        closure = self.graph().closure(self.lib)
        self.assertIn(str(self.a), closure)
        self.assertIn(str(self.b), closure)
        self.assertNotIn(str(self.machine), closure)

    def test_every_case_lands_where_it_should(self):
        wanted = [str(p) for p in (self.a, self.b, self.unreached, self.lonely,
                                   self.machine, self.child, self.c_machine)]
        plan = tool.assign_headers(wanted, self.entries, self.graph())
        got = {path: (a.entry.target, a.how) for path, a in plan.assigned.items()}
        self.assertEqual(got, {
            # The library is preferred although the test unit includes A too.
            str(self.a): ("sce_runtime", "included"),
            str(self.b): ("sce_runtime", "included"),
            str(self.unreached): ("sce_runtime", "sibling"),
            str(self.machine): ("w3c", "included"),
            str(self.child): ("w3c", "sibling"),
        })
        self.assertEqual(plan.c_only, {str(self.c_machine)})
        self.assertEqual(plan.unowned, [str(self.lonely)])

    def test_the_jobs_document_measures_each_kind(self):
        generated = tool.generated_headers(self.root, self.build)
        self.assertEqual(generated, sorted(str(p) for p in (self.machine, self.child,
                                                            self.c_machine)))
        doc = tool.build_jobs(self.root, self.build, self.entries, {"sce_runtime": 2020},
                              generated, cxx_override="g++", c_override="gcc")
        by_kind = {}
        for job in doc["jobs"]:
            by_kind.setdefault(job["kind"], []).append(job)
        self.assertEqual([j["subject"] for j in by_kind["tu"]], ["sce/src/x.cpp"])
        self.assertEqual(by_kind["tu"][0]["floor"], 2020)
        self.assertEqual(sorted(j["subject"] for j in by_kind["hdr"]),
                         ["sce/include/core/A.h", "sce/include/core/B.h",
                          "sce/include/core/Unreached.h"])
        self.assertEqual(sorted(j["subject"] for j in by_kind["gen"]),
                         ["<build>/tests/gen/child_sm.h", "<build>/tests/gen/m_sm.h"])
        self.assertEqual([j["subject"] for j in by_kind["ctu"]], ["backends/c/tests/u.c"])
        self.assertEqual(by_kind["ctu"][0]["standard"], "own")
        self.assertNotIn("forge", by_kind)
        self.assertEqual([(i["kind"], i["subject"]) for i in doc["not_measured"]],
                         [("hdr", "sce/include/lonely/C.h")])
        self.assertEqual(doc["c_headers"], 1)

        a_job = next(j for j in by_kind["hdr"] if j["subject"] == "sce/include/core/A.h")
        self.assertEqual(a_job["argv"], [
            "g++", "-Werror", "-std=@STD@", f"-I{self.root}/sce/include", "-fsyntax-only",
            "-x", "c++", "-include", str(self.a), os.devnull])
        self.assertEqual(a_job["via"], {"how": "included", "tu": "sce/src/x.cpp",
                                        "target": "sce_runtime"})


class TheRepositorySideIsWhatGitTracks(unittest.TestCase):
    """A file only this checkout has does not enter the measurement.

    A CI checkout holds what is committed, so a local run that also read a
    scratch header (an experiment's output, a local example build) would
    measure a different set, and list it as not measured or fail on it. The
    build tree is the exception: what this configure generated is the
    population, committed or not.
    """

    MARK = "// SCE-GENERATED -- DO NOT EDIT\n// source-hash: 0\n"

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        base = Path(self._tmp.name).resolve()
        self.root = base / "root"
        self.build = base / "build"
        self.committed = write(self.root / "sce/src/mesh/generated/envelope.h", self.MARK)
        self.public = write(self.root / "sce/include/core/A.h", "#pragma once\n")
        self.golden = write(self.root / "tests/forge/expected/algorithm.h", "#pragma once\n")
        write(self.root / "sce/CMakeLists.txt",
              "target_compile_features(sce_runtime PRIVATE cxx_std_20)\n")
        track(self.root)
        # Written after the index was filled: on disk, never committed.
        write(self.root / "scratch/run/built/power_auto_cut.h", self.MARK)
        write(self.root / "examples/doom_wasm/build/generated/doom_sm.h", self.MARK)
        write(self.root / "sce/include/core/Scratch.h", "#pragma once\n")
        write(self.root / "tests/forge/expected/scratch.h", "#pragma once\n")
        write(self.root / "cmake/Scratch.cmake",
              "target_compile_features(sce_base PRIVATE cxx_std_20)\n")
        self.built = write(self.build / "tests/gen/m_sm.h", self.MARK)

    def tearDown(self):
        self._tmp.cleanup()

    def test_only_committed_generated_headers_join_the_build_trees_own(self):
        self.assertEqual(tool.generated_headers(self.root, self.build),
                         sorted([str(self.built), str(self.committed)]))

    def test_an_untracked_public_header_or_golden_is_not_in_the_population(self):
        self.assertEqual(tool.public_headers(self.root), [str(self.public)])
        self.assertEqual(tool.forge_goldens(self.root), [str(self.golden)])

    def test_an_untracked_cmake_file_declares_no_standard(self):
        self.assertEqual(tool.implementation_floors(tool.cmake_sources(self.root)),
                         {"sce_runtime": 2020})

    def test_a_tree_git_cannot_read_is_an_error_not_an_empty_population(self):
        with tempfile.TemporaryDirectory() as plain:
            with self.assertRaises(RuntimeError):
                tool.generated_headers(Path(plain).resolve(), self.build)


class AMissingDependencyIsNotAFailure(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()
        write(self.root / "sce/include/core/B.h", "#pragma once\n")
        self.index = tool.FileIndex((self.root,))
        self.included = {"kind": "hdr", "subject": "h", "target": "t",
                         "via": {"how": "included", "tu": "a.cpp", "target": "t"}}
        self.sibling = {"kind": "gen", "subject": "g", "target": "t",
                        "via": {"how": "sibling", "tu": "b.cpp", "target": "t"}}

    def tearDown(self):
        self._tmp.cleanup()

    @staticmethod
    def missing(name):
        return (f"In file included from <command-line>:\n"
                f"/r/a.h:3:10: fatal error: {name}: No such file or directory\n"
                f"compilation terminated.\n")

    def test_a_clean_compile_passes(self):
        self.assertEqual(tool.classify(self.included, "c++17", 0, "", self.index).verdict, "pass")

    def test_a_file_this_machine_lacks_is_not_measured_and_named(self):
        outcome = tool.classify(self.included, "c++17", 1, self.missing("zenoh.hxx"), self.index)
        self.assertEqual(outcome.verdict, "unmeasured")
        self.assertIn("zenoh.hxx", outcome.detail)

    def test_a_file_the_tree_holds_is_a_failure(self):
        outcome = tool.classify(self.included, "c++17", 1, self.missing("core/B.h"), self.index)
        self.assertEqual(outcome.verdict, "fail")
        self.assertIn("core/B.h", outcome.detail)

    def test_borrowed_flags_that_miss_a_file_are_the_borrowings_limit(self):
        outcome = tool.classify(self.sibling, "c++20", 1, self.missing("core/B.h"), self.index)
        self.assertEqual(outcome.verdict, "unmeasured")
        self.assertIn("b.cpp", outcome.detail)

    def test_any_other_error_is_a_failure_with_its_first_line(self):
        output = ("In file included from /x/common/error.h:12,\n"
                  "/x/a.h:5:3: error: 'uint64_t' does not name a type\n"
                  "/x/a.h:6:3: error: later\n")
        outcome = tool.classify(self.included, "c++17", 1, output, self.index)
        self.assertEqual(outcome.verdict, "fail")
        self.assertEqual(outcome.detail, "/x/a.h:5:3: error: 'uint64_t' does not name a type")


class RunsAndFloors(unittest.TestCase):
    def test_a_cxx20_implementation_is_excused_at_cxx17_and_nothing_else_is(self):
        tu = {"kind": "tu", "subject": "x.cpp", "target": "sce_runtime", "floor": 2020}
        hdr = {"kind": "hdr", "subject": "a.h", "target": "t"}
        ctu = {"kind": "ctu", "subject": "u.c", "target": "c", "standard": "own"}
        runs, excused = tool.plan_runs([tu, hdr, ctu], ["c++17", "c++20"])
        self.assertEqual([(job["subject"], std) for job, std in runs],
                         [("x.cpp", "c++20"), ("a.h", "c++17"), ("a.h", "c++20"), ("u.c", None)])
        self.assertEqual(dict(excused), {("tu", "c++17"): [tu]})

    def test_a_floor_override_is_checked(self):
        self.assertEqual(tool.parse_floors(["gen=5"])["gen"], 5)
        self.assertEqual(tool.parse_floors(["gen=5"])["hdr"], tool.DEFAULT_FLOORS["hdr"])
        for bad in (["bogus=1"], ["gen"], ["gen=x"]):
            with self.assertRaises(ValueError, msg=bad):
                tool.parse_floors(bad)

    def test_a_standard_that_measured_too_little_is_a_breach(self):
        job = {"kind": "gen", "subject": "g", "target": "t"}
        outcomes = [tool.Outcome(job, "c++17", "pass") for _ in range(3)]
        outcomes.append(tool.Outcome(job, "c++20", "unmeasured", "needs x"))
        table = tool.tally(outcomes)
        self.assertEqual(table[("gen", "c++20")].measured, 0)
        floors = {kind: 0 for kind in tool.KINDS}
        floors["gen"] = 3
        breaches = tool.floor_breaches(table, ["c++17", "c++20"], floors)
        self.assertEqual(len(breaches), 1)
        self.assertTrue(breaches[0].startswith("gen at c++20: measured 0"))


class TheRunCommandEndToEnd(unittest.TestCase):
    """`run` over jobs whose "compiler" is this interpreter.

    The stand-in passes at C++20 and fails at C++17 the way GCC reports an
    error, so the case reads the standard substitution, the table, the failure
    listing and the exit status of one real run.
    """

    COMPILER = ("import sys; std = sys.argv[1]; "
                "print('a.h:1:1: error: needs C++20') if std != '-std=c++20' else None; "
                "sys.exit(0 if std == '-std=c++20' else 1)")

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name).resolve()

    def tearDown(self):
        self._tmp.cleanup()

    def jobs_file(self, code):
        doc = {"format": 1, "root": str(self.dir), "build_dir": str(self.dir / "build"),
               "jobs": [{"kind": "hdr", "subject": "sce/include/a.h", "target": "t",
                         "via": {"how": "included", "tu": "sce/src/a.cpp", "target": "t"},
                         "cwd": str(self.dir),
                         "argv": [sys.executable, "-c", code, "-std=@STD@"]}],
               "not_measured": [{"kind": "hdr", "subject": "sce/include/b.h",
                                 "reason": "no unit includes it"}],
               "c_headers": 0}
        path = self.dir / "jobs.json"
        path.write_text(json.dumps(doc), encoding="utf-8")
        return str(path)

    def floors(self, hdr):
        return [arg for kind in tool.KINDS
                for arg in ("--floor", f"{kind}={hdr if kind == 'hdr' else 0}")]

    def test_a_failure_at_one_standard_fails_the_run_and_is_listed(self):
        status, out, _ = run_main(["run", "--jobs", self.jobs_file(self.COMPILER),
                                   "--parallel", "2", *self.floors(1)])
        self.assertEqual(status, 1)
        self.assertIn("FAILED (1):", out)
        self.assertIn("[hdr c++17] sce/include/a.h", out)
        self.assertIn("a.h:1:1: error: needs C++20", out)
        self.assertNotIn("[hdr c++20] sce/include/a.h", out)
        self.assertIn("[hdr] sce/include/b.h: no unit includes it", out)

    def test_a_clean_run_above_its_floors_passes(self):
        status, out, _ = run_main(["run", "--jobs", self.jobs_file("import sys; sys.exit(0)"),
                                   *self.floors(1)])
        self.assertEqual(status, 0)
        self.assertNotIn("FAILED", out)

    def test_a_clean_run_below_its_floor_fails(self):
        status, out, _ = run_main(["run", "--jobs", self.jobs_file("import sys; sys.exit(0)"),
                                   *self.floors(2)])
        self.assertEqual(status, 1)
        self.assertIn("FLOOR: hdr at c++17: measured 1", out)

    def test_an_unknown_standard_is_refused(self):
        status, _, err = run_main(["run", "--jobs", self.jobs_file("pass"), "--std", "c++18"])
        self.assertEqual(status, 2)
        self.assertIn("not a C++ standard", err)


# ── Shards ────────────────────────────────────────────────────────


GENERATION_GRAPH = "\n".join([
    "digraph ninja {",
    'rankdir="LR"',
    # The parent's generation writes its header and the child document beside it.
    '"0x1" [label="tests/gen/m_sm.h"]',
    '"0x2" [label="tests/gen/m_child.scxml"]',
    '"0xe1" [label="CUSTOM_COMMAND", shape=ellipse]',
    '"0xe1" -> "0x1"',
    '"0xe1" -> "0x2"',
    '"0x3" -> "0xe1" [arrowhead=none]',
    '"0x4" -> "0xe1" [arrowhead=none]',
    '"0x3" [label="/src/m.scxml"]',
    '"0x4" [label="tests/gen/staged.scxml"]',
    # A staging step that writes no planned header, which two families read.
    '"0x5" -> "0x4" [label=" CUSTOM_COMMAND"]',
    '"0x5" [label="/src/staged.scxml"]',
    # The child's generation runs on the document the parent's wrote.
    '"0x6" [label="tests/gen/m_child_sm.h"]',
    '"0x2" -> "0x6" [label=" CUSTOM_COMMAND"]',
    # An unrelated family that reads the same staged document.
    '"0x7" [label="tests/gen/n_sm.h"]',
    '"0xe3" [label="CUSTOM_COMMAND", shape=ellipse]',
    '"0xe3" -> "0x7"',
    '"0x4" -> "0xe3" [arrowhead=none]',
    '"0x8" -> "0xe3" [arrowhead=none]',
    '"0x8" [label="/src/n.scxml"]',
    '"0x9" [label="ctests/gen/c_sm.h"]',
    '"0xa" -> "0x9" [label=" CUSTOM_COMMAND"]',
    '"0xa" [label="/src/c.scxml"]',
    '"0xb" [label="tests/gen/orphan_sm.h"]',
    '"0xc" -> "0xb" [label=" CUSTOM_COMMAND"]',
    '"0xc" [label="/src/orphan.scxml"]',
    "}",
])

PLANNED = ["ctests/gen/c_sm.h", "tests/gen/m_child_sm.h", "tests/gen/m_sm.h",
           "tests/gen/n_sm.h", "tests/gen/orphan_sm.h"]

MARK = "// SCE-GENERATED -- DO NOT EDIT\n// source-hash: 0\n"


def generation_graph():
    return tool.ctest_shards_module().Graph(GENERATION_GRAPH)


class GenerationFamiliesFollowTheStepsThatWriteHeaders(unittest.TestCase):
    def test_a_child_joins_its_parent_and_a_shared_prerequisite_joins_nothing(self):
        family_of, steps = tool.generation_families(
            PLANNED + ["tests/gen/unknown_sm.h"], generation_graph(), Path("/b"))
        self.assertEqual(family_of["tests/gen/m_sm.h"], "tests/gen/m_child_sm.h")
        self.assertEqual(family_of["tests/gen/m_child_sm.h"], "tests/gen/m_child_sm.h")
        # n_sm.h reads the staged document m_sm.h's generation reads too; the
        # step that stages it writes no planned header, so it couples nothing.
        self.assertEqual(family_of["tests/gen/n_sm.h"], "tests/gen/n_sm.h")
        self.assertEqual(family_of["tests/gen/orphan_sm.h"], "tests/gen/orphan_sm.h")
        self.assertEqual(family_of["tests/gen/unknown_sm.h"], "tests/gen/unknown_sm.h")
        self.assertEqual(steps["tests/gen/m_child_sm.h"], frozenset({"s:0x6", "e:0xe1", "s:0x4"}))
        self.assertIn("s:0x4", steps["tests/gen/n_sm.h"],
                      "a shared prerequisite is a step of every family that needs it")
        self.assertEqual(steps["tests/gen/unknown_sm.h"], frozenset())


class AShardPlanGivesEverySubjectToOneShard(unittest.TestCase):
    """The plan, the shard's filter and its account, on a tree with one of each case."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        base = Path(self._tmp.name).resolve()
        self.root = base / "root"
        self.build = base / "build"
        inc = self.root / "sce/include"
        write(inc / "core/A.h", '#pragma once\n#include "core/B.h"\n')
        write(inc / "core/B.h", "#pragma once\n")
        write(inc / "core/Late.h", "#pragma once\n")
        write(self.root / "tests/forge/expected/alg.h", "#pragma once\n")
        write(self.root / "backends/cpp/forge-runtime/include/sce/forge/rt.h", "#pragma once\n")
        lib_src = write(self.root / "sce/src/x.cpp", '#include "core/A.h"\n')
        t1_src = write(self.root / "tests/t1.cpp", '#include "m_sm.h"\n#include "core/A.h"\n')
        t2_src = write(self.root / "tests/t2.cpp", '#include "n_sm.h"\n')
        c_src = write(self.root / "backends/c/tests/u.c", '#include "c_sm.h"\n')
        track(self.root)
        self.build.mkdir(parents=True)
        gen = f"-I{self.build}/tests/gen"
        self.entries = [
            tool.Entry(str(self.build / "tests"), str(t2_src),
                       ["c++", gen, "-c", str(t2_src)], "w3c2", "tests", "c++"),
            tool.Entry(str(self.build / "sce"), str(lib_src),
                       ["c++", f"-I{inc}", "-c", str(lib_src)], "sce_runtime", "sce", "c++"),
            tool.Entry(str(self.build / "tests"), str(t1_src),
                       ["c++", gen, f"-I{inc}", "-c", str(t1_src)], "w3c", "tests", "c++"),
            tool.Entry(str(self.build / "ctests"), str(c_src),
                       ["cc", f"-I{self.build}/ctests/gen", "-c", str(c_src)],
                       "ct", "backends/c/tests", "c"),
        ]

    def tearDown(self):
        self._tmp.cleanup()

    def plan(self, count):
        return tool.plan_shards(self.root, self.build, self.entries, PLANNED,
                                generation_graph(), count)

    @staticmethod
    def shard_of(plan, key):
        held = [i for i, part in enumerate(plan["shards"]) if key in part["subjects"]]
        return held[0] if len(held) == 1 else None

    def generate(self, outputs):
        """What the generation rules would write for `outputs`."""
        bodies = {"tests/gen/m_sm.h": MARK + '#include "m_child_sm.h"\n#include "core/Late.h"\n'}
        for rel in outputs:
            write(self.build / rel, bodies.get(rel, MARK))

    def ungenerate(self):
        for rel in PLANNED:
            (self.build / rel).unlink(missing_ok=True)

    def test_a_planned_header_is_reached_before_it_is_written(self):
        t1 = next(e for e in self.entries if e.target == "w3c")
        planned = {os.path.normpath(str(self.build / rel)) for rel in PLANNED}
        self.assertNotIn(str(self.build / "tests/gen/m_sm.h"),
                         tool.IncludeGraph((self.root, self.build)).closure(t1))
        self.assertIn(str(self.build / "tests/gen/m_sm.h"),
                      tool.PlannedIncludeGraph((self.root, self.build), (), planned).closure(t1))

    def test_every_subject_is_in_exactly_one_shard_and_late_ones_in_none(self):
        everything = None
        for count in (1, 2, 3, 5):
            plan = self.plan(count)
            given = [k for part in plan["shards"] for k in part["subjects"]]
            self.assertEqual(len(given), len(set(given)), f"{count}: a subject in two shards")
            self.assertEqual(len(given), plan["subjects"])
            self.assertEqual(len(plan["shards"]), count)
            if everything is None:
                everything = set(given)
            self.assertEqual(set(given), everything, f"{count}: the same subjects at any count")
            self.assertEqual(plan["late"], ["hdr:sce/include/core/Late.h"])
            self.assertEqual(plan, self.plan(count), f"{count}: the same tree plans the same way")
        self.assertEqual(everything, {
            "tu:sce/src/x.cpp@sce_runtime", "ctu:backends/c/tests/u.c@ct",
            "hdr:sce/include/core/A.h", "hdr:sce/include/core/B.h",
            "hdr:backends/cpp/forge-runtime/include/sce/forge/rt.h",
            "forge:tests/forge/expected/alg.h",
            *(f"gen:<build>/{rel}" for rel in PLANNED)})
        self.assertNotEqual(self.plan(2)["digest"], self.plan(3)["digest"])

    def test_what_belongs_together_is_planned_together_and_generated_there(self):
        plan = self.plan(5)
        lib = self.shard_of(plan, "tu:sce/src/x.cpp@sce_runtime")
        # The library owns the public headers it includes.
        self.assertEqual(self.shard_of(plan, "hdr:sce/include/core/A.h"), lib)
        self.assertEqual(self.shard_of(plan, "hdr:sce/include/core/B.h"), lib)
        # A family is one shard's, with the unit that reaches it and the header
        # a unit reaches only by its directory.
        parent = self.shard_of(plan, "gen:<build>/tests/gen/m_sm.h")
        self.assertEqual(self.shard_of(plan, "gen:<build>/tests/gen/m_child_sm.h"), parent)
        self.assertEqual(self.shard_of(plan, "gen:<build>/tests/gen/orphan_sm.h"), parent)
        self.assertEqual(self.shard_of(plan, "gen:<build>/ctests/gen/c_sm.h"),
                         self.shard_of(plan, "ctu:backends/c/tests/u.c@ct"))
        for part in plan["shards"]:
            for key in part["subjects"]:
                if key.startswith("gen:<build>/"):
                    self.assertIn(key[len("gen:<build>/"):], part["outputs"],
                                  f"{key} is measured where it is not generated")
        self.assertIn("ctests/gen/c_sm.h", plan["shards"][self.shard_of(
            plan, "ctu:backends/c/tests/u.c@ct")]["outputs"])

    def test_the_shards_measure_what_the_whole_run_measures_each_once(self):
        self.generate(PLANNED)
        generated = tool.generated_headers(self.root, self.build)
        whole = tool.build_jobs(self.root, self.build, self.entries, {}, generated,
                                cxx_override="g++", c_override="gcc")
        expected = {tool.job_key(j) for j in whole["jobs"]}
        self.ungenerate()

        plan = self.plan(3)
        measured = []
        late_shards = 0
        for index in (1, 2, 3):
            self.generate(plan["shards"][index - 1]["outputs"])
            generated = tool.generated_headers(self.root, self.build)
            doc = tool.build_jobs(self.root, self.build, self.entries, {}, generated,
                                  cxx_override="g++", c_override="gcc")
            mine = tool.filter_for_shard(doc, {**plan, "shard": index}, self.entries,
                                         self.root, self.build, generated)
            faults = {k: s for k, s in mine["accounting"].items()
                      if s in tool.ACCOUNTING_FAULTS}
            self.assertEqual(faults, {}, f"shard {index} could not account for its subjects")
            self.assertEqual(set(mine["accounting"]),
                             set(plan["shards"][index - 1]["subjects"]))
            measured += [j["key"] for j in mine["jobs"] if not j.get("late")]
            late = [j for j in mine["jobs"] if j.get("late")]
            if late:
                late_shards += 1
                self.assertEqual([j["key"] for j in late], ["hdr:sce/include/core/Late.h"])
            self.assertEqual(mine["shard"]["index"], index)
            self.assertEqual(set(mine["unbuilt"]) & set(plan["shards"][index - 1]["outputs"]),
                             set())
            self.ungenerate()
        self.assertEqual(len(measured), len(set(measured)), "a subject measured twice")
        self.assertEqual(set(measured) | {"hdr:sce/include/core/Late.h"}, expected)
        self.assertGreaterEqual(late_shards, 1, "no shard measured the late header")

    def test_the_c_header_is_accounted_as_compiled_through_its_c_unit(self):
        plan = self.plan(1)
        self.generate(PLANNED)
        generated = tool.generated_headers(self.root, self.build)
        doc = tool.build_jobs(self.root, self.build, self.entries, {}, generated,
                              cxx_override="g++", c_override="gcc")
        mine = tool.filter_for_shard(doc, {**plan, "shard": 1}, self.entries,
                                     self.root, self.build, generated)
        self.assertEqual(mine["accounting"]["gen:<build>/ctests/gen/c_sm.h"],
                         "compiled through C units")
        self.assertEqual(mine["accounting"]["gen:<build>/tests/gen/m_child_sm.h"], "measured")

    def test_a_planned_header_the_generation_did_not_write_is_a_fault(self):
        plan = self.plan(1)
        self.generate([rel for rel in PLANNED if rel != "tests/gen/n_sm.h"])
        generated = tool.generated_headers(self.root, self.build)
        doc = tool.build_jobs(self.root, self.build, self.entries, {}, generated,
                              cxx_override="g++", c_override="gcc")
        mine = tool.filter_for_shard(doc, {**plan, "shard": 1}, self.entries,
                                     self.root, self.build, generated)
        self.assertEqual(mine["accounting"]["gen:<build>/tests/gen/n_sm.h"], "absent")


class AShardRunsItsPart(unittest.TestCase):
    def test_a_header_generated_in_another_shard_is_a_failure_not_a_miss(self):
        job = {"kind": "gen", "subject": "g", "target": "t",
               "via": {"how": "included", "tu": "a.cpp", "target": "t"}}
        output = "/b/x.h:2:10: fatal error: m_child_sm.h: No such file or directory\n"
        with tempfile.TemporaryDirectory() as empty:
            index = tool.FileIndex((Path(empty),))
            outcome = tool.classify(job, "c++17", 1, output, index,
                                    ["tests/gen/m_child_sm.h"])
            self.assertEqual(outcome.verdict, "fail")
            self.assertIn("another shard", outcome.detail)
            self.assertEqual(tool.classify(job, "c++17", 1, output, index).verdict, "unmeasured")

    def test_a_shards_floors_are_its_share_of_the_whole(self):
        floors = dict(tool.DEFAULT_FLOORS)
        scaled = tool.scaled_floors(floors, {"gen": 100, "hdr": 0}, {"gen": 400, "hdr": 300})
        self.assertEqual(scaled["gen"], tool.DEFAULT_FLOORS["gen"] // 4)
        self.assertEqual(scaled["hdr"], 0)
        self.assertEqual(scaled["tu"], 0, "a kind the plan counted none of holds no floor")

    def test_a_late_failure_is_left_to_the_verdict(self):
        with tempfile.TemporaryDirectory() as scratch:
            work = Path(scratch)
            fail = [sys.executable, "-c", "import sys; print('x.h:1:1: error: no'); sys.exit(1)",
                    "-std=@STD@"]
            ok = [sys.executable, "-c", "import sys; sys.exit(0)", "-std=@STD@"]
            via = {"how": "included", "tu": "sce/src/a.cpp", "target": "t"}
            doc = {"format": 1, "root": str(work), "build_dir": str(work / "build"),
                   "jobs": [{"kind": "hdr", "subject": "sce/include/a.h", "target": "t",
                             "via": via, "cwd": str(work), "argv": ok,
                             "key": "hdr:sce/include/a.h"},
                            {"kind": "hdr", "subject": "sce/include/late.h", "target": "t",
                             "via": via, "cwd": str(work), "argv": fail,
                             "key": "hdr:sce/include/late.h",
                             "late": {"rank": 3, "how": "included"}}],
                   "not_measured": [], "c_headers": 0, "unbuilt": [],
                   "shard": {"index": 1, "count": 2, "digest": "0" * 64,
                             "counts": {"hdr": 1}, "totals": {"hdr": 1000}}}
            jobs = work / "jobs.json"
            jobs.write_text(json.dumps(doc), encoding="utf-8")
            results = work / "results.json"
            status, out, _ = run_main(["run", "--jobs", str(jobs), "--results", str(results)])
            self.assertEqual(status, 0, out)
            self.assertIn("LATE (2 compile(s))", out)
            recorded = json.loads(results.read_text(encoding="utf-8"))["outcomes"]
            self.assertEqual(sorted((r["key"], r["standard"], r["verdict"]) for r in recorded), [
                ("hdr:sce/include/a.h", "c++17", "pass"), ("hdr:sce/include/a.h", "c++20", "pass"),
                ("hdr:sce/include/late.h", "c++17", "fail"),
                ("hdr:sce/include/late.h", "c++20", "fail")])


class TheVerdictHoldsTheShardsTogether(unittest.TestCase):
    """`verify-shards` over the files two shards leave."""

    DIGEST = "d" * 64

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name).resolve()
        self.plan = {"format": 1, "count": 2, "digest": self.DIGEST, "stds": ["c++17", "c++20"],
                     "subjects": 2, "late": ["hdr:late.h"],
                     "totals": {kind: 0 for kind in tool.KINDS},
                     "shards": [{"subjects": ["hdr:a.h"], "outputs": [], "cost": 2, "counts": {}},
                                {"subjects": ["hdr:b.h"], "outputs": [], "cost": 2,
                                 "counts": {}}]}
        self.outcomes = {
            1: [self.record("hdr:a.h", "pass"),
                self.record("hdr:late.h", "fail", late={"rank": 4, "how": "included"})],
            2: [self.record("hdr:b.h", "pass"),
                self.record("hdr:late.h", "pass", late={"rank": 1, "how": "included"})],
        }

    def tearDown(self):
        self._tmp.cleanup()

    @staticmethod
    def record(key, verdict, late=None):
        return [{"key": key, "kind": "hdr", "subject": key[4:], "target": "t", "standard": std,
                 "verdict": verdict, "detail": "" if verdict == "pass" else "x.h: error: no",
                 "how": (late or {}).get("how", "included"), "tu": "a.cpp", "late": late}
                for std in ("c++17", "c++20")]

    def leave(self, index, digest=None, accounting=None, results=True):
        shard = self.dir / f"shard-{index}"
        shard.mkdir(parents=True, exist_ok=True)
        plan = {**self.plan, "shard": index, "digest": digest or self.DIGEST}
        (shard / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
        mine = self.plan["shards"][index - 1]["subjects"]
        account = accounting if accounting is not None else {k: "measured" for k in mine}
        (shard / "jobs.json").write_text(json.dumps({"accounting": account,
                                                     "not_measured": []}), encoding="utf-8")
        if results:
            flat = [r for group in self.outcomes[index] for r in group]
            (shard / "results.json").write_text(json.dumps({"outcomes": flat}), encoding="utf-8")

    def verify(self):
        return tool.verify_shards(self.dir, {kind: 0 for kind in tool.KINDS})

    def test_a_whole_run_passes_and_counts_the_late_header_where_the_whole_run_would(self):
        self.leave(1)
        self.leave(2)
        faults, verdict = self.verify()
        self.assertEqual(faults, [])
        # The late header failed in shard 1 under a later owner; shard 2's
        # owner comes first, and its pass is the one counted.
        self.assertEqual(verdict["failures"], [])
        self.assertEqual(verdict["table"][("hdr", "c++17")].passed, 3)

    def test_the_earliest_owner_decides_even_when_it_fails(self):
        self.outcomes[1] = [self.record("hdr:a.h", "pass"),
                            self.record("hdr:late.h", "fail", late={"rank": 0, "how": "included"})]
        self.leave(1)
        self.leave(2)
        _, verdict = self.verify()
        self.assertEqual(len(verdict["failures"]), 2)

    def test_an_including_owner_comes_before_a_sibling(self):
        self.outcomes[1] = [self.record("hdr:a.h", "pass"),
                            self.record("hdr:late.h", "fail", late={"rank": 0, "how": "sibling"})]
        self.leave(1)
        self.leave(2)
        _, verdict = self.verify()
        self.assertEqual(verdict["failures"], [])

    def test_shards_that_planned_differently_are_refused(self):
        self.leave(1)
        self.leave(2, digest="e" * 64)
        faults, _ = self.verify()
        self.assertTrue(any("planned differently" in f for f in faults), faults)

    def test_a_shard_that_did_not_finish_is_named(self):
        self.leave(1)
        self.leave(2, results=False)
        faults, _ = self.verify()
        self.assertTrue(any("shard 2 of 2 left no account or no results" in f for f in faults),
                        faults)

    def test_a_shard_that_left_nothing_is_named(self):
        self.leave(1)
        faults, _ = self.verify()
        self.assertTrue(any("shard 2 of 2 left no plan" in f for f in faults), faults)

    def test_a_subject_its_shard_did_not_account_for_is_a_fault(self):
        self.leave(1)
        self.leave(2, accounting={"hdr:b.h": "absent"})
        faults, _ = self.verify()
        self.assertTrue(any("hdr:b.h is absent" in f for f in faults), faults)

    def test_the_whole_floors_are_held_over_the_shards_together(self):
        self.leave(1)
        self.leave(2)
        floors = {kind: 0 for kind in tool.KINDS}
        floors["hdr"] = 4
        _, verdict = tool.verify_shards(self.dir, floors)
        self.assertEqual(len(verdict["breaches"]), 2)
        floors["hdr"] = 3
        _, verdict = tool.verify_shards(self.dir, floors)
        self.assertEqual(verdict["breaches"], [])

    def test_no_shard_at_all_is_not_a_pass(self):
        faults, verdict = self.verify()
        self.assertEqual(verdict, {})
        self.assertTrue(faults)


if __name__ == "__main__":
    unittest.main()
