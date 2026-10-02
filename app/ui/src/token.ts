// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The browser shell's bearer token. The address the shell prints ends in
// `#token=...`; the fragment is never sent to a server, and it is taken out of the
// address bar as soon as it is read.
//
// A fragment is also the first thing a link handler drops (a terminal app that
// makes a printed address tappable, a chat that shortens it), so the screen does
// not depend on it: when the server wants a token and has none, the screen asks
// for it (see `Credentials`), and what is pasted may be the token or the whole
// address.

const KEY = "sce.token";
const FRAGMENT = /(?:^#|&)token=([^&]+)/;
const PASTED = /token=([^&\s#]+)/;

export interface TokenStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function keep(storage: TokenStorage | null, token: string): void {
  try {
    storage?.setItem(KEY, token);
  } catch {
    // A browser that refuses storage still has the token for this load.
  }
}

/** The token in the page's fragment, kept for the tab; else the one kept earlier. */
export function takeToken(hash: string, storage: TokenStorage | null): { token: string | null; cleanHash: string } {
  const match = FRAGMENT.exec(hash);
  if (match?.[1] !== undefined) {
    const token = decodeURIComponent(match[1]);
    keep(storage, token);
    return { token, cleanHash: "" };
  }
  let kept: string | null = null;
  try {
    kept = storage?.getItem(KEY) ?? null;
  } catch {
    kept = null;
  }
  return { token: kept, cleanHash: hash };
}

/** What a person pasted into the token field: the token itself, or an address that carries it. */
export function tokenFromPaste(pasted: string): string {
  const text = pasted.trim();
  const inAddress = PASTED.exec(text)?.[1];
  return inAddress === undefined ? text : decodeURIComponent(inAddress);
}

/** The token the transport sends, and the one way to replace it. */
export interface Credentials {
  token(): string | null;
  save(token: string): void;
}

export function credentials(initial: string | null, storage: TokenStorage | null): Credentials {
  let current = initial;
  return {
    token: () => current,
    save(token) {
      current = token;
      keep(storage, token);
    },
  };
}
