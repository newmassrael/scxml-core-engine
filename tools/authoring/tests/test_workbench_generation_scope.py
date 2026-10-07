"""A model-writing client cannot widen its access through authoring tool arguments."""

import json
import os
import pathlib
import re
import sys
import tempfile
import unittest
from unittest.mock import patch

from sce_author import mcp, process

# The cases the application's reader of the answer an AI ends with is held to as well
# (app-core/src/document_files.rs): one rule in two languages.
FILE_REFERENCES = pathlib.Path(__file__).resolve().parent / "fixtures" / "file_references.json"

NS ='xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"'
AN_IMPORT_OF_A_PRIVATE_FILE = (
    f'<scxml {NS}><sce:import as="Level" src="/private/secret.scxml" kind="enum"/></scxml>')


class WorkbenchScope(unittest.TestCase):
    def setUp(self):
        self.env = patch.dict(os.environ, {"SCE_AUTHOR_WORK": "assigned-work"})
        self.env.start()
        self.addCleanup(self.env.stop)

    def test_only_generation_tools_are_advertised(self):
        answer = mcp.handle({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
        self.assertEqual(mcp.WORKBENCH_TOOLS,
                         {t["name"] for t in answer["result"]["tools"]})

    def test_an_unadvertised_mutation_is_refused_before_dispatch(self):
        with patch.object(mcp.works, "read_work") as read:
            answer = mcp.call_tool("works_save_model", {"work": "assigned-work"})
            self.assertTrue(answer["isError"])
            read.assert_not_called()

    def test_another_work_is_refused_before_it_is_read(self):
        with patch.object(mcp.works, "read_work") as read:
            answer = mcp.call_tool("works_read", {"work": "other-work"})
            self.assertTrue(answer["isError"])
            read.assert_not_called()

    def test_paths_are_refused_even_for_an_allowed_checker(self):
        for name, args in [
            ("validate_scxml", {"document": "/private/canary.scxml"}),
            ("validate_scxml_set", {"documents": ["/private/canary.scxml"]}),
            ("scxml_requirement_set", {"specification": "/private/canary.txt"}),
        ]:
            with self.subTest(name=name):
                answer = mcp.call_tool(name, args)
                self.assertTrue(answer["isError"])
                self.assertIn("_text", answer["content"][0]["text"])

    def test_import_companions_are_staged_beside_the_entry_for_requirements(self):
        def check(document, manifest, *, cwd):
            self.assertEqual("<scxml/>", (cwd / document).read_text())
            self.assertEqual("<schema/>", (cwd / "event.scxml").read_text())
            self.assertEqual("{}", (cwd / manifest).read_text())
            return "ok", ""

        with patch.object(mcp, "requirement_records", side_effect=check):
            answer = mcp.call_tool("scxml_requirements", {
                "document_text": "<scxml/>", "document_name": "main.scxml",
                "companions_text": [{"name": "event.scxml", "text": "<schema/>"}],
                "manifest_text": "{}",
            })
            self.assertFalse(answer.get("isError", False), answer)

    def test_a_companion_cannot_escape_the_staging_folder(self):
        answer = mcp.call_tool("scxml_requirements", {
            "document_text": "<scxml/>",
            "companions_text": [{"name": "../private.scxml", "text": "<scxml/>"}],
        })
        self.assertTrue(answer["isError"])

    def test_a_path_hidden_in_inline_scxml_cannot_restore_file_access(self):
        for reference in ("../private.scxml", "/private/secret.scxml", "file:///private.scxml",
                          "https://example.invalid/secret.scxml"):
            with self.subTest(reference=reference):
                answer = mcp.call_tool("scxml_requirements", {
                    "document_text": '<scxml xmlns:sce="http://sce.dev/ext"><sce:import src="'
                                     + reference + '"/></scxml>',
                })
                self.assertTrue(answer["isError"])
                self.assertIn("inline companions", answer["content"][0]["text"])

    def test_an_inline_document_cannot_load_an_external_dtd(self):
        answer = mcp.call_tool("scxml_requirements", {
            "document_text": '<!DOCTYPE scxml SYSTEM "file:///private/data.dtd"><scxml/>',
        })
        self.assertTrue(answer["isError"])
        self.assertIn("document type", answer["content"][0]["text"])

    def test_the_assigned_work_is_read_normally(self):
        with patch.object(mcp, "_work_reading", return_value={"source": None}) as read:
            answer = mcp.call_tool("works_read", {"work": "assigned-work"})
            self.assertFalse(answer.get("isError", False))
            self.assertEqual("assigned-work", read.call_args.args[0])

    def test_what_a_document_is_called_does_not_decide_whether_it_is_checked(self):
        # The caller names the file, and the product reads it as SCXML whatever it is called:
        # a check that looked at the name was one the caller could step round.
        for name in ("main.xml", "main.txt", "main", "MAIN.SCXML", "main.scxml.txt"):
            for tool in ("scxml_requirements", "validate_scxml"):
                with self.subTest(name=name, tool=tool):
                    answer = mcp.call_tool(tool, {
                        "document_text": AN_IMPORT_OF_A_PRIVATE_FILE, "document_name": name})
                    self.assertTrue(answer["isError"])
                    self.assertIn("inline companions", answer["content"][0]["text"])

    def test_a_companion_is_checked_whatever_it_is_called(self):
        entry = f'<scxml {NS}><sce:import as="Level" src="evil.xml" kind="enum"/></scxml>'
        for companion in ("evil.scxml", "evil.xml", "evil"):
            with self.subTest(companion=companion):
                answer = mcp.call_tool("scxml_requirements", {
                    "document_text": entry.replace("evil.xml", companion),
                    "document_name": "main.scxml",
                    "companions_text": [{"name": companion,
                                         "text": AN_IMPORT_OF_A_PRIVATE_FILE}],
                })
                self.assertTrue(answer["isError"])
                self.assertIn("inline companions", answer["content"][0]["text"])

    def test_the_documents_of_a_set_are_checked_whatever_they_are_called(self):
        answer = mcp.call_tool("validate_scxml_set", {
            "documents_text": [{"name": "main.txt", "text": AN_IMPORT_OF_A_PRIVATE_FILE}],
        })
        self.assertTrue(answer["isError"])
        self.assertIn("inline companions", answer["content"][0]["text"])

    def test_an_attribute_that_opens_a_file_is_refused_on_any_element(self):
        # Taken by the attribute and not by a list of elements: `sce:driver href` and an element
        # that does not exist yet are as much a way to a file as `sce:import src`.
        for element in ('<sce:driver href="/private/header.h"/>',
                        '<sce:something-new src="/private/x"/>',
                        '<state id="a"><invoke src="/private/x"/></state>',
                        '<xi:include xmlns:xi="http://www.w3.org/2001/XInclude" '
                        'href="/private/x.xml"/>'):
            with self.subTest(element=element):
                answer = mcp.call_tool("validate_scxml", {
                    "document_text": f'<scxml {NS} initial="a">{element}</scxml>'})
                self.assertTrue(answer["isError"])
                self.assertIn("inline companions", answer["content"][0]["text"])

    def test_the_base_references_resolve_against_cannot_be_changed(self):
        # A plain name is staged beside the document; against another base it is not.
        answer = mcp.call_tool("validate_scxml", {
            "document_text": f'<scxml {NS} xml:base="/private/">'
                             f'<sce:import as="Level" src="x.scxml" kind="enum"/></scxml>'})
        self.assertTrue(answer["isError"])
        self.assertIn("base", answer["content"][0]["text"])

    def test_a_document_this_check_cannot_read_is_not_passed_on_to_the_product(self):
        # Only a document both readers read the same way is one the check has looked at.
        with patch.object(mcp, "requirement_records") as product:
            answer = mcp.call_tool("scxml_requirements", {
                "document_text": f'<scxml {NS}><sce:import src="x.scxml"></scxml>'})
            self.assertTrue(answer["isError"])
            self.assertIn("not well-formed", answer["content"][0]["text"])
            product.assert_not_called()

    def test_a_reference_to_a_companion_staged_beside_it_is_accepted(self):
        # The control for the refusals above: what is refused is a path, and not an import.
        def check(document, manifest, *, cwd):
            self.assertTrue((cwd / "event.scxml").is_file())
            return "ok", ""

        with patch.object(mcp, "requirement_records", side_effect=check):
            answer = mcp.call_tool("scxml_requirements", {
                "document_text": f'<scxml {NS}><sce:import as="Event" src="event.scxml" '
                                 f'kind="enum"/></scxml>',
                "document_name": "main.scxml",
                "companions_text": [{"name": "event.scxml", "text": "<scxml/>"}],
                "manifest_text": "{}",
            })
            self.assertFalse(answer.get("isError", False), answer)

    def test_the_cases_both_readers_are_held_to_are_refused_and_accepted_as_they_say(self):
        cases = json.loads(FILE_REFERENCES.read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(cases), 20, "the shared cases were lost")
        for case in cases:
            with self.subTest(case=case["name"]):
                if case["refused"]:
                    with self.assertRaises(mcp.ToolArgumentError):
                        mcp._refuse_file_access_in(case["text"])
                else:
                    mcp._refuse_file_access_in(case["text"])
        # Both sides of the line are in them: a reader that refuses everything, or nothing, fails.
        self.assertTrue(any(case["refused"] for case in cases))
        self.assertTrue(any(not case["refused"] for case in cases))

    def test_the_product_is_run_in_the_one_folder_it_may_open_files_in(self):
        # A document that names a file elsewhere is a way to ask the machine about its files, so
        # the product is told that the folder it runs in is the only one.
        report = [sys.executable, "-c", "import os; print(os.environ.get('SCE_FILE_ROOT'))"]
        with tempfile.TemporaryDirectory() as folder:
            said = process.run(report, cwd=folder).stdout.strip()
            self.assertEqual(os.path.realpath(said), os.path.realpath(folder))
            # Not a generation: nothing is confined, as for a person who runs it on their own files.
            with patch.dict(os.environ):
                os.environ.pop("SCE_AUTHOR_WORK")
                os.environ.pop("SCE_FILE_ROOT", None)
                self.assertEqual(process.run(report, cwd=folder).stdout.strip(), "None")

    def test_a_template_named_by_a_path_is_answered_as_a_file_that_is_not_there(self):
        # The guard that reads a document's attributes is a list of the places somebody thought
        # of, so it is switched off here: it is the product that opens the file, and it holds
        # without the guard.
        marker = "CANARY-TEMPLATE-MARKER-7741"
        with tempfile.TemporaryDirectory() as outside, \
                patch.object(mcp, "_refuse_file_access_in"):
            template = os.path.join(outside, "secret.sce-template.xml")
            document = (f'<scxml {NS} initial="a"><sce:use template="{template}"/>'
                        f'<state id="a"/></scxml>')

            def ask():
                answer = mcp.call_tool("validate_scxml", {
                    "document_text": document, "document_name": "main.scxml"})
                return answer["content"][0]["text"]

            with open(template, "w", encoding="utf-8") as handle:
                handle.write(f'<sce:template {NS} name="outside"><state id="a">'
                             f'<transition event="{marker}" target="a"/></state></sce:template>')
            there = ask()
            os.remove(template)
            gone = ask()
        self.assertNotIn(marker, there)
        self.assertEqual(there, gone, "the file's being there is said")

    def test_a_template_handed_over_beside_the_document_is_expanded_as_it_was(self):
        # The boundary is not a refusal of every file: a template that is staged with the document
        # is inside it.
        template = (f'<sce:template {NS} name="shared"><state id="a">'
                    f'<transition event="tick" target="b"/></state><final id="b"/></sce:template>')
        answer = mcp.call_tool("validate_scxml", {
            "document_text": f'<scxml {NS} initial="a"><sce:use template="shared.xml"/></scxml>',
            "document_name": "main.scxml",
            "companions_text": [{"name": "shared.xml", "text": template}],
        })
        self.assertFalse(answer.get("isError", False), answer["content"][0]["text"][:400])

    def test_the_tools_a_generation_may_use_are_the_ones_the_application_approves(self):
        # The application names these in Rust, to approve them unattended and to enable them in
        # the client, and this server names them again to advertise and to refuse the rest. Two
        # lists kept by hand: they are the same one only for as long as this test says so.
        source = (pathlib.Path(__file__).resolve().parents[3]
                  / "app-core" / "src" / "client_run.rs").read_text(encoding="utf-8")
        declared = re.search(r"pub const AUTHOR_TOOLS: \[&str; \d+\] = \[(.*?)\];", source, re.S)
        self.assertIsNotNone(declared, "AUTHOR_TOOLS is not where this test looks for it")
        approved = set(re.findall(r'"([a-z_]+)"', declared.group(1)))
        self.assertEqual(approved, set(mcp.WORKBENCH_TOOLS))
