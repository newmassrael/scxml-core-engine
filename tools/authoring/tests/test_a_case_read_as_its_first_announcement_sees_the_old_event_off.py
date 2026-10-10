"""A case read as `observed: first` sees the old event off when the slot announces it.

A document computes where an event slot stands AFTER a round. A component that
`announces_old_off` moves the slot from one event to another by first publishing the OLD event
off, so the first announcement of such a round is `(old ID, OFF)` and not the state the round
ends in. `verify`'s pure path judged the end state for every case, so a case whose first
announcement was the old event off failed for a document that behaved as the component does.
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from sce_author.pack import load_model
from sce_author.verify import History, first_announcement

MODEL = """\
version: 1
entries:
  - address: Plant.Event.Lamp
    role: output
    {announces}
    names: [Lamp]
    fields:
      ID:   {{type: text}}
      Stat: {{values: {{NONE: 0, "OFF": 1, "ON": 2}}}}
"""

ID, STAT = "Plant.Event.Lamp.ID", "Plant.Event.Lamp.Stat"


class ACaseReadAsItsFirstAnnouncementSeesTheOldEventOff(unittest.TestCase):
    def model(self, announces="announces_old_off: true"):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        path = pathlib.Path(folder.name) / "interface-model.yaml"
        path.write_text(MODEL.format(announces=announces), encoding="utf-8")
        return load_model([path])

    @staticmethod
    def after(published: dict, started: bool = True) -> History:
        history = History()
        history.published.update(published)
        history.started = started
        return history

    def test_a_slot_moved_to_another_event_while_shown_announces_the_old_one_off_first(self):
        history = self.after({ID: "E1", STAT: "ON"})

        first = first_announcement(self.model(), history, {ID: "E2", STAT: "ON"})

        self.assertEqual(first, {ID: "E1", STAT: "OFF"})

    def test_a_slot_that_was_already_off_publishes_nothing_before_the_new_event(self):
        history = self.after({ID: "E1", STAT: "OFF"})

        self.assertEqual(first_announcement(self.model(), history, {ID: "E2", STAT: "ON"}), {})

    def test_the_same_event_again_is_no_move(self):
        history = self.after({ID: "E1", STAT: "ON"})

        self.assertEqual(first_announcement(self.model(), history, {ID: "E1", STAT: "ON"}), {})

    def test_a_component_that_does_not_announce_the_old_event_off_is_read_as_it_ends(self):
        history = self.after({ID: "E1", STAT: "ON"})

        first = first_announcement(self.model(announces=""), history, {ID: "E2", STAT: "ON"})

        self.assertEqual(first, {})

    def test_the_first_round_has_no_old_event(self):
        history = self.after({}, started=False)

        self.assertEqual(first_announcement(self.model(), history, {ID: "E2", STAT: "ON"}), {})

    def test_a_slot_that_was_never_written_has_no_old_event(self):
        history = self.after({STAT: "ON"})

        self.assertEqual(first_announcement(self.model(), history, {ID: "E2", STAT: "ON"}), {})

    def test_a_position_the_run_could_not_settle_is_not_read_into(self):
        history = self.after({ID: "E1", STAT: "ON"})

        first = first_announcement(self.model(), history, {ID: "E2", STAT: "ON"},
                                   undetermined=frozenset({ID}))

        self.assertEqual(first, {})

    def test_a_document_run_with_no_history_is_read_as_it_ends(self):
        self.assertEqual(first_announcement(self.model(), None, {ID: "E2", STAT: "ON"}), {})


if __name__ == "__main__":
    unittest.main()
