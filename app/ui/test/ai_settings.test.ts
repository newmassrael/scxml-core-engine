// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// @vitest-environment jsdom

// The AI settings as a person meets them: what is asked when the screen opens, what is shown for
// each state Claude Code can be in, and what a save sends. The core is a fake that records what
// it was asked, because what matters here is which commands go out and with which arguments
// (the base revision a save is written from, the default it expects to find).

import { beforeEach, describe, expect, it } from "vitest";

import { AiSettings, type AiSettingsHost } from "../src/ai_settings";
import { CLAUDE_CONNECTION_ID, CODEX_CONNECTION_ID } from "../src/ai_settings_model";
import type { Api } from "../src/api";
import { SUPPORTED_COMMAND_SET_VERSION } from "../src/contract";
import type {
  Candidate,
  ClaudeAccount,
  ClaudeStatus,
  CodexStatus,
  Connection,
  ConnectionListing,
  Described,
  Revision,
  Saved,
} from "../src/contract";
import { CommandFailure } from "../src/ipc";
import { translate } from "../src/i18n";

const REVISION_1 = "1".repeat(64);
const REVISION_2 = "2".repeat(64);

/** The program the client answered from. */
const PROGRAM = "/home/me/.local/bin/claude";
/** Another one the application found. */
const OTHER_PROGRAM = "/usr/local/bin/claude";

const DESKTOP: Described = {
  command_set_version: SUPPORTED_COMMAND_SET_VERSION,
  commands: [],
  root: "/works",
  entrance: "desktop",
  settings: true,
  writes_settings: true,
  starts_programs: true,
};
const BROWSER: Described = { ...DESKTOP, entrance: "browser", writes_settings: false, starts_programs: false };

const SIGN_IN = [
  { billing: "subscription", command: "claude auth login" },
  { billing: "usage", command: "claude auth login --console" },
] as const;

const signedIn = (over: Partial<Extract<ClaudeAccount, { state: "signed-in" }>> = {}): ClaudeAccount => ({
  state: "signed-in",
  route: "claude-official-login",
  billing: "subscription",
  environment: null,
  usable: true,
  decision: { decision: "use", status: "conditional" },
  ...over,
});

const statusOf = (account: ClaudeAccount): ClaudeStatus => ({
  client: { state: "installed", version: "2.1.291", path: PROGRAM },
  account,
  sign_in: SIGN_IN,
});

const connection = (model: string | null): Connection => ({
  id: CLAUDE_CONNECTION_ID,
  adapter: "claude-code",
  display_name: null,
  executable: null,
  model,
  auth: "official-login",
  server_url: null,
  limits: { turns: 40, seconds: null },
});

/** The program Codex answered from, and another one the application found. */
const CODEX_PROGRAM = "/home/me/.local/bin/codex";
const OTHER_CODEX = "/usr/local/bin/codex";
const CODEX_HOME = "/home/me/.local/share/sce/settings/codex-home";

const CODEX_SIGN_IN: CodexStatus["sign_in"] = [
  { source: "official-login", billing: "subscription", command: "codex login -c cli_auth_credentials_store=file", home: null },
  { source: "official-login", billing: "usage", command: "codex login --with-api-key -c cli_auth_credentials_store=file", home: null },
  { source: "app-store", billing: "subscription", command: "codex login -c cli_auth_credentials_store=file", home: CODEX_HOME },
  { source: "app-store", billing: "usage", command: "codex login --with-api-key -c cli_auth_credentials_store=file", home: CODEX_HOME },
];

const codexSignedIn = (over: Partial<Extract<ClaudeAccount, { state: "signed-in" }>> = {}): ClaudeAccount =>
  signedIn({ route: "codex-cli-chat-gpt-login", ...over });

/** What the core says of Codex: installed, verified, and signed in by the official client's login. */
function codexStatusOf(over: Partial<CodexStatus> = {}, accounts?: [ClaudeAccount, ClaudeAccount, ClaudeAccount]): CodexStatus {
  const [official, store, key] = accounts ?? [codexSignedIn(), { state: "signed-out" }, { state: "signed-out" }];
  return {
    client: { state: "installed", version: "0.159.0", path: CODEX_PROGRAM },
    support: { state: "verified" },
    accounts: [
      { ...official, source: "official-login" },
      { ...store, source: "app-store" },
      { ...key, source: "env-api-key" },
    ],
    sign_in: CODEX_SIGN_IN,
    key_variable: "CODEX_API_KEY",
    ...over,
  };
}

const codexConnection = (over: Partial<Connection> = {}): Connection => ({
  id: CODEX_CONNECTION_ID,
  adapter: "codex",
  display_name: null,
  executable: null,
  model: null,
  auth: "official-login",
  server_url: null,
  limits: { turns: null, seconds: 900 },
  ...over,
});

