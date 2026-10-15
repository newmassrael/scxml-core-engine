"""What the targets' tests share: the made-up pack, a working copy of it, and the chain.

The fixture is synthetic on purpose. A test here never carries a platform's names, so the
tests can be read by anyone and the tools cannot have learned a platform from them.
"""

from __future__ import annotations

import atexit
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
STATECHART_FIXTURE = HERE / "fixtures" / "lamp"
BEACON_FIXTURE = HERE / "fixtures" / "beacon"
LATCH_FIXTURE = HERE / "fixtures" / "latch"
GAUGE_FIXTURE = HERE / "fixtures" / "gauge"
CHIME_FIXTURE = HERE / "fixtures" / "chime"
SCHEMA = TARGETS / "schema" / "logical-interface.v1.schema.json"
STANDALONE_TARGET = TARGETS / "targets" / "standalone.yaml"


def codegen_is_built() -> bool:
    from sce_author.verify import _default_codegen
    return _default_codegen().exists()


def compiler() -> str | None:
    return shutil.which("g++") or shutil.which("c++")


REPO = TARGETS.parent.parent
# What a statechart library links beyond the machine the generator writes: three runtime sources
# the generator copies from one flat directory, and the headers they and the machine include.
RUNTIME_SOURCES = (REPO / "sce" / "src" / "common" / "UniqueIdGenerator.cpp",
                   REPO / "sce" / "src" / "common" / "EventDataHelper.cpp",
                   REPO / "sce" / "src" / "runtime" / "JsonUtils.cpp")
# The last is where a machine told a typed event payload finds the algorithm header it includes.
RUNTIME_INCLUDES = (REPO / "sce" / "include", REPO / "third_party" / "nlohmann_json" / "include",
                    REPO / "backends" / "cpp" / "forge-runtime" / "include")


def runtime_is_present() -> bool:
    return all(p.is_file() for p in RUNTIME_SOURCES) and all(p.is_dir() for p in RUNTIME_INCLUDES)


_runtime_objects: dict = {}


def runtime_to_link(lib: pathlib.Path, includes: list) -> list:
    """What a statechart library links of the runtime: the sources the generator copied beside it.

    They are the same three files for every library, and compiling them is most of what building
    one costs, so a copy that is byte for byte the repository's is compiled once for the whole run
    and linked as an object. A copy that differs (the generator changed it) is compiled in place."""
    linked = []
    for copy in sorted(lib.glob("Sce*.cpp")):
        original = next(p for p in RUNTIME_SOURCES if p.name == copy.name[3:])
        if copy.read_bytes() != original.read_bytes():
            linked.append(str(copy))
            continue
        if copy.name not in _runtime_objects:
            if "directory" not in _runtime_objects:
                _runtime_objects["directory"] = pathlib.Path(tempfile.mkdtemp(prefix="sce-targets-runtime-"))
                atexit.register(shutil.rmtree, _runtime_objects["directory"], ignore_errors=True)
            obj = _runtime_objects["directory"] / (copy.stem + ".o")
            built = subprocess.run([compiler(), "-std=c++17", "-Wall", "-Wextra", "-Werror", "-c",
                                    *includes, str(copy), "-o", str(obj)],
                                   capture_output=True, text=True)
            if built.returncode != 0:
                raise AssertionError(f"the runtime source {copy.name} does not build:\n{built.stderr}")
            _runtime_objects[copy.name] = obj
        linked.append(str(_runtime_objects[copy.name]))
    return linked


class Working(unittest.TestCase):
    """A scratch copy of the pack, because the tools write beside what they read. The pack is the
    pump (a transform) unless a test names another."""

    fixture = FIXTURE
    component = "pump"

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self._tmp.name)
        self.pack = self.root / "pack"
        shutil.copytree(self.fixture, self.pack)
        self.binding = self.pack / f"{self.component}.binding.yaml"
        self.spec = self.pack / "specification.md"

    def tearDown(self):
        self._tmp.cleanup()

    def run_module(self, module: str, *args, env_extra: dict | None = None) -> subprocess.CompletedProcess:
        """A tool as a person runs it, with the authoring core on the module path."""
        env = {**os.environ, **(env_extra or {})}
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
        interface = split_dir / "logical" / f"{self.component}.interface.yaml"
        script = self.root / "replay.txt"
        done = self.run_module("sce_targets.replay", interface, split_dir / "profile" / f"{self.component}.yaml",
                               self.pack, script)
        self.assertEqual(0, done.returncode, done.stderr)
        lib = self.root / "lib"
        statechart = self.interface(split_dir)["kind"] == "statechart"
        env_extra = {}
        if statechart:
            staged = self.root / "runtime"
            staged.mkdir()
            for source in RUNTIME_SOURCES:
                shutil.copy2(source, staged / source.name)
            env_extra["SCE_RUNTIME_SOURCE_DIR"] = str(staged)
        done = self.run_module("sce_targets.standalone", interface, STANDALONE_TARGET,
                               _default_codegen(), lib, env_extra=env_extra)
        self.assertEqual(0, done.returncode, done.stdout + done.stderr)
        binary = self.root / "replay"
        includes = [x for p in RUNTIME_INCLUDES for x in ("-I", str(p))] if statechart else []
        runtime = runtime_to_link(lib, includes) if statechart else []
        built = subprocess.run([compiler(), "-std=c++17", "-Wall", "-Wextra", "-Werror",
                                "-I", str(lib), "-I", str(lib / "machine"), *includes,
                                str(lib / f"{self.component}Replay.cpp"), *runtime, "-o", str(binary)],
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
        return yaml.safe_load((split_dir / "logical" / f"{self.component}.interface.yaml")
                              .read_text(encoding="utf-8"))

    def schema(self) -> dict:
        return json.loads(SCHEMA.read_text(encoding="utf-8"))
