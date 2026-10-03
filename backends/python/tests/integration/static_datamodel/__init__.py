# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The machines ``scripts/regen_static_datamodel_python.sh`` generates, and the
algorithms they call, as one package.

A ``datamodel="sce-static"`` machine imports each algorithm it calls as a
sibling module (``from . import days_in_month``), the way a forge kind imports
another, so the two have to be modules of one package. Nothing generated is
committed: this marker and ``test_static_scenarios.py`` are the directory's
own files.
"""