/** A core that keeps one list of connections and says what it was asked. */
class FakeSettings {
  readonly calls: Array<{ name: string; args: unknown }> = [];
  listing: ConnectionListing = { connections: [], unreadable: [], default: null };
  status: ClaudeStatus | CommandFailure = statusOf(signedIn());
  /** Refuse the next save with this, once. */
  refuseSave: CommandFailure | null = null;
  /** The programs the application says it found. */
  found: Candidate[] = [
    { path: PROGRAM, version: "2.1.291", found: "search-path" },
    { path: OTHER_PROGRAM, version: "2.1.280", found: "known-location" },
  ];
  /** Refuse to find programs with this, as the core does an entrance that may not start one. */
  refuseFinding: CommandFailure | null = null;
  /** What the core says of Codex. */
  codexStatus: CodexStatus | CommandFailure = codexStatusOf();
  /** The Codex programs the application says it found. */
  foundCodex: Candidate[] = [
    { path: CODEX_PROGRAM, version: "0.159.0", found: "search-path" },
    { path: OTHER_CODEX, version: "0.158.0", found: "known-location" },
  ];

  api(): Pick<
    Api,
    | "listConnections"
    | "saveConnection"
    | "setDefaultConnection"
    | "readClaudeStatus"
    | "readCodexStatus"
    | "findClients"
  > {
    return {
      findClients: async () => {
        this.calls.push({ name: "find_clients", args: null });
        if (this.refuseFinding !== null) throw this.refuseFinding;
        return { claude: this.found, codex: this.foundCodex };
      },
      readCodexStatus: async () => {
        this.calls.push({ name: "read_codex_status", args: null });
        if (this.codexStatus instanceof CommandFailure) throw this.codexStatus;
        return this.codexStatus;
      },
      listConnections: async () => {
        this.calls.push({ name: "list_connections", args: null });
        return this.listing;
      },
      readClaudeStatus: async () => {
        this.calls.push({ name: "read_claude_status", args: null });
        if (this.status instanceof CommandFailure) throw this.status;
        return this.status;
      },
      saveConnection: async (saved: Connection, base: Revision | null): Promise<Saved> => {
        this.calls.push({ name: "save_connection", args: { connection: saved, base } });
        if (this.refuseSave !== null) {
          const refusal = this.refuseSave;
          this.refuseSave = null;
          throw refusal;
        }
        // What is kept under this id is replaced, and what is kept under another stays.
        this.listing = {
          ...this.listing,
          connections: [
            ...this.listing.connections.filter((c) => c.connection.id !== saved.id),
            { connection: saved, revision: REVISION_2 },
          ],
        };
        return { outcome: "saved", revision: REVISION_2, parent: base };
      },
      setDefaultConnection: async (id: string | null, expect: string | null) => {
        this.calls.push({ name: "set_default_connection", args: { id, expect } });
        this.listing = { ...this.listing, default: id };
        return id;
      },
    };
  }

  asked(name: string): unknown[] {
    return this.calls.filter((c) => c.name === name).map((c) => c.args);
  }
}

interface Rig {
  readonly core: FakeSettings;
  readonly settings: AiSettings;
  readonly copied: string[];
  redraws: number;
  readonly root: HTMLElement;
  draw(): void;
}

function rig(described: Described = DESKTOP, options: { copy?: (text: string) => Promise<void> } = {}): Rig {
  const core = new FakeSettings();
  const copied: string[] = [];
  const root = document.createElement("div");
  document.body.replaceChildren(root);
  const state = { redraws: 0 };
  const host: AiSettingsHost = {
    api: core.api(),
    described,
    t: (key, values) => translate("en", key, values),
    redraw: () => {
      state.redraws += 1;
      draw();
    },
    handled: () => false,
    explain: (error) => (error instanceof Error ? error.message : String(error)),
    copy:
      options.copy ??
      (async (text) => {
        copied.push(text);
      }),
  };
  const settings = new AiSettings(host);
  const draw = () => {
    const view = settings.view();
    root.replaceChildren(...(view === null ? [] : [view]));
  };
  return {
    core,
    settings,
    copied,
    get redraws() {
      return state.redraws;
    },
    root,
    draw,
  } as Rig;
}

const click = (root: HTMLElement, selector: string): void => {
  const button = root.querySelector<HTMLElement>(selector);
  if (button === null) throw new Error(`no ${selector} on screen: ${root.textContent}`);
  button.click();
};

const settle = async (): Promise<void> => {
  for (let i = 0; i < 6; i += 1) await Promise.resolve();
};

beforeEach(() => {
  document.body.replaceChildren();
});

describe("what is asked when the screen opens", () => {
  it("is the connections and who is signed in, in the desktop window", async () => {
    const r = rig();

    await r.settings.load();

    expect(r.core.calls.map((c) => c.name)).toEqual(["list_connections", "read_claude_status", "find_clients"]);
  });

  it("is only the connections in a window that may not start a program, and Claude Code is not asked", async () => {
    const r = rig(BROWSER);

    await r.settings.load();
    r.draw();

    expect(r.core.calls.map((c) => c.name)).toEqual(["list_connections"]);
    expect(r.root.textContent).toContain("desktop application");
  });

  it("is nothing at all when the entrance has no settings folder", async () => {
    const r = rig({ ...BROWSER, settings: false });

    await r.settings.load();

    expect(r.core.calls).toEqual([]);
    expect(r.settings.view()).toBeNull();
  });

  it("is a refusal to ask, said as such, and the connections are still read", async () => {
    const r = rig();
    r.core.status = new CommandFailure("not-allowed-here", "only the desktop application does that");

    await r.settings.load();
    r.draw();

    expect(r.core.asked("list_connections")).toHaveLength(1);
    expect(r.root.textContent).toContain("desktop application");
  });
});

