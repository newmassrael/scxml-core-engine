"""A design from a caller on another machine is played only where the operator said how.

Playing a design runs code nobody has read, and the isolation this package gives
it is a child process under the kernel's limits (`process.isolation_level`). That
is enough for the owner's own assistant on the owner's own machine. It is not a
sandbox for a stranger: the limits stop a runaway, not a design that reads a file
or opens a socket. The README showed `--http 0.0.0.0` as the way to serve a
hosted assistant, and over it `compare` and `scxml_scenarios` played whatever
they were handed, so a server started as documented ran a remote caller's
designs under limits its operator had never been asked about.

So the default for a remote call is to withhold, the way the default for a path
is to refuse. A design is played for a remote caller when the operator names the
weakest isolation they accept (`--run-designs-under`) and the host gives at least
that, and the server will not start otherwise. What is withheld is said, with the
way to allow it; a comparison still answers every level that runs nothing.
"""

from __future__ import annotations

import contextlib
import http.client
import io
import json
import threading
import unittest
from contextlib import contextmanager
from unittest import mock

from sce_author import mcp, mcp_http, process
from sce_author.verify import _default_codegen

BUILT = unittest.skipUnless(_default_codegen().exists(),
                            "the product's code generator is not built")

DOOR = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" sce:kind="statechart" version="1.0" '
        'initial="closed">\n'
        '  <state id="closed"><transition event="open" target="opened"/></state>\n'
        '  <state id="opened"><transition event="close" target="closed"/></state>\n'
        '</scxml>\n')
# The same machine with its input called something else.
RENAMED = DOOR.replace('"open"', '"lift"')

QUOTE = "The door starts closed and opens when asked."
SPECIFICATION = QUOTE + "\n"
SCENARIOS = json.dumps({
    "record": "sce-scenario-set", "v": 1,
    "specification": {"doc_id": "door", "rev": "1"}, "origin": "ai-proposed",
    "interface": {"inputs": [{"name": "open"}], "outputs": [],
                  "conditions": ["closed", "opened"]},
    "scenarios": [{"id": "S1", "quote": QUOTE,
                   "steps": [{"expect": {"condition": "closed"}},
                             {"send": "open", "expect": {"condition": "opened"}}]}],
})

SCENARIO_ARGS = {"scenarios_text": SCENARIOS, "specification_text": SPECIFICATION,
                 "documents_text": [{"name": "door.scxml", "text": DOOR}]}
COMPARE_ARGS = {"documents_text": [{"name": "first.scxml", "text": DOOR},
                                   {"name": "second.scxml", "text": RENAMED}]}


@contextmanager
def nothing_may_start():
    """Any attempt to start a program that plays a design fails the case."""
    refusal = AssertionError("a design was played")
    with mock.patch("sce_author.process.Session", side_effect=refusal), \
            mock.patch("sce_author.process.run_isolated", side_effect=refusal):
        yield


def answer(reply: dict) -> dict:
    return json.loads(reply["content"][0]["text"])


class TheIsolationLevelsAreOrdered(unittest.TestCase):
    def test_a_level_is_ranked_by_its_name(self):
        ranks = [process.isolation_rank(level) for level in process.ISOLATION_LEVELS]
        self.assertEqual(sorted(ranks), ranks)
        self.assertEqual(len(set(ranks)), len(ranks))

    def test_a_description_ranks_as_the_name_it_begins_with(self):
        self.assertEqual(process.isolation_rank("process"),
                         process.isolation_rank("process (kernel limits are not enforced on darwin)"))

    def test_a_host_without_the_kernel_limits_ranks_below_one_with_them(self):
        self.assertLess(process.isolation_rank("process (kernel limits are not enforced on darwin)"),
                        process.isolation_rank("process+rlimit"))

    def test_a_level_nobody_defined_ranks_below_all(self):
        self.assertEqual(-1, process.isolation_rank("a sandbox nobody wrote"))


