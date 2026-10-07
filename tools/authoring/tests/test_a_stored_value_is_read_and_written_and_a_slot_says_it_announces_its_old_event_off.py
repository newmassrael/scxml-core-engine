"""Two facts of the platform's addresses that no document and no specification can state.

`role: stored` -- a value the platform keeps, that the component reads when it starts and
writes back. One role per address was the rule, and an address read AND written could only be
declared as one of them: declared `input` the component's write was an expected position no
rule could be asked to write (a judged case passed without comparing it); declared `output`
the planted value was an address the component was not said to receive.

`announces_old_off` -- the component moves an event slot to another event by first publishing
the OLD event off. Of 244 product sources 79 read the cached event ID back; applied to one that
does not, the first publication of a direct switch is wrong, and left out of one that does, its
"event off" cases fail. So the model says it, per address.
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from sce_author.errors import PackError
from sce_author.pack import load_model
from sce_author.verify import received_by

MODEL = """\
version: 1
entries:
  - address: Plant.Memory.Level
    role: stored
    names: [Level]
    type: integer
  - address: Plant.Input.Switch
    role: input
    names: [Switch]
    values: {{"OFF": 0, "ON": 1}}
  - address: Plant.Event.Lamp
    role: {role}
    {announces}
    names: [Lamp]
    fields:
      ID:   {{type: text}}
      Stat: {{values: {{NONE: 0, "OFF": 1, "ON": 2}}}}
  - address: Plant.Output.Scalar
    role: output
    {scalar_announces}
    names: [Scalar]
    type: integer
"""


class AStoredValueIsReadAndWrittenAndASlotSaysItAnnouncesItsOldEventOff(unittest.TestCase):
    def model(self, role="output", announces="", scalar_announces=""):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        path = pathlib.Path(folder.name) / "interface-model.yaml"
        path.write_text(MODEL.format(role=role, announces=announces,
                                     scalar_announces=scalar_announces), encoding="utf-8")
        return load_model([path])

    def test_a_stored_value_is_an_output_position_and_an_input_of_the_document(self):
        model = self.model()
        self.assertIn("Plant.Memory.Level", [e.address for e in model.outputs()],
                      "the component writes it back, so a record can expect it")
        self.assertTrue(received_by(model, "Plant.Memory.Level"),
                        "the platform plants it and the component reads it")

    def test_an_input_is_not_an_output_position_and_an_output_is_not_received(self):
        """The discriminator: `stored` is the only role that is both."""
        model = self.model()
        written = [e.address for e in model.outputs()]
        self.assertNotIn("Plant.Input.Switch", written)
        self.assertFalse(received_by(model, "Plant.Event.Lamp"))
        self.assertTrue(received_by(model, "Plant.Input.Switch"))

    def test_a_slot_says_it_announces_its_old_event_off(self):
        model = self.model(announces="announces_old_off: true")
        self.assertTrue(model.by_address["Plant.Event.Lamp"].announces_old_off)
        self.assertFalse(self.model().by_address["Plant.Event.Lamp"].announces_old_off,
                         "a component that does not do it is the default")

    def test_it_is_refused_on_an_address_the_document_does_not_write(self):
        with self.assertRaises(PackError) as caught:
            self.model(role="input", announces="announces_old_off: true")
        self.assertIn("announces_old_off", str(caught.exception))
        self.assertIn("not written", str(caught.exception))

    def test_it_is_refused_where_there_is_no_stat_to_publish_off(self):
        with self.assertRaises(PackError) as caught:
            self.model(scalar_announces="announces_old_off: true")
        self.assertIn("without a Stat", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