describe("what is shown for each state Claude Code can be in", () => {
  const shown = async (status: ClaudeStatus): Promise<Rig> => {
    const r = rig();
    r.core.status = status;
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    return r;
  };

  it("says to install it when there is none, and does not offer to sign in", async () => {
    const r = await shown({ client: { state: "missing" }, account: { state: "unknown", reason: "x" }, sign_in: SIGN_IN });

    expect(r.root.textContent).toContain("not installed");
    expect(r.root.querySelector("code")).toBeNull();
  });

  it("gives the two commands that sign in when nobody is, each with its billing", async () => {
    const r = await shown({ client: { state: "installed", version: "2.1.291", path: PROGRAM }, account: { state: "signed-out" }, sign_in: SIGN_IN });

    const commands = [...r.root.querySelectorAll("code")].map((c) => c.textContent);
    expect(commands).toEqual(["claude auth login", "claude auth login --console"]);
    expect(r.root.textContent).toContain("Subscription");
    expect(r.root.textContent).toContain("billed by API use");
    // The application starts no sign-in of its own: there is no button named for one.
    expect(r.root.querySelector("#ai-sign-in")).toBeNull();
  });

  it("says a client that could not be asked could not be asked, and does not send anyone to sign in", async () => {
    const r = await shown(statusOf({ state: "unknown", reason: "it did not answer within 20 seconds" }));

    expect(r.root.textContent).toContain("it did not answer within 20 seconds");
    expect(r.root.querySelector("code")).toBeNull();
  });

  it("says a login the build does not use is a login that is there, and offers no model to choose", async () => {
    const r = await shown(
      statusOf(
        signedIn({
          route: "unlisted",
          billing: null,
          usable: false,
          decision: { decision: "refuse", status: "unconfirmed", reason: "unconfirmed" },
        }),
      ),
    );

    expect(r.root.textContent).toContain("does not use");
    expect(r.root.querySelector("#ai-save")).toBeNull();
    expect(r.root.querySelector("#ai-model")).toBeNull();
  });

  it("says how a subscription is billed, and offers the models and the save", async () => {
    const r = await shown(statusOf(signedIn()));

    expect(r.root.textContent).toContain("subscription login");
    expect(r.root.querySelector("#ai-save")).not.toBeNull();
    const models = [...r.root.querySelectorAll<HTMLOptionElement>("#ai-model option")].map((o) => o.value);
    expect(models).toEqual(["", "opus", "sonnet", "haiku"]);
  });

  it("names the variable that decided a key, and says it is used whatever else is signed in", async () => {
    const r = await shown(
      statusOf(signedIn({ route: "claude-api-key", billing: "usage", environment: "ANTHROPIC_API_KEY" })),
    );

    expect(r.root.textContent).toContain("ANTHROPIC_API_KEY");
    expect(r.root.textContent).toContain("billed by use");
    expect(r.root.textContent).toContain("used even if");
  });
});

describe("copying a command", () => {
  it("puts the command on the clipboard and says so", async () => {
    const r = rig();
    r.core.status = { client: { state: "installed", version: "2.1.291", path: PROGRAM }, account: { state: "signed-out" }, sign_in: SIGN_IN };
    await r.settings.load();
    r.settings.openPanel();
    r.draw();

    click(r.root, "[data-copy='claude auth login --console']");
    await settle();

    expect(r.copied).toEqual(["claude auth login --console"]);
    expect(r.root.textContent).toContain("Copied");
  });

  it("says to select it by hand when the clipboard refuses", async () => {
    const r = rig(DESKTOP, {
      copy: async () => {
        throw new Error("denied");
      },
    });
    r.core.status = { client: { state: "installed", version: "2.1.291", path: PROGRAM }, account: { state: "signed-out" }, sign_in: SIGN_IN };
    await r.settings.load();
    r.settings.openPanel();
    r.draw();

    click(r.root, "[data-copy='claude auth login']");
    await settle();

    expect(r.root.textContent).toContain("select the command");
  });
});

describe("asking again", () => {
  it("asks Claude Code again and not the connections, and shows what it says now", async () => {
    const r = rig();
    r.core.status = { client: { state: "installed", version: "2.1.291", path: PROGRAM }, account: { state: "signed-out" }, sign_in: SIGN_IN };
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    r.core.status = statusOf(signedIn());

    click(r.root, "#ai-recheck");
    await settle();

    expect(r.core.asked("read_claude_status")).toHaveLength(2);
    expect(r.core.asked("list_connections")).toHaveLength(1);
    expect(r.root.querySelector("#ai-save")).not.toBeNull();
  });
});