@BUILT
class ARemoteCallWithholdsUnlessTold(unittest.TestCase):
    def test_a_scenario_set_is_read_and_not_played(self):
        with nothing_may_start():
            reply = mcp.call_tool("scxml_scenarios", SCENARIO_ARGS, remote=True)
        self.assertFalse(reply.get("isError"), reply)
        body = answer(reply)
        self.assertEqual("not run", body["verdict"])
        self.assertIn("--run-designs-under", body["says"])
        # What reading the set found is still the owner's to see.
        self.assertTrue(body["set"]["usable"])

    def test_a_comparison_answers_every_level_that_runs_nothing(self):
        with nothing_may_start():
            reply = mcp.call_tool("compare", COMPARE_ARGS, remote=True)
        self.assertFalse(reply.get("isError"), reply)
        report = answer(reply)
        # The two drafts differ in bytes and in vocabulary, and say so.
        self.assertEqual(2, len(report["levels"]["bytes"]))
        self.assertEqual(2, len(report["levels"]["vocabulary"]))
        behaviour = report["behaviour"]
        self.assertEqual("not judged", behaviour["verdict"])
        self.assertIn("--run-designs-under", behaviour["why"])
        self.assertNotIn("classes", behaviour)

    def test_a_call_that_says_the_designs_may_be_played_plays_them(self):
        reply = mcp.call_tool("scxml_scenarios", SCENARIO_ARGS, remote=True,
                              designs_withheld=None)
        body = answer(reply)
        self.assertEqual("judged", body["verdict"])
        self.assertEqual(1, body["counts"]["pass"])
        self.assertEqual(process.isolation_level(), body["isolation"])

    def test_a_local_call_is_not_asked(self):
        """stdio: the owner started this server beside their own files."""
        body = answer(mcp.call_tool("scxml_scenarios", {
            **SCENARIO_ARGS, "scenarios_text": SCENARIOS}))
        self.assertEqual("judged", body["verdict"])


@BUILT
class OverHttp(unittest.TestCase):
    def serve(self, **policy) -> int:
        server = mcp_http.make_server("127.0.0.1", 0, None, **policy)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        return server.server_address[1]

    def call(self, port: int, name: str, arguments: dict) -> dict:
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=120)
        self.addCleanup(connection.close)
        connection.request("POST", "/mcp", json.dumps({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}}).encode(),
            {"Content-Type": "application/json"})
        reply = json.loads(connection.getresponse().read().decode())
        return answer(reply["result"])

    def test_a_server_started_with_no_word_about_isolation_plays_nothing(self):
        port = self.serve()
        with nothing_may_start():
            body = self.call(port, "scxml_scenarios", SCENARIO_ARGS)
        self.assertEqual("not run", body["verdict"])

    def test_a_server_whose_operator_accepted_the_level_plays_the_design(self):
        port = self.serve(run_designs_under="process+rlimit")
        body = self.call(port, "scxml_scenarios", SCENARIO_ARGS)
        self.assertEqual("judged", body["verdict"])
        self.assertEqual(1, body["counts"]["pass"])

    def test_a_comparison_over_it_is_played_when_accepted(self):
        port = self.serve(run_designs_under="process+rlimit")
        report = self.call(port, "compare", COMPARE_ARGS)
        self.assertEqual("judged", report["behaviour"]["verdict"])


class AServerRefusesToStartBelowTheLevelItWasAsked(unittest.TestCase):
    def test_a_host_that_gives_less_is_a_refusal_to_start(self):
        weaker = "process (kernel limits are not enforced on darwin)"
        with mock.patch("sce_author.process.isolation_level", return_value=weaker):
            with self.assertRaises(SystemExit) as caught:
                mcp_http.make_server("127.0.0.1", 0, None, run_designs_under="process+rlimit")
        self.assertIn("process+rlimit", str(caught.exception))
        self.assertIn("darwin", str(caught.exception))

    def test_a_host_that_gives_the_level_starts(self):
        server = mcp_http.make_server("127.0.0.1", 0, None, run_designs_under="process")
        server.server_close()

    def refused(self, *argv: str) -> str:
        """What the command line said when it refused these arguments."""
        said = io.StringIO()
        with contextlib.redirect_stderr(said), self.assertRaises(SystemExit) as caught:
            mcp.main(list(argv))
        self.assertEqual(2, caught.exception.code)
        return said.getvalue()

    def test_a_level_nobody_defined_is_not_accepted_on_the_command_line(self):
        said = self.refused("--http", "127.0.0.1:0", "--run-designs-under",
                            "a sandbox nobody wrote")
        # Said as a choice among the levels, not as an argument nobody knows.
        self.assertIn("invalid choice", said)
        self.assertIn("process+rlimit", said)

    def test_the_flag_goes_with_http(self):
        self.assertIn("goes with --http",
                      self.refused("--run-designs-under", "process+rlimit"))


if __name__ == "__main__":
    unittest.main()
