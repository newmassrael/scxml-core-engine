// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "log"

// LogRender writes the line a generated machine's `<sce:render>` writes once it
// has replaced `location` with the rendering of `template` (docs/adr/0087,
// section 3.1, item 6): the machine, the template and the variable, never the
// text, which may hold what a person wrote. The same line the Rust, C++ and C11
// machines write through their logs; a render whose hole failed raised
// error.execution instead and writes no line.
func LogRender(machine, template, location string) {
	log.Printf("[sce] %s: <sce:render template=\"%s\"> wrote %s", machine, template, location)
}