describe("saving the connection", () => {
  const ready = async (): Promise<Rig> => {
    const r = rig();
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    return r;
  };

  it("saves a first connection on top of nothing and makes it the default, expecting none", async () => {
    const r = await ready();
    const select = r.root.querySelector<HTMLSelectElement>("select")!;
    select.value = "opus";
    select.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save");
    await settle();

    expect(r.core.asked("save_connection")).toEqual([
      { connection: { ...connection("opus"), limits: { turns: null, seconds: null } }, base: null },
    ]);
    expect(r.core.asked("set_default_connection")).toEqual([{ id: CLAUDE_CONNECTION_ID, expect: null }]);
    expect(r.root.textContent).toContain("from the next generation");
    // And what the person reads next is the connection as kept.
    expect(r.settings.connectionForRequest()).toEqual({ id: CLAUDE_CONNECTION_ID, revision: REVISION_2 });
  });

  it("writes on top of the revision it read, keeps the limits it had, and expects the default it saw", async () => {
    const r = rig();
    r.core.listing = {
      connections: [{ connection: connection("sonnet"), revision: REVISION_1 }],
      unreadable: [],
      default: "pc2",
    };
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    const select = r.root.querySelector<HTMLSelectElement>("select")!;
    expect(select.value).toBe("sonnet");
    select.value = "haiku";
    select.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save");
    await settle();

    expect(r.core.asked("save_connection")).toEqual([{ connection: connection("haiku"), base: REVISION_1 }]);
    expect(r.core.asked("set_default_connection")).toEqual([{ id: CLAUDE_CONNECTION_ID, expect: "pc2" }]);
  });

  it("saves the client's own default as no model", async () => {
    const r = rig();
    r.core.listing = {
      connections: [{ connection: connection("opus"), revision: REVISION_1 }],
      unreadable: [],
      default: CLAUDE_CONNECTION_ID,
    };
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    const select = r.root.querySelector<HTMLSelectElement>("select")!;
    select.value = "";
    select.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save");
    await settle();

    expect((r.core.asked("save_connection")[0] as { connection: Connection }).connection.model).toBeNull();
  });

  it("saves a model id the person typed, which wins over the list, and keeps it as a choice afterwards", async () => {
    const r = await ready();
    const field = r.root.querySelector<HTMLInputElement>("#ai-model-id")!;
    field.value = "  claude-opus-4-1  ";
    field.dispatchEvent(new Event("input"));

    click(r.root, "#ai-save");
    await settle();

    expect((r.core.asked("save_connection")[0] as { connection: Connection }).connection.model).toBe("claude-opus-4-1");
    const models = [...r.root.querySelectorAll<HTMLOptionElement>("#ai-model option")].map((o) => o.value);
    expect(models).toContain("claude-opus-4-1");
  });

  it("reads again and says so when another window changed the connection, and sends nothing more", async () => {
    const r = await ready();
    r.core.refuseSave = new CommandFailure("conflict", "the connection was changed after it was read");

    click(r.root, "#ai-save");
    await settle();

    expect(r.core.asked("set_default_connection")).toEqual([]);
    expect(r.core.asked("list_connections")).toHaveLength(2);
    expect(r.root.textContent).toContain("Another window changed");
  });

  it("is one save at a time: a second press while the first is on its way sends nothing", async () => {
    const r = await ready();
    // The same button twice: one that was on screen when the first press landed.
    const button = r.root.querySelector<HTMLElement>("#ai-save")!;

    button.click();
    button.click();
    await settle();

    expect(r.core.asked("save_connection")).toHaveLength(1);
  });

  it("does not offer a save in a window that may not change the settings", async () => {
    const r = rig({ ...DESKTOP, writes_settings: false });
    await r.settings.load();
    r.settings.openPanel();
    r.draw();

    expect(r.root.querySelector("#ai-save")).toBeNull();
  });
});

