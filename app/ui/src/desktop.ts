// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What a desktop window offers the screen beyond the commands. A browser tab is
// asked "leave this page?" by the browser (`beforeunload`); a window is not, so the
// shell asks the SCREEN, and the screen asks the person. Only a Tauri window has
// one of these, and the screen works without it.

export interface Desktop {
  /** Tell the shell whether the screen holds changes the core has not been given. */
  unsaved(unsaved: boolean): void;
  /** The person decided the window may close, with or without what was unsaved. */
  close(): Promise<void>;
}
