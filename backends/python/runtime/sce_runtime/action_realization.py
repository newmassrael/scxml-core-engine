# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""What a host does with the duties of a machine (docs/adr/0038).

A `<sce:action effect="duty">` is an operation the machine's correctness
depends on the host performing. The generated host protocol of a machine that
states one carries a `realization` attribute with no default, so the author of
a host says beside the code whether it performs the duties or only simulates
them (a recorder, a test double), and the machine refuses a host that
simulates unless it is built for one on purpose with
`accept_simulated_host=True`.
"""

from enum import Enum


class ActionRealization(Enum):
    """What a host does with a machine's duties.

    ``PERFORMS`` is a statement the host's author makes about the bodies of
    its methods, in a line a reviewer can find. SCE cannot read the bodies, and
    an empty body under ``PERFORMS`` is that author's false statement and not an
    unwritten fact.
    """

    PERFORMS = "performs"
    """The host performs the duties: its methods do what the operations' names
    say."""

    SIMULATES = "simulates"
    """The host does not perform them: it records, drops or fakes them. A
    machine with a duty refuses such a host unless it is built with
    `accept_simulated_host=True`."""