describe("which Claude Code a connection runs", () => {
  const withProgram = (executable: string | null): ConnectionListing => ({
    connections: [{ connection: { ...connection("opus"), executable }, revision: REVISION_1 }],
    unreadable: [],
    default: CLAUDE_CONNECTION_ID,
  });

  const opened = async (listing?: ConnectionListing): Promise<Rig> => {
    const r = rig();
    if (listing !== undefined) r.core.listing = listing;
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    return r;
  };

  const programs = (r: Rig): string[] =>
    [...r.root.querySelectorAll<HTMLOptionElement>("#ai-program option")].map((o) => o.value);

  it("is chosen among the programs the application found, and the first choice is to leave it to it", async () => {
    const r = await opened();

    expect(programs(r)).toEqual(["", PROGRAM, OTHER_PROGRAM]);
    expect(r.root.querySelector<HTMLSelectElement>("#ai-program")?.value).toBe("");
    // Each says its version, so that the person can tell one from another.
    expect(r.root.textContent).toContain("2.1.280");
  });

  it("is shown for the connection that names one, even when the application does not find it now", async () => {
    const r = await opened(withProgram("/opt/elsewhere/claude"));

    expect(programs(r)).toEqual(["", PROGRAM, OTHER_PROGRAM, "/opt/elsewhere/claude"]);
    expect(r.root.querySelector<HTMLSelectElement>("#ai-program")?.value).toBe("/opt/elsewhere/claude");
    expect(r.root.textContent).toContain("not found now");
  });

  it("is saved with the connection, as the path of the one chosen", async () => {
    const r = await opened();
    const select = r.root.querySelector<HTMLSelectElement>("#ai-program")!;
    select.value = OTHER_PROGRAM;
    select.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save");
    await settle();

    expect((r.core.asked("save_connection")[0] as { connection: Connection }).connection.executable).toBe(
      OTHER_PROGRAM,
    );
  });

  it("is left to the application again by choosing that, which saves none", async () => {
    const r = await opened(withProgram(PROGRAM));
    const select = r.root.querySelector<HTMLSelectElement>("#ai-program")!;
    select.value = "";
    select.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save");
    await settle();

    expect((r.core.asked("save_connection")[0] as { connection: Connection }).connection.executable).toBeNull();
  });

  it("is kept when only the model is changed: a save does not drop what it was not asked to change", async () => {
    const r = await opened(withProgram(PROGRAM));
    const model = r.root.querySelector<HTMLSelectElement>("#ai-model")!;
    model.value = "sonnet";
    model.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save");
    await settle();

    const saved = (r.core.asked("save_connection")[0] as { connection: Connection }).connection;
    expect(saved.model).toBe("sonnet");
    expect(saved.executable).toBe(PROGRAM);
  });

  it("is asked of again with the person's next check, so that one installed since appears", async () => {
    const r = await opened();
    r.core.found = [...r.core.found, { path: "/new/bin/claude", version: "2.2.0", found: "known-location" }];

    click(r.root, "#ai-recheck");
    await settle();

    expect(programs(r)).toContain("/new/bin/claude");
  });

  it("is not offered when the core would not look: the choice is the automatic one and nothing else", async () => {
    const r = rig();
    r.core.refuseFinding = new CommandFailure("not-allowed-here", "only the desktop application does that");
    await r.settings.load();
    r.settings.openPanel();
    r.draw();

    expect(programs(r)).toEqual([""]);
  });

  it("is said where the client is, with the program the status was asked of", async () => {
    const r = await opened();

    expect(r.root.textContent).toContain(PROGRAM);
  });

  it("can be changed when nobody is signed in to the one that answered, which is when it matters", async () => {
    const r = rig();
    r.core.status = { ...statusOf({ state: "signed-out" }) };
    await r.settings.load();
    r.settings.openPanel();
    r.draw();
    const select = r.root.querySelector<HTMLSelectElement>("#ai-program")!;
    select.value = OTHER_PROGRAM;
    select.dispatchEvent(new Event("change"));

    click(r.root, "#ai-save-program");
    await settle();

    // The same save as the one in the ready state, with the model it already has.
    const saved = (r.core.asked("save_connection")[0] as { connection: Connection }).connection;
    expect(saved.executable).toBe(OTHER_PROGRAM);
    expect(saved.model).toBeNull();
  });

  it("is asked of again after a save, so that what is shown is the program that is now named", async () => {
    const r = await opened();
    const select = r.root.querySelector<HTMLSelectElement>("#ai-program")!;
    select.value = OTHER_PROGRAM;
    select.dispatchEvent(new Event("change"));
    const before = r.core.asked("read_claude_status").length;

    click(r.root, "#ai-save");
    await settle();

    expect(r.core.asked("read_claude_status").length).toBe(before + 1);
  });

  it("is not offered a choice in a window that may not change the settings", async () => {
    const r = rig({ ...DESKTOP, writes_settings: false });
    await r.settings.load();
    r.settings.openPanel();
    r.draw();

    expect(r.root.querySelector("#ai-save")).toBeNull();
    expect(r.root.querySelector("#ai-save-program")).toBeNull();
  });
});

describe("the connection a request is made for", () => {
  it("is none until the settings were read, and the default after", async () => {
    const r = rig();
    expect(r.settings.connectionForRequest()).toBeNull();
    r.core.listing = {
      connections: [{ connection: connection("opus"), revision: REVISION_1 }],
      unreadable: [],
      default: CLAUDE_CONNECTION_ID,
    };

    await r.settings.load();

    expect(r.settings.connectionForRequest()).toEqual({ id: CLAUDE_CONNECTION_ID, revision: REVISION_1 });
  });

  it("is said beside the generate button as what will be asked", async () => {
    const r = rig();
    r.core.listing = {
      connections: [{ connection: connection("opus"), revision: REVISION_1 }],
      unreadable: [],
      default: CLAUDE_CONNECTION_ID,
    };
    await r.settings.load();

    expect(r.settings.targetLine()).toBe("Will ask: Claude Code (opus)");
  });

  it("is said to be none when nothing is chosen, with where to choose it", async () => {
    const r = rig();
    await r.settings.load();

    expect(r.settings.targetLine()).toContain("No AI connection is chosen");
  });

  it("is read again when the core says it moved", async () => {
    const r = rig();
    r.core.listing = {
      connections: [{ connection: connection("opus"), revision: REVISION_1 }],
      unreadable: [],
      default: CLAUDE_CONNECTION_ID,
    };
    await r.settings.load();
    r.core.listing = {
      connections: [{ connection: connection("sonnet"), revision: REVISION_2 }],
      unreadable: [],
      default: CLAUDE_CONNECTION_ID,
    };

    await r.settings.reload();

    expect(r.settings.connectionForRequest()).toEqual({ id: CLAUDE_CONNECTION_ID, revision: REVISION_2 });
  });
});

