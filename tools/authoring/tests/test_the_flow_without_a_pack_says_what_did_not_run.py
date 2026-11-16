"""The flow without a pack says which tools did not run because there is no pack.

The workbench flow, and every flow this server serves without platform data, checks a
model with the product alone. The platform flow does more, and all of it needs a pack:
`questions` finds what the specification leaves open by comparing it with the pack, and
`check` holds a document's names and values to the platform's interface model. Until
2026-11-16 nothing said those did not run, so a model with nothing `open` read as one
about which nothing was open, when only what the model itself marked had been looked at.

What is held here: the list is the tool table's own (every tool whose input requires a
pack), the server's instructions name every tool on it, and the answers that report a
model checked in the workbench flow carry it.
"""

from __future__ import annotations

import unittest

from sce_author import mcp


def requires_a_pack(tool: dict) -> bool:
    return "pack" in tool["inputSchema"].get("required", ())


class TheFlowWithoutAPack(unittest.TestCase):
    def test_the_list_is_every_tool_that_requires_a_pack(self):
        names = [entry["tool"] for entry in mcp.WITHOUT_A_PACK]
        self.assertEqual([t["name"] for t in mcp.TOOLS if requires_a_pack(t)], names)
        # The two a person reading a model most needs to hear are absent.
        self.assertIn("questions", names)
        self.assertIn("check", names)
        # And nothing of this flow is named as not run.
        for flow_tool in ("validate_scxml", "render_scxml_pseudocode", "specs_save_model"):
            self.assertNotIn(flow_tool, names)

    def test_each_entry_says_what_the_tool_does_in_its_own_words(self):
        for entry in mcp.WITHOUT_A_PACK:
            with self.subTest(tool=entry["tool"]):
                tool = next(t for t in mcp.TOOLS if t["name"] == entry["tool"])
                self.assertTrue(tool["description"].startswith(entry["does"]))
                self.assertTrue(entry["does"].endswith("."), entry["does"])

    def test_a_tool_that_comes_to_require_a_pack_is_named_at_once(self):
        added = {"name": "probe", "description": "Probe the pack. More words.",
                 "inputSchema": {"type": "object", "required": ["pack"]}}
        listed = mcp.without_a_pack(mcp.TOOLS + [added])
        self.assertEqual({"tool": "probe", "does": "Probe the pack."}, listed[-1])
        self.assertEqual(len(mcp.WITHOUT_A_PACK) + 1, len(listed))

    def test_the_instructions_name_every_tool_that_did_not_run(self):
        instructions = mcp.SERVER_INSTRUCTIONS
        start = instructions.index("No pack or binding is needed for this flow")
        sentence = instructions[start:instructions.index("When the owner keeps", start)]
        for entry in mcp.WITHOUT_A_PACK:
            self.assertIn(entry["tool"], sentence)
        self.assertIn("sce:unresolved", sentence)
        self.assertIn("`without_a_pack`", sentence)

    def test_the_answer_a_workbench_tool_carries_names_the_same_tools(self):
        answer = mcp._WITHOUT_A_PACK_ANSWER
        self.assertEqual(mcp.WITHOUT_A_PACK, answer["not_run"])
        self.assertIn("sce:unresolved", answer["reason"])


if __name__ == "__main__":
    unittest.main()
