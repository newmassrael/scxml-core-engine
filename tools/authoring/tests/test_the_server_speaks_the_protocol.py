"""The MCP server, driven over a pipe the way a client drives it.

Testing `call_tool` alone would prove the functions work and nothing about the
protocol, and the protocol is the part a client sees. So these cases write
JSON-RPC frames into the server and read frames back.

The load-bearing case is the last one: **the server does not write documents.**
If a tool named `generate` ever appears here, the split this core is built on
has been quietly abandoned, and the test is what makes that a failure rather
than a surprise.
"""

import io
import json
import pathlib
import tempfile
import unittest

import yaml

from sce_author import mcp

from tests.test_refusals_actually_fire import (
    BINDING, CONVENTIONS, DOCUMENT, EXAMPLES, MODEL,
)


def drive(*messages):
    """Write frames in, read frames out."""
    stdin = io.StringIO("".join(json.dumps(m) + "\n" for m in messages))
    stdout = io.StringIO()
    mcp.serve(stdin, stdout)
    return [json.loads(line) for line in stdout.getvalue().splitlines() if line.strip()]


class TheServerSpeaksTheProtocol(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self._tmp.name)
        self.pack_dir = self.root / "pack"
        self.pack_dir.mkdir()
        (self.pack_dir / "interface-model.yaml").write_text(yaml.safe_dump(MODEL), encoding="utf-8")
        (self.pack_dir / "conventions.yaml").write_text(yaml.safe_dump(CONVENTIONS), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump(EXAMPLES), encoding="utf-8")
        (self.root / "fixture.scxml").write_text(DOCUMENT, encoding="utf-8")
        self.binding = self.root / "fixture.binding.yaml"
        self.binding.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        self.spec = self.root / "spec.md"
        self.spec.write_text(
            "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
            "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n",
            encoding="utf-8",
        )

    def tearDown(self):
        self._tmp.cleanup()

    def call(self, name, **arguments):
        replies = drive({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": arguments},
        })
        self.assertEqual(1, len(replies))
        return replies[0]["result"]

    def test_it_announces_a_protocol_version_and_its_tools(self):
        replies = drive(
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
        )
        self.assertEqual(mcp.PROTOCOL_VERSION, replies[0]["result"]["protocolVersion"])
        self.assertIn("tools", replies[0]["result"]["capabilities"])
        names = {t["name"] for t in replies[1]["result"]["tools"]}
        self.assertEqual({"brief", "questions", "review", "check", "verify"},
                         names)

    def test_the_two_surfaces_offer_the_same_commands(self):
        """⚠ THE CASE THAT WOULD HAVE CAUGHT `review` BEING MISSING HERE.

        A command exists twice -- once on the command line and once over this
        transport -- and nothing tied the two together, so one was added to
        the first and forgotten on the second. What it cost is the shape that
        matters: a caller reaching this core over MCP had no way to ask
        whether the pack it was being answered from was any good, which is the
        first question when a pack is new or hand-written, and every other
        tool trusts that pack silently.

        The list above is written out by hand on purpose -- two tests that
        derive the same set from the same place agree with each other and say
        nothing. This one derives the CLI's half from the CLI.
        """
        import ast

        source = (pathlib.Path(mcp.__file__).parent / "__main__.py").read_text(
            encoding="utf-8")
        commands = set()
        for node in ast.walk(ast.parse(source)):
            if (isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Attribute)
                    and node.func.attr == "add_parser"
                    and node.args
                    and isinstance(node.args[0], ast.Constant)):
                commands.add(node.args[0].value)
        self.assertTrue(commands, "no subcommands found -- the scan is broken")
        self.assertEqual(commands, {t["name"] for t in mcp.TOOLS})

    def test_a_notification_is_answered_with_silence(self):
        """Replying to a notification is a protocol error, and the client that
        receives one has no id to match it against."""
        self.assertEqual([], drive({"jsonrpc": "2.0", "method": "notifications/initialized"}))

    def test_an_unknown_method_is_an_error_not_a_crash(self):
        replies = drive({"jsonrpc": "2.0", "id": 7, "method": "tools/summon"})
        self.assertEqual(-32601, replies[0]["error"]["code"])

    def test_a_torn_line_does_not_stop_the_server(self):
        """A server that dies takes its diagnosis with it."""
        replies = drive_raw('{"jsonrpc": "2.0", "id": 1, "method": "ping"}\nnot json\n'
                            '{"jsonrpc": "2.0", "id": 2, "method": "ping"}\n')
        self.assertEqual(3, len(replies))
        self.assertEqual(-32700, replies[1]["error"]["code"])
        self.assertEqual(2, replies[2]["id"])

    def test_brief_returns_the_page(self):
        result = self.call("brief", pack=str(self.pack_dir), prose=[str(self.spec)])
        page = result["content"][0]["text"]
        self.assertIn("Fig_Lamp_stat", page)
        self.assertIn("Plant.Out.Lamp", page)

    def test_questions_returns_them_as_data(self):
        result = self.call("questions", pack=str(self.pack_dir), prose=[str(self.spec)])
        payload = json.loads(result["content"][0]["text"])
        self.assertEqual(1, payload["version"])
        self.assertEqual([], payload["questions"])
        self.assertEqual({}, payload["counts"])

    def test_every_question_carries_what_a_program_needs(self):
        """A caller that draws markers needs four things the prose answer does
        not give it: which class, how loudly, where, and whether this is the
        same question it saw last run."""
        self.spec.write_text(
            "Fig_Lamp_stat is ON when In_SupplyMode == BLINKING, and OFF otherwise.\n"
            "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n",
            encoding="utf-8",
        )
        result = self.call("questions", pack=str(self.pack_dir), prose=[str(self.spec)])
        payload = json.loads(result["content"][0]["text"])
        found = payload["questions"]
        self.assertTrue(found)
        for q in found:
            self.assertIn(q["severity"], {"error", "warning", "note"})
            self.assertRegex(q["id"], r"^[0-9a-f]{12}$")
        self.assertEqual(payload["counts"],
                         {"value-not-in-space": 1})

    def test_the_same_question_keeps_its_id_across_runs(self):
        """Without this nothing can be dismissed or tracked; every run is a
        fresh list of strangers."""
        self.spec.write_text(
            "Fig_Lamp_stat is ON when In_SupplyMode == BLINKING, and OFF otherwise.\n"
            "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n",
            encoding="utf-8",
        )
        args = dict(pack=str(self.pack_dir), prose=[str(self.spec)])
        first = json.loads(self.call("questions", **args)["content"][0]["text"])
        second = json.loads(self.call("questions", **args)["content"][0]["text"])
        self.assertEqual([q["id"] for q in first["questions"]],
                         [q["id"] for q in second["questions"]])

    def test_a_question_with_no_line_carries_no_line(self):
        """No location is not location zero. A caller drawing at line 0 puts a
        marker next to a sentence that has nothing to do with the question."""
        self.spec.write_text(
            "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n",
            encoding="utf-8",
        )
        result = self.call("questions", pack=str(self.pack_dir), prose=[str(self.spec)],
                           kind="no-decision-logic")
        payload = json.loads(result["content"][0]["text"])
        self.assertTrue(payload["questions"])
        for q in payload["questions"]:
            self.assertNotIn("line", q)
            self.assertNotIn("file", q)

    def test_review_answers_with_figures_and_its_alarms_apart(self):
        """The figures are counts somebody still has to interpret; `alarms` is
        the only part that claims anything. A caller that folded them together
        would read "attribution 0.31" as a complaint.
        """
        result = self.call("review", pack=str(self.pack_dir),
                           prose=[str(self.spec)])
        payload = json.loads(result["content"][0]["text"])
        self.assertEqual(1, payload["version"])
        self.assertIn("prose_attributed", payload["figures"])
        self.assertIn("output_positions_never_expected", payload["figures"])
        self.assertIsInstance(payload["alarms"], list)
        self.assertGreater(payload["figures"]["addresses"], 0)

    def test_check_reports_a_refusal_as_an_error_result(self):
        broken = dict(BINDING)
        broken["outputs"] = {"lamp": {"address": "Plant.Out.Ghost", "field": "Stat", "map": {1: "ON"}}}
        self.binding.write_text(yaml.safe_dump(broken), encoding="utf-8")
        result = self.call("check", pack=str(self.pack_dir), binding=str(self.binding))
        self.assertTrue(result.get("isError"))
        self.assertIn("not in the interface model", result["content"][0]["text"])

    def test_a_missing_pack_comes_back_as_an_answer(self):
        result = self.call("brief", pack=str(self.root / "nowhere"), prose=[str(self.spec)])
        self.assertTrue(result.get("isError"))
        self.assertIn("not a directory", result["content"][0]["text"])

    def test_the_server_offers_no_way_to_write_a_document(self):
        """The split this core is built on, held as a test.

        The tools hand a model the materials and judge what it wrote. A tool
        that produced the document would be a mechanical translator, and one
        built against this corpus reached 6 of 17 cases where a model reading
        the same materials reached 17 of 17.
        """
        names = {t["name"] for t in mcp.TOOLS}
        for forbidden in ("generate", "convert", "write", "translate", "emit"):
            self.assertNotIn(forbidden, names)
        for tool in mcp.TOOLS:
            self.assertNotIn("scxml", tool["inputSchema"].get("properties", {}))


def drive_raw(payload):
    stdout = io.StringIO()
    mcp.serve(io.StringIO(payload), stdout)
    return [json.loads(line) for line in stdout.getvalue().splitlines() if line.strip()]


if __name__ == "__main__":
    unittest.main()
