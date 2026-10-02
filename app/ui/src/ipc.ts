// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen's one way to the core: a command name and its arguments in, a JSON
// value out. In the window that is Tauri's `invoke`; in a browser it is a POST to
// the browser shell. Nothing above this file knows which.

import { asCommandError } from "./contract";

export type Args = Record<string, unknown>;

export interface Transport {
  call(name: string, args?: Args): Promise<unknown>;
}

/** A command that failed, or a call that never reached one. */
export class CommandFailure extends Error {
  constructor(
    /** What a program branches on: the core's kinds, plus `transport` and `unauthorized`. */
    readonly kind: string,
    message: string,
    readonly detail?: unknown,
  ) {
    super(message);
    this.name = "CommandFailure";
  }
}

/** Kinds that belong to the wire, not to a command. */
export const TRANSPORT = "transport";
export const UNAUTHORIZED = "unauthorized";

export interface HttpOptions {
  readonly fetch: typeof fetch;
  readonly token: () => string | null;
  /** Where `/api/call` lives; the page's own origin when empty. */
  readonly base?: string;
}

export function httpTransport(options: HttpOptions): Transport {
  const url = `${options.base ?? ""}/api/call`;
  return {
    async call(name, args = {}) {
      const token = options.token();
      let response: Response;
      try {
        response = await options.fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            ...(token === null ? {} : { Authorization: `Bearer ${token}` }),
          },
          body: JSON.stringify({ name, args }),
        });
      } catch (cause) {
        throw new CommandFailure(TRANSPORT, `the server could not be reached: ${String(cause)}`);
      }
      let body: unknown;
      try {
        body = await response.json();
      } catch {
        throw new CommandFailure(TRANSPORT, `the server answered ${response.status} with something that is not JSON`);
      }
      if (response.ok) return body;
      const refusal = asCommandError(body);
      if (refusal === null) {
        throw new CommandFailure(TRANSPORT, `the server answered ${response.status} with a body that is not a refusal`);
      }
      throw new CommandFailure(refusal.kind, refusal.message, refusal.detail);
    },
  };
}

/** Whether this page is running inside a Tauri window. */
export function insideTauri(scope: object): boolean {
  return "__TAURI_INTERNALS__" in scope;
}

/** What a rejected `invoke` carries, as a `CommandFailure`. */
export function failureFromInvoke(rejection: unknown): CommandFailure {
  const refusal = asCommandError(rejection);
  if (refusal !== null) return new CommandFailure(refusal.kind, refusal.message, refusal.detail);
  return new CommandFailure(TRANSPORT, `the application did not answer: ${String(rejection)}`);
}
