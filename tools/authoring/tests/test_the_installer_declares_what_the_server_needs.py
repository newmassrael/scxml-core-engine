"""The installer declares the Python packages the server needs, because the server says which.

The workbench's `.deb` carries the authoring server and not the Python it runs in: it declares
packages for the package manager to install. A declaration written beside the server and not read
from it goes out of date the day the server needs one more module, and the `.deb` then starts, passes
every check that does not use that module, and fails for the owner at the first thing that does:
`jsonschema` was missing from it for exactly that reason, and the first work with an owner's answer
in it was refused with an AI that "failed".

`sce_author.needs` is the list. These hold the two places that cannot read it at run time to it:
the `.deb`'s `Depends`, and the packages the CI machine is given to start the installed application
with (a machine that already has a module would otherwise pass an installer that does not ask for it).
`scripts/verify_installed_app.sh` holds the `.deb` that was built to the same list.
"""

from __future__ import annotations

import json
import pathlib
import unittest

from sce_author import needs

REPO = pathlib.Path(__file__).resolve().parents[3]
TAURI_CONFIG = REPO / "app" / "src-tauri" / "tauri.conf.json"
INSTALLER_WORKFLOW = REPO / ".github" / "workflows" / "installer.yml"


class TheInstallerDeclaresWhatTheServerNeeds(unittest.TestCase):
    def test_the_deb_depends_on_python_and_on_a_package_for_every_module_the_server_needs(self):
        depends = json.loads(TAURI_CONFIG.read_text(encoding="utf-8"))["bundle"]["linux"]["deb"]["depends"]

        self.assertIn("python3", depends)
        for need in needs.NEEDS:
            self.assertIn(need.debian, depends,
                          f"the server needs {need.name} ({need.module}), and the .deb does not "
                          f"declare {need.debian}")

    def test_the_machine_that_tries_the_installer_is_given_the_same_packages(self):
        workflow = INSTALLER_WORKFLOW.read_text(encoding="utf-8")

        for need in needs.NEEDS:
            self.assertTrue(need.debian in workflow,
                            f"{INSTALLER_WORKFLOW.name} does not install {need.debian}, so the "
                            f"application it starts could not find {need.name}")


if __name__ == "__main__":
    unittest.main()