// ---- Codex ---------------------------------------------------------------------------------

const KEPT_CODEX: ConnectionListing = {
  connections: [{ connection: codexConnection({ model: "gpt-kept", auth: "env-api-key" }), revision: REVISION_1 }],
  unreadable: [],
  default: CODEX_CONNECTION_ID,
};

/** The settings opened, with the kept connections and what the core says of Codex. */
async function opened(over: { listing?: ConnectionListing; codex?: CodexStatus | CommandFailure } = {}): Promise<Rig> {
  const r = rig();
  if (over.listing !== undefined) r.core.listing = over.listing;
  if (over.codex !== undefined) r.core.codexStatus = over.codex;
  await r.settings.load();
  r.settings.openPanel();
  r.draw();
  return r;
}

/** The person chooses Codex among the clients. */
async function chooseCodex(r: Rig): Promise<void> {
  click(r.root, "#ai-kind-codex");
  await settle();
}

const typeInto = (root: HTMLElement, selector: string, text: string): void => {
  const field = root.querySelector<HTMLInputElement>(selector);
  if (field === null) throw new Error(`no ${selector} on screen`);
  field.value = text;
  field.dispatchEvent(new Event("input"));
};

const choose = (root: HTMLElement, selector: string, value: string): void => {
  const select = root.querySelector<HTMLSelectElement>(selector);
  if (select === null) throw new Error(`no ${selector} on screen`);
  select.value = value;
  select.dispatchEvent(new Event("change"));
};

describe("choosing which client the settings are for", () => {
  it("starts on Claude Code when no connection is the default, and asks only Claude Code", async () => {
    const r = await opened();

    expect(r.core.calls.map((c) => c.name)).toEqual(["list_connections", "read_claude_status", "find_clients"]);
    expect(r.root.querySelector<HTMLInputElement>("#ai-kind-claude-code")?.checked).toBe(true);
    expect(r.root.querySelector<HTMLInputElement>("#ai-kind-codex")?.checked).toBe(false);
  });

  it("starts on Codex when the default connection is for Codex, and asks Codex and not Claude Code", async () => {
    const r = await opened({ listing: KEPT_CODEX });

    expect(r.core.calls.map((c) => c.name)).toEqual(["list_connections", "read_codex_status", "find_clients"]);
    expect(r.root.querySelector<HTMLInputElement>("#ai-kind-codex")?.checked).toBe(true);
  });

  it("asks Codex the first time it is chosen, and does not ask Claude Code again when it is chosen back", async () => {
    const r = await opened();

    await chooseCodex(r);
    click(r.root, "#ai-kind-claude-code");
    await settle();

    expect(r.core.asked("read_codex_status")).toHaveLength(1);
    expect(r.core.asked("read_claude_status")).toHaveLength(1);
    expect(r.root.querySelector("#ai-codex-source")).toBeNull();
  });

  it("keeps what was chosen for one client while the other is looked at", async () => {
    const r = await opened();
    choose(r.root, "#ai-model", "sonnet");

    await chooseCodex(r);
    typeInto(r.root, "#ai-codex-model", "gpt-x");
    click(r.root, "#ai-kind-claude-code");
    await settle();

    expect(r.root.querySelector<HTMLSelectElement>("#ai-model")?.value).toBe("sonnet");
    await chooseCodex(r);
    expect(r.root.querySelector<HTMLInputElement>("#ai-codex-model")?.value).toBe("gpt-x");
  });

  it("offers no choice of client in a window that may not start a program", async () => {
    const r = rig(BROWSER);
    await r.settings.load();
    r.settings.openPanel();
    r.draw();

    expect(r.root.querySelector("#ai-kind-codex")).toBeNull();
    expect(r.root.textContent).toContain("desktop application");
  });
});

