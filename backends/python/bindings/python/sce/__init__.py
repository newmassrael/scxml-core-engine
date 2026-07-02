# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""SCE - SCXML Core Engine Python bindings.

W3C SCXML 1.0 compliant state machine engine.

Usage:
    import sce

    engine = sce.Engine.from_file("workflow.scxml")
    engine.start()
    engine.send_event("next")
    print(engine.current_state)
    engine.stop()

Development build:
    PYTHONPATH=build_python/sce-python:sce-python/python python3 -c "import sce"
"""

from _sce import Engine, Statistics

__version__ = "1.0.0"
__all__ = ["Engine", "Statistics"]
