"""The boundary, measured rather than asserted.

This package claims to be general. The claim is worth nothing as prose in a
README, because the way it stops being true is that somebody adds one helpful
special case for the subject matter in front of them. So it is a test.

Two guards, deliberately unlike each other:

  by vocabulary   a list of words no general tool has any business spelling.
                  Only as wide as somebody's imagination, which is why there
                  is a second one.

  by structure    the core opens no file it was not handed. A domain fact has
                  no route in except through a pack, whatever it is called.
"""

import ast
import pathlib
import re
import unittest

CORE = pathlib.Path(__file__).resolve().parent.parent / "sce_author"

# Subject matters, not words. Each entry is something a general tool could only
# know by having been specialised. Extend this whenever a pack is written for a
# new subject matter: the cost of a wrong entry is one renamed local variable.
SUBJECT_WORDS = {
    "vehicle", "ignition", "telltale", "cluster", "odometer", "speedometer",
    "tachometer", "brake", "airbag", "seatbelt", "engine", "battery", "chassis",
    "powertrain", "transmission", "wiper", "headlamp", "mileage", "coolant",
    "patient", "diagnosis", "prescription", "invoice", "ledger", "portfolio",
    "trading", "avionics", "autopilot", "thermostat", "elevator", "centrifuge",
}

# Opening a file is how a fact gets in. The core is allowed exactly the paths
# its caller named, plus its own schema directory.
ALLOWED_OPEN_SITES = {"pack.py", "prose.py", "check.py", "brief.py"}


def sources():
    return sorted(CORE.glob("*.py"))


class CoreIsDomainFree(unittest.TestCase):
    def test_no_source_names_a_subject_matter(self):
        offences = []
        for path in sources():
            for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                for word in re.findall(r"[A-Za-z]+", line.lower()):
                    if word in SUBJECT_WORDS:
                        offences.append(f"{path.name}:{lineno}: {word!r} in {line.strip()!r}")
        self.assertEqual(
            [], offences,
            "the core named a subject matter. It belongs in a pack:\n" + "\n".join(offences),
        )

    def test_the_core_names_no_location_of_its_own(self):
        """No absolute path, and no file opened at a constant.

        A hard-coded location is how a tool acquires a home, and a tool with a
        home is a tool for one installation. The check is on locations rather
        than on the word `/`, because the first form of it flagged the module
        docstring and a glob suffix, and a test that cries on prose gets its
        assertion widened until it stops saying anything.
        """
        offences = []
        for path in sources():
            tree = ast.parse(path.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Constant) and isinstance(node.value, str):
                    if node.value.startswith(("/", "~/", "../", "./")):
                        offences.append(f"{path.name}:{node.lineno}: absolute path {node.value!r}")
                if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                    if node.func.id in {"open"} and node.args:
                        arg = node.args[0]
                        if isinstance(arg, ast.Constant):
                            offences.append(f"{path.name}:{node.lineno}: open() at a constant")
        self.assertEqual([], offences, "\n".join(offences))

    def test_only_one_module_may_run_another_program(self):
        """⚠ Running a subprocess is a wider hole than opening a file.

        `verify` has to build the document, and building it means running the
        product's code generator -- so exactly one module here spawns, and the
        program it spawns is named by the caller or defaults to this tree's
        own. Any other module gaining that power is a new way for the core to
        depend on something outside the pack, and this case is here so it
        cannot happen quietly.
        """
        allowed = {"verify.py"}
        offences = []
        for path in sources():
            tree = ast.parse(path.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                spawns = (
                    isinstance(node, ast.Attribute)
                    and node.attr in {"run", "Popen", "call", "check_output",
                                      "check_call", "system", "execv", "spawnv"}
                    and isinstance(node.value, ast.Name)
                    and node.value.id in {"subprocess", "os"}
                )
                if spawns and path.name not in allowed:
                    offences.append(f"{path.name}:{node.lineno}: spawns a process")
        self.assertEqual([], offences, "\n".join(offences))

    def test_the_program_verify_runs_can_be_named_by_the_caller(self):
        """And a generator that is not there is a refusal, not a crash.

        The default is derived from this package's own location so it moves
        with the package -- `test_the_core_names_no_location_of_its_own` above
        already forbids spelling one out. What this adds is the other half: a
        caller on a tree laid out differently can say where the generator is,
        and naming one that does not exist says so.
        """
        import inspect

        from sce_author.verify import VerifyError, verify

        signature = inspect.signature(verify)
        self.assertIn("codegen", signature.parameters,
                      "the caller has to be able to name the generator")
        self.assertIsNone(signature.parameters["codegen"].default)

        crossing = pathlib.Path(__file__).resolve().parent / "fixtures" / "crossing"
        from sce_author.pack import load_pack
        with self.assertRaises(VerifyError) as caught:
            verify(load_pack(crossing),
                   crossing / "controller.resolved.binding.yaml",
                   codegen=crossing / "no-such-generator")
        self.assertIn("no-such-generator", str(caught.exception))

    def test_every_module_is_importable_without_a_pack(self):
        """Importing must not require a subject matter to exist anywhere."""
        import importlib

        for path in sources():
            if path.name == "__main__.py":
                continue
            importlib.import_module(f"sce_author.{path.stem}" if path.stem != "__init__" else "sce_author")


if __name__ == "__main__":
    unittest.main()