describe("what is shown for each state Codex can be in", () => {
  const shown = async (codex: CodexStatus | CommandFailure): Promise<Rig> => {
    const r = await opened({ codex });
    await chooseCodex(r);
    return r;
  };

  it("says to install it when there is none, and where to say where it is", async () => {
    const r = await shown(codexStatusOf({ client: { state: "missing" } }));

    expect(r.root.textContent).toContain("Codex is not installed");
    expect(r.root.textContent).toContain("SCE_CODEX");
    expect(r.root.querySelector("code")).toBeNull();
  });

  it("says a file that does not say it is Codex is not used", async () => {
    const r = await shown(codexStatusOf({ client: { state: "unverified" } }));

    expect(r.root.textContent).toContain("did not say it is Codex");
  });

  it("says a failure to ask in the core's words, and does not send anyone to sign in", async () => {
    const r = await shown(new CommandFailure("failed", "Codex could not be started"));

    expect(r.root.textContent).toContain("Codex could not be started");
    expect(r.root.querySelector("code")).toBeNull();
  });

  it("says a version this build verified, a login, and that a request will run", async () => {
    const r = await shown(codexStatusOf());

    expect(r.root.textContent).toContain("has verified this version");
    expect(r.root.textContent).toContain("will run");
    expect(r.root.textContent).not.toContain("will wait");
    expect(r.root.querySelector("#ai-save")).not.toBeNull();
    const sources = [...r.root.querySelectorAll<HTMLOptionElement>("#ai-codex-source option")].map((o) => o.value);
    expect(sources).toEqual(["official-login", "app-store", "env-api-key"]);
  });

  it("says a version this build did not verify, whoever is signed in, and that a request will wait", async () => {
    const r = await shown(
      codexStatusOf({ support: { state: "unverified", reason: "this version has not been verified" } }),
    );

    expect(r.root.textContent).toContain("has not verified this version");
    expect(r.root.textContent).toContain("this version has not been verified");
    expect(r.root.textContent).toContain("will wait");
    // Not offered as the default: every generation would wait. Another program can still be chosen,
    // which is how a person reaches a version this build did verify.
    expect(r.root.querySelector("#ai-save")).toBeNull();
    expect(r.root.querySelector("#ai-codex-model")).toBeNull();
    expect(r.root.querySelector("#ai-save-program")).not.toBeNull();
    expect(r.root.querySelector("#ai-codex-program")).not.toBeNull();
  });

  it("says a version that could not be checked is not one that was refused", async () => {
    const r = await shown(codexStatusOf({ support: { state: "unknown", reason: "it did not list its features" } }));

    expect(r.root.textContent).toContain("it did not list its features");
    expect(r.root.textContent).not.toContain("has not verified this version");
  });

  it("gives the commands of the source that is chosen, and the folder the application's own login is made in", async () => {
    const r = await shown(
      codexStatusOf({}, [codexSignedIn(), { state: "signed-out" }, { state: "signed-out" }]),
    );

    choose(r.root, "#ai-codex-source", "app-store");
    await settle();
    r.draw();

    const commands = [...r.root.querySelectorAll("code")].map((c) => c.textContent);
    expect(commands).toEqual([
      "codex login -c cli_auth_credentials_store=file",
      "codex login --with-api-key -c cli_auth_credentials_store=file",
    ]);
    expect(r.root.textContent).toContain("Nobody is signed in by this source");
    expect(r.root.textContent).toContain(CODEX_HOME);
  });

  it("gives the official client's commands without a folder for its own login", async () => {
    const r = await shown(codexStatusOf({}, [{ state: "signed-out" }, { state: "signed-out" }, { state: "signed-out" }]));

    expect(r.root.textContent).not.toContain(CODEX_HOME);
    expect([...r.root.querySelectorAll("code")]).toHaveLength(2);
  });

  it("says a key in the environment is named by its variable, and has no command to sign in with", async () => {
    const r = await shown(codexStatusOf({}, [{ state: "signed-out" }, { state: "signed-out" }, { state: "signed-out" }]));

    choose(r.root, "#ai-codex-source", "env-api-key");
    await settle();
    r.draw();

    expect(r.root.textContent).toContain("CODEX_API_KEY is not set");
    expect(r.root.querySelector("code")).toBeNull();
  });

  it("names the variable of a key that is set, and says it is billed by use", async () => {
    const key = codexSignedIn({ route: "codex-cli-api-key", billing: "usage", environment: "CODEX_API_KEY" });
    const r = await shown(codexStatusOf({}, [{ state: "signed-out" }, { state: "signed-out" }, key]));

    choose(r.root, "#ai-codex-source", "env-api-key");
    await settle();
    r.draw();

    expect(r.root.textContent).toContain("CODEX_API_KEY");
    expect(r.root.textContent).toContain("billed by use");
  });

  it("says a login the build does not use is a login that is there, and gives the commands that sign in another way", async () => {
    const unused = codexSignedIn({
      route: "unlisted",
      billing: null,
      usable: false,
      decision: { decision: "refuse", status: "unconfirmed", reason: "unconfirmed" },
    });
    const r = await shown(codexStatusOf({}, [unused, { state: "signed-out" }, { state: "signed-out" }]));

    expect(r.root.textContent).toContain("does not use");
    expect(r.root.textContent).toContain("will wait");
    expect([...r.root.querySelectorAll("code")]).toHaveLength(2);
  });

  it("says who is signed in by a source could not be asked, and does not send anyone to sign in for it", async () => {
    const r = await shown(
      codexStatusOf({}, [{ state: "unknown", reason: "it did not answer" }, { state: "signed-out" }, { state: "signed-out" }]),
    );

    expect(r.root.textContent).toContain("it did not answer");
    expect(r.root.querySelector("code")).toBeNull();
  });
});

