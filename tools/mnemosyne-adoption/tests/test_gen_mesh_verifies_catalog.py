"""Regression tests for the mesh verifies-catalog generator.

The catalog is a gate input, so the properties that matter are the ones that
decide whether the gate can be fooled: a marker naming a section that does not
exist must fail rather than emit an entry no binding can match, an unmarked
file must not appear, and the rendering must be byte-stable so `--check` does
not report drift that is not there.
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.abspath(os.path.join(HERE, "..", "gen_mesh_verifies_catalog.py"))


class FakeRepo:
    """A throwaway tree with the store layout the generator reads."""

    def __init__(self, sections):
        self.dir = tempfile.mkdtemp(prefix="mesh_verifies_cat_")
        store_dir = os.path.join(self.dir, "docs", "sce-ledger", "mesh", ".atomic")
        os.makedirs(store_dir)
        with open(os.path.join(store_dir, "workspace.atomic.json"), "w") as fh:
            json.dump({"sections": {s: {"title": s} for s in sections}}, fh)
        os.makedirs(os.path.join(self.dir, "tests", "mesh"))
        self.out = os.path.join(store_dir, "verifies-catalog.json")

    def write_test(self, name, body):
        path = os.path.join(self.dir, "tests", "mesh", name)
        with open(path, "w") as fh:
            fh.write(body)
        return path

    def run(self, *extra):
        return subprocess.run(
            [sys.executable, SCRIPT, "--repo-root", self.dir, "--out", self.out, *extra],
            capture_output=True, text=True)

    def catalog(self):
        with open(self.out) as fh:
            return json.load(fh)


HEADER = "// SPDX-License-Identifier: X\n//\n// SCE-VERIFIES: {ids}\n//\n\nint main() {{}}\n"


class GenMeshVerifiesCatalogTest(unittest.TestCase):
    def test_marked_file_becomes_an_entry(self):
        repo = FakeRepo(["mesh-10.5", "mesh-10.6"])
        repo.write_test("AlphaTest.cpp", HEADER.format(ids="mesh-10.5 mesh-10.6"))
        proc = repo.run()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(repo.catalog()["entries"], [
            {"file": "tests/mesh/AlphaTest.cpp",
             "section_ids": ["mesh-10.5", "mesh-10.6"]},
        ])

    def test_unmarked_file_is_absent(self):
        repo = FakeRepo(["mesh-10.5"])
        repo.write_test("AlphaTest.cpp", "// no marker here\nint main() {}\n")
        proc = repo.run()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(repo.catalog()["entries"], [])

    def test_unknown_section_is_rejected(self):
        # The hallucination guard. Without it the generator would emit an
        # entry that no binding can ever match, and the linkage gate would
        # stay green while the catalog described a section that does not exist.
        repo = FakeRepo(["mesh-10.5"])
        repo.write_test("AlphaTest.cpp", HEADER.format(ids="mesh-99.9"))
        proc = repo.run()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("not a section in the mesh ledger", proc.stderr)

    def test_malformed_section_id_is_rejected(self):
        repo = FakeRepo(["mesh-10.5"])
        repo.write_test("AlphaTest.cpp", HEADER.format(ids="scxml-3.10"))
        proc = repo.run()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("is not a mesh-<n> section id", proc.stderr)

    def test_duplicate_declaration_is_rejected(self):
        repo = FakeRepo(["mesh-10.5"])
        repo.write_test("AlphaTest.cpp", HEADER.format(ids="mesh-10.5 mesh-10.5"))
        proc = repo.run()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("declared twice", proc.stderr)

    def test_entries_and_sections_are_sorted(self):
        # Determinism: the file is committed and pinned by sha256, so two runs
        # over the same tree must render identically regardless of readdir
        # order or the order sections appear in a marker.
        repo = FakeRepo(["mesh-10.5", "mesh-10.6", "mesh-9.5"])
        repo.write_test("ZuluTest.cpp", HEADER.format(ids="mesh-9.5"))
        repo.write_test("AlphaTest.cpp", HEADER.format(ids="mesh-10.6 mesh-10.5"))
        self.assertEqual(repo.run().returncode, 0)
        entries = repo.catalog()["entries"]
        self.assertEqual([e["file"] for e in entries],
                         ["tests/mesh/AlphaTest.cpp", "tests/mesh/ZuluTest.cpp"])
        self.assertEqual(entries[0]["section_ids"], ["mesh-10.5", "mesh-10.6"])

    def test_check_detects_a_stale_catalog(self):
        repo = FakeRepo(["mesh-10.5", "mesh-10.6"])
        path = repo.write_test("AlphaTest.cpp", HEADER.format(ids="mesh-10.5"))
        self.assertEqual(repo.run().returncode, 0)
        self.assertEqual(repo.run("--check").returncode, 0)
        with open(path, "w") as fh:
            fh.write(HEADER.format(ids="mesh-10.6"))
        proc = repo.run("--check")
        self.assertEqual(proc.returncode, 1)
        self.assertIn("catalog STALE", proc.stderr)


if __name__ == "__main__":
    unittest.main()
