"""The server says, before a client starts it, whether it can do its work.

An application that starts an AI client with this server (`SCE_AUTHOR_MCP`) cannot see what happens
inside: a server that fails to start because Python has no PyYAML, or because the product's
generator is not where it is looked for, is an AI that "does not answer" and an owner who is told
nothing. `--check` is the question asked first, so that the application can say what to install or
set where the owner looks. It is answered by the server itself, once: the launchers (the bundle's and
the checkout's) pass `--check` through to it, so the authority on what this server needs is not
copied into whatever starts it.
"""

from __future__ import annotations

import contextlib
import io
import os
import pathlib
import tempfile
import unittest
import unittest.mock

from sce_author import mcp


def check(env: dict[str, str]) -> tuple[int, str, str]:
    """Run `--check` as the launcher would, with `env` over the environment."""
    out, err = io.StringIO(), io.StringIO()
    with unittest.mock.patch.dict(os.environ, env), \
            contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        status = mcp.main(["--check"])
    return status, out.getvalue(), err.getvalue()


class TheServerSaysWhetherItCanStart(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = pathlib.Path(self._tmp.name)

    def program(self, name: str) -> str:
        path = self.dir / name
        path.write_text("#!/bin/sh\n", encoding="utf-8")
        path.chmod(0o755)
        return str(path)

    def test_a_server_with_what_it_needs_says_it_is_ready_and_exits_cleanly(self):
        status, out, err = check({"SCE_CODEGEN": self.program("sce-codegen"),
                                  "SCE_WORK": self.program("sce-work")})

        self.assertEqual(0, status, err)
        self.assertIn("ready", out)
        self.assertEqual("", err)

    def test_a_generator_that_is_not_there_is_named_with_the_way_to_set_it(self):
        status, _, err = check({"SCE_CODEGEN": str(self.dir / "nowhere" / "sce-codegen"),
                                "SCE_WORK": self.program("sce-work")})

        self.assertEqual(1, status)
        self.assertIn("generator", err)
        self.assertIn("SCE_CODEGEN", err)
        self.assertIn(str(self.dir / "nowhere" / "sce-codegen"), err)

    def test_a_works_command_that_is_not_there_is_named_with_the_way_to_set_it(self):
        status, _, err = check({"SCE_CODEGEN": self.program("sce-codegen"),
                                "SCE_WORK": str(self.dir / "nowhere" / "sce-work")})

        self.assertEqual(1, status)
        self.assertIn("sce-work", err)
        self.assertIn("SCE_WORK", err)

    def test_every_problem_is_said_not_only_the_first(self):
        status, _, err = check({"SCE_CODEGEN": str(self.dir / "a"), "SCE_WORK": str(self.dir / "b")})

        self.assertEqual(1, status)
        self.assertEqual(2, len([line for line in err.splitlines() if line.strip()]))

    def test_a_missing_yaml_module_is_said_with_what_to_install(self):
        env = {"SCE_CODEGEN": self.program("sce-codegen"), "SCE_WORK": self.program("sce-work")}
        real_import = __import__

        def refusing(name, *args, **kwargs):
            if name == "yaml":
                raise ImportError("No module named 'yaml'")
            return real_import(name, *args, **kwargs)

        with unittest.mock.patch("builtins.__import__", refusing):
            status, _, err = check(env)

        self.assertEqual(1, status)
        self.assertIn("PyYAML", err)
        self.assertIn("pip install pyyaml", err)

    def test_the_check_is_not_a_way_to_start_a_server(self):
        with self.assertRaises(SystemExit) as refused, contextlib.redirect_stderr(io.StringIO()):
            mcp.main(["--check", "--http", "127.0.0.1:8765"])

        self.assertNotEqual(0, refused.exception.code)


if __name__ == "__main__":
    unittest.main()