describe("saving a connection to Codex", () => {
  const readyToSave = async (over: Parameters<typeof opened>[0] = {}): Promise<Rig> => {
    const r = await opened(over);
    if (over.listing === undefined) await chooseCodex(r);
    return r;
  };

  it("saves a first connection on top of nothing and makes it the default, expecting none", async () => {
    const r = await readyToSave();

    click(r.root, "#ai-save");
    await settle();

    expect(r.core.asked("save_connection")).toEqual([
      {
        connection: {
          id: CODEX_CONNECTION_ID,
          adapter: "codex",
          display_name: null,
          executable: null,
          model: null,
          auth: "official-login",
          server_url: null,
          limits: { turns: null, seconds: null },
        },
        base: null,
      },
    ]);
    expect(r.core.asked("set_default_connection")).toEqual([{ id: CODEX_CONNECTION_ID, expect: null }]);
    expect(r.root.textContent).toContain("from the next generation");
    expect(r.settings.connectionForRequest()).toEqual({ id: CODEX_CONNECTION_ID, revision: REVISION_2 });
    expect(r.settings.targetLine()).toBe("Will ask: Codex (client default)");
    // What is shown is of what is kept now: Codex is asked again.
    expect(r.core.asked("read_codex_status")).toHaveLength(2);
  });

  it("saves the model that was typed, the source and the program that were chosen", async () => {
    // Somebody is signed in by the application's own login too, or choosing it would leave nothing to save.
    const r = await readyToSave({
      codex: codexStatusOf({}, [codexSignedIn(), codexSignedIn(), { state: "signed-out" }]),
    });
    typeInto(r.root, "#ai-codex-model", "  gpt-x  ");
    choose(r.root, "#ai-codex-source", "app-store");
    choose(r.root, "#ai-codex-program", OTHER_CODEX);

    click(r.root, "#ai-save");
    await settle();

    const saved = r.core.asked("save_connection")[0] as { connection: Connection };
    expect(saved.connection).toMatchObject({
      adapter: "codex",
      model: "gpt-x",
      auth: "app-store",
      executable: OTHER_CODEX,
    });
  });

  it("writes on top of the revision it read, keeps the limits it had, and expects the default it saw", async () => {
    const key = codexSignedIn({ route: "codex-cli-api-key", billing: "usage", environment: "CODEX_API_KEY" });
    const r = await readyToSave({
      listing: KEPT_CODEX,
      codex: codexStatusOf({}, [{ state: "signed-out" }, { state: "signed-out" }, key]),
    });

    // What is kept is what is shown: its model, and the source it names.
    expect(r.root.querySelector<HTMLInputElement>("#ai-codex-model")?.value).toBe("gpt-kept");
    expect(r.root.querySelector<HTMLSelectElement>("#ai-codex-source")?.value).toBe("env-api-key");
    click(r.root, "#ai-save");
    await settle();

    expect(r.core.asked("save_connection")).toEqual([
      {
        connection: codexConnection({ model: "gpt-kept", auth: "env-api-key" }),
        base: REVISION_1,
      },
    ]);
    expect(r.core.asked("set_default_connection")).toEqual([{ id: CODEX_CONNECTION_ID, expect: CODEX_CONNECTION_ID }]);
  });

  it("saves a blank model as the client's own, and not as the one that was kept", async () => {
    const key = codexSignedIn({ route: "codex-cli-api-key", billing: "usage", environment: "CODEX_API_KEY" });
    const r = await readyToSave({
      listing: KEPT_CODEX,
      codex: codexStatusOf({}, [{ state: "signed-out" }, { state: "signed-out" }, key]),
    });

    typeInto(r.root, "#ai-codex-model", "   ");
    click(r.root, "#ai-save");
    await settle();

    const saved = r.core.asked("save_connection")[0] as { connection: Connection };
    expect(saved.connection.model).toBeNull();
  });

  it("does not drop the connection to Claude Code that is kept beside it", async () => {
    const r = await readyToSave({
      listing: {
        connections: [{ connection: connection("opus"), revision: REVISION_1 }],
        unreadable: [],
        default: CLAUDE_CONNECTION_ID,
      },
    });
    await chooseCodex(r);

    click(r.root, "#ai-save");
    await settle();

    expect(r.core.listing.connections.map((c) => c.connection.id).sort()).toEqual([
      CLAUDE_CONNECTION_ID,
      CODEX_CONNECTION_ID,
    ]);
    // The one the person last saved is the one a request is made for.
    expect(r.core.asked("set_default_connection")).toEqual([
      { id: CODEX_CONNECTION_ID, expect: CLAUDE_CONNECTION_ID },
    ]);
  });

  it("says another window changed it first, and overwrites nothing of theirs", async () => {
    const r = await readyToSave();
    r.core.refuseSave = new CommandFailure("conflict", "changed");

    click(r.root, "#ai-save");
    await settle();

    expect(r.root.textContent).toContain("Another window changed this connection first");
    expect(r.core.asked("set_default_connection")).toEqual([]);
  });
});
