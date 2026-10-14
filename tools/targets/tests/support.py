"""What the targets' tests share: the made-up pack, a working copy of it, and the chain.

The fixture is synthetic on purpose. A test here never carries a platform's names, so the
tests can be read by anyone and the tools cannot have learned a platform from them.
"""

from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

import yaml

HERE = pathlib.Path(__file__).resolve().parent
TARGETS = HERE.parent
FIXTURE = HERE / "fixtures" / "pump"
SCHEMA = TARGETS / "schema" / "logical-interface.v1.schema.json"
STANDALONE_TARGET = TARGETS / "targets" / "standalone.yaml"


def codegen_is_built() -> bool:
    from sce_author.verify import _default_codegen
    return _default_codegen().exists()


def compiler() -> str | None:
    return shutil.which("g++") or shutil.which("c++")


class Working(unittest.TestCase):
    """A scratch copy of the pack, because the tools write beside what they read."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self._tmp.name)
        self.pack = self.root / "pack"
        shutil.copytree(FIXTURE, self.pack)
        self.binding = self.pack / "pump.binding.yaml"
        self.spec = self.pack / "specification.md"

    def tearDown(self):
        self._tmp.cleanup()

    def run_module(self, module: str, *args) -> subprocess.CompletedProcess:
        """A tool as a person runs it, with the authoring core on the module path."""
        env = dict(os.environ)
        env["PYTHONPATH"] = os.pathsep.join(
            [str(TARGETS), str(TARGETS.parent / "authoring"), env.get("PYTHONPATH", "")])
        return subprocess.run([sys.executable, "-m", module, *map(str, args)],
                              capture_output=True, text=True, env=env)

    def split(self, into: pathlib.Path | None = None) -> pathlib.Path:
        out = into or self.root / "split"
        done = self.run_module("sce_targets.split", "split", self.binding, self.pack, self.spec, out)
        self.assertEqual(0, done.returncode, done.stderr)
        return out

    def built_and_replayed(self, split_dir: pathlib.Path) -> dict:
        """Build the standalone library from a split, replay the pack's examples on it, and
        say what came of it: the replay's exit code (`rc`), what each case did (`cases`, by
        name: passed or not) and the whole log."""
        from sce_author.verify import _default_codegen
        interface = split_dir / "logical" / "pump.interface.yaml"
        script = self.root / "replay.txt"
        done = self.run_module("sce_targets.replay", interface, split_dir / "profile" / "pump.yaml",
                               self.pack, script)
        self.assertEqual(0, done.returncode, done.stderr)
        lib = self.root / "lib"
        done = self.run_module("sce_targets.standalone", interface, STANDALONE_TARGET,
                               _default_codegen(), lib)
        self.assertEqual(0, done.returncode, done.stdout + done.stderr)
        binary = self.root / "replay"
        built = subprocess.run([compiler(), "-std=c++17", "-Wall", "-Wextra", "-Werror",
                                "-I", str(lib), "-I", str(lib / "machine"),
                                str(lib / "pumpReplay.cpp"), "-o", str(binary)],
                               capture_output=True, text=True)
        self.assertEqual(0, built.returncode, built.stderr)
        ran = subprocess.run([str(binary), str(script)], capture_output=True, text=True)
        verdicts = {}
        for line in ran.stdout.splitlines():
            if line.startswith("PASS "):
                verdicts[line[5:]] = True
            elif line.startswith("FAIL "):
                verdicts[line[5:].split(" --")[0]] = False
        return {"rc": ran.returncode, "cases": verdicts, "log": ran.stdout}

    def interface(self, split_dir: pathlib.Path) -> dict:
        return yaml.safe_load((split_dir / "logical" / "pump.interface.yaml").read_text(encoding="utf-8"))

    def schema(self) -> dict:
        return json.loads(SCHEMA.read_text(encoding="utf-8"))
