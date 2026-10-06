// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen's guards against the replies the core actually gives.
//
// `app-core/contract/replies.json` is produced by running the core's commands
// (`app-core/tests/contract.rs` fails if it is stale). Here it is read back and
// held against the guards, so the screen cannot expect a shape the core does not
// produce, and the core cannot change a shape without this test seeing it.

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  asCommandError,
  conflictRevisions,
  ContractError,
  isOpenRequest,
  movedInRefusal,
  parseAdapterListing,
  parseAdapterReport,
  parseBundleRead,
  parseAuthPolicy,
  parseClaudeStatus,
  parseCodexStatus,
  parseCompletedRequest,
  parseConnectionListing,
  parseDefaultConnection,
  parseDeletedConnection,
  parseDescribed,
  parseGenerationRequest,
  parseHostListing,
  parseRequestCandidateTexts,
  parseRegisteredRequest,
  parseRequestList,
  parseRequestReply,
  parseFigures,
  parseFindClients,
  parseHistory,
  parseJudgment,
  parseListing,
  parseReadAcceptance,
  parseReadAnswers,
  parseReadConnection,
  parseReadModel,
  parseReadRequirements,
  parseReadSource,
  parseRemoved,
  parseRequirementsReport,
  parseReview,
  parseSaved,
  parseServerStatus,
  parseWork,
  parseWorkAndHead,
  parseWorkHeads,
  parseWorkSnapshot,
  SUPPORTED_COMMAND_SET_VERSION,
} from "../src/contract";

interface Replies {
  command_set_version: number;
  answers: Record<string, unknown>;
  refusals: Record<string, unknown>;
}

const replies = JSON.parse(
  readFileSync(new URL("../../../app-core/contract/replies.json", import.meta.url), "utf8"),
) as Replies;

const parsers: Record<string, (value: unknown) => unknown> = {
  describe: parseDescribed,
  list_works: parseListing,
  create_work: (v) => parseWork(v),
  read_work: parseWorkAndHead,
  read_source: parseReadSource,
  save_source: parseSaved,
  history: parseHistory,
  save_model: parseSaved,
  read_model: parseReadModel,
  model_history: parseHistory,
  figures: parseFigures,
  remove_work: parseRemoved,
  review: parseReview,
  read_answers: parseReadAnswers,
  save_answers: parseSaved,
  save_requirements: parseSaved,
  read_requirements: parseReadRequirements,
  requirements_report: parseRequirementsReport,
  accept: parseSaved,
  read_acceptance: parseReadAcceptance,
  read_work_snapshot: parseWorkSnapshot,
  read_judgment: parseJudgment,
  read_work_heads: parseWorkHeads,
  request_generation: parseRegisteredRequest,
  read_request: parseRequestReply,
  list_requests: parseRequestList,
  list_open_requests: parseRequestList,
  claim_request: parseRequestReply,
  heartbeat_request: parseRequestReply,
  save_request_candidate: parseRequestReply,
  read_request_candidate: parseRequestCandidateTexts,
  complete_request: parseCompletedRequest,
  fail_request: parseRequestReply,
  cancel_request: parseRequestReply,
  report_adapter: parseAdapterReport,
  read_adapter_status: parseAdapterListing,
  read_host_status: parseHostListing,
  read_bundle: parseBundleRead,
  bundle_history: parseHistory,
  list_connections: parseConnectionListing,
  read_connection: parseReadConnection,
  read_auth_policy: parseAuthPolicy,
  read_claude_status: parseClaudeStatus,
  read_codex_status: parseCodexStatus,
  find_clients: parseFindClients,
  read_server_status: parseServerStatus,
  save_connection: parseSaved,
  delete_connection: parseDeletedConnection,
  set_default_connection: parseDefaultConnection,
};

/** The command an answer's name belongs to: the longest command name it starts with. */
function commandOf(name: string): string {
  const matches = Object.keys(parsers).filter((c) => name === c || name.startsWith(`${c}_`));
  const longest = matches.sort((a, b) => b.length - a.length)[0];
  if (longest === undefined) throw new Error(`${name} belongs to no command the screen knows`);
  return longest;
}

describe("the replies the core gives", () => {
  it("are of the command set this screen was written for", () => {
    expect(replies.command_set_version).toBe(SUPPORTED_COMMAND_SET_VERSION);
  });

  it("are all accepted by the guard of their command", () => {
    for (const [name, reply] of Object.entries(replies.answers)) {
      const parse = parsers[commandOf(name)];
      expect(parse, name).toBeDefined();
      expect(() => parse?.(reply), name).not.toThrow();
    }
  });

  it("cover every command the core lists", () => {
    const described = parseDescribed(replies.answers["describe"]);
    for (const command of described.commands) {
      expect(parsers[command], `the screen has no guard for \`${command}\``).toBeDefined();
      const covered = Object.keys(replies.answers).some((name) => commandOf(name) === command);
      expect(covered, `no reply of \`${command}\` is written down`).toBe(true);
    }
  });

  it("carry the facts the screen branches on", () => {
    expect(parseReadSource(replies.answers["read_source_none"])).toBeNull();
    expect(parseReadSource(replies.answers["read_source"])?.text).toContain("Three misses");
    expect(parseWorkAndHead(replies.answers["read_work_no_text"]).head).toBeNull();
    expect(parseSaved(replies.answers["save_source_first"])).toMatchObject({ outcome: "saved", parent: null });
    expect(parseSaved(replies.answers["save_source_unchanged"]).outcome).toBe("unchanged");
    expect(parseHistory(replies.answers["history"])).toHaveLength(2);
  });

  it("carry what the screen shows of a model: where it stands, and the sheets in SCE's order", () => {
    expect(parseReadModel(replies.answers["read_model_none"])).toMatchObject({ model: null, standing: null });
    // The same model, for the text before and after it moved on.
    expect(parseReadModel(replies.answers["read_model"]).standing).toBe("behind");
    expect(parseReadModel(replies.answers["read_model_kept"]).standing).toBe("current");
    const model = parseReadModel(replies.answers["read_model"]);
    expect(model.model?.written_for).not.toBe(model.source_head);
    const drawn = parseFigures(replies.answers["figures"]);
    expect(drawn.sheets.map((s) => s.name)).toEqual(["picture.svg", "fields-1.svg"]);
    expect(drawn.sheets[0]?.svg.startsWith("<svg")).toBe(true);
    expect(drawn.generator).toBe("fake-sce 0");
  });

  it("carry a model as its documents: one under the name model.scxml, or several with the entry first", () => {
    const one = parseReadModel(replies.answers["read_model"]).model;
    expect(one?.entry).toBe("model.scxml");
    expect(one?.documents).toEqual([{ name: "model.scxml", text: one?.text }]);

    const several = parseReadModel(replies.answers["read_model_set"]).model;
    expect(several?.entry).toBe("door.scxml");
    expect(several?.documents.map((d) => d.name)).toEqual(["door.scxml", "open.scxml"]);
    expect(several?.text).toBe(several?.documents[0]?.text);
    expect(parseSaved(replies.answers["save_model_set"]).outcome).toBe("saved");
    expect(asCommandError(replies.refusals["invalid-model"])?.kind).toBe("invalid-model");
    expect(asCommandError(replies.refusals["invalid-model"])?.message).toContain("file name an import can name");
  });

  it("carry what the screen shows of SCE's review: the verdict, what is left open, the page or why not", () => {
    const accepted = parseReview(replies.answers["review"]);
    expect(accepted.check).toMatchObject({ verdict: "accepted", kind: "statechart", records: [] });
    expect(accepted.check.open).toHaveLength(1);
    expect(accepted.check.unresolved[0]).toEqual({
      id: "open-guard",
      node_path: "states.closed.transitions[0]",
      line: 3,
      reason: "Which card values open the door?",
    });
    expect(accepted.page).toContain("machine");
    expect(accepted.page_refusal).toBeNull();
    // The same standing the figures and the model read carry: one word, from the core.
    expect(accepted.standing).toBe("behind");

    const refused = parseReview(replies.answers["review_refused"]);
    expect(refused.check.verdict).toBe("refused");
    expect(refused.check.kind).toBeNull();
    expect(refused.page).toBeNull();
    expect(refused.check.records[0]).toMatchObject({ code: "validation/invalid-reference", line: 3 });

    const noPage = parseReview(replies.answers["review_no_page"]);
    expect(noPage.check.verdict).toBe("accepted");
    expect(noPage.page).toBeNull();
    expect(noPage.page_refusal?.code).toBe("cli/pseudo-unsupported");
  });

  it("carry the owner's answers by question, each with its words and when they changed", () => {
    expect(parseReadAnswers(replies.answers["read_answers_none"])).toBeNull();
    const held = parseReadAnswers(replies.answers["read_answers"]);
    expect(held?.revision).toMatch(/^[0-9a-f]{64}$/);
    expect(held?.entries["open-guard"]?.answer).toBe("Any card on the list opens it.");
    expect(held?.entries["open-guard"]?.answered_at).toBe("2026-10-03T09:00:00Z");
    expect(parseSaved(replies.answers["save_answers_first"]).outcome).toBe("saved");
    expect(parseSaved(replies.answers["save_answers_unchanged"]).outcome).toBe("unchanged");
    expect(asCommandError(replies.refusals["invalid-answers"])?.message).toContain("is empty");
    // The question's own words are what the screen shows the owner.
    const review = parseReview(replies.answers["review"]);
    expect(review.check.unresolved[0]?.reason).toBe("Which card values open the door?");
  });

  it("carry the requirement list, SCE's measure of the design against it, and what the owner accepted", () => {
    expect(parseReadRequirements(replies.answers["read_requirements_none"])).toMatchObject({
      requirements: null,
      standing: null,
    });
    const held = parseReadRequirements(replies.answers["read_requirements"]);
    expect(held.standing).toBe("current");
    // The two files come back as the authoring client wrote them: an acceptance pins their bytes.
    expect(held.requirements?.manifest).toContain('"requirements":[{"id":"R1"},{"id":"R2"}]');
    expect(held.requirements?.sidecar).toContain('"R1":"The door opens."');
    expect(parseSaved(replies.answers["save_requirements_first"]).outcome).toBe("saved");
    expect(parseSaved(replies.answers["save_requirements_unchanged"]).outcome).toBe("unchanged");

    const report = parseRequirementsReport(replies.answers["requirements_report"]);
    expect(report.outcomes.map((o) => [o.id, o.outcome])).toEqual([
      ["R1", "implemented"],
      ["R2", "implemented"],
    ]);
    expect(report).toMatchObject({ model_standing: "current", requirements_standing: "current", denominator: "synthesized" });
    expect(report.basis.answers).toBeNull();
    expect(report.page).toContain("ACCEPTANCE REPORT");
    const behind = parseRequirementsReport(replies.answers["requirements_report_behind"]);
    expect(behind).toMatchObject({ model_standing: "behind", requirements_standing: "behind" });

    expect(parseSaved(replies.answers["accept"]).outcome).toBe("saved");
    const none = parseReadAcceptance(replies.answers["read_acceptance_none"]);
    expect(none).toMatchObject({ acceptance: null, standing: "none", lapse: null, now: null });
    // What the owner accepted is what they were shown: the same revisions, stated here.
    const accepted = parseReadAcceptance(replies.answers["read_acceptance"]);
    expect(accepted).toMatchObject({ standing: "holds", lapse: null });
    expect(accepted.acceptance?.channel).toBe("direct");
    expect(accepted.acceptance?.basis).toEqual(report.basis);
    expect(accepted.acceptance?.open).toEqual(["1 question(s) the specification leaves open (open-guard)"]);
    const lapsed = parseReadAcceptance(replies.answers["read_acceptance_lapsed"]);
    expect(lapsed).toMatchObject({ standing: "lapsed", lapse: "design/model.scxml moved" });
    expect(lapsed.now?.model).not.toBe(lapsed.acceptance?.basis.model);
  });

  it("give a work as one state, in the words of the commands that read one chain", () => {
    const empty = parseWorkSnapshot(replies.answers["read_work_snapshot_empty"]);
    expect(empty).toMatchObject({
      source: null,
      model: null,
      model_standing: null,
      answers: null,
      requirements: null,
      requirements_standing: null,
      acceptance: null,
    });
    // A work with a text, a model and answers, and no list and no acceptance.
    const door = parseWorkSnapshot(replies.answers["read_work_snapshot"]);
    expect(door.work.title).toBe("Door lock");
    expect(door.model_standing).toBe("current");
    expect(door.model).toEqual(parseReadModel(replies.answers["read_model_kept"]).model);
    expect(door.answers).toEqual(parseReadAnswers(replies.answers["read_answers"]));
    expect(door.requirements).toBeNull();
    expect(door.acceptance).toBeNull();
    // A work with a text, a model, a list and an acceptance, and no answers.
    const garage = parseWorkSnapshot(replies.answers["read_work_snapshot_accepted"]);
    expect(garage.answers).toBeNull();
    expect(garage.requirements_standing).toBe("current");
    expect(garage.requirements).toEqual(parseReadRequirements(replies.answers["read_requirements"]).requirements);
    expect(garage.acceptance).toEqual(parseReadAcceptance(replies.answers["read_acceptance"]).acceptance);
  });

  it("give what SCE says of the revisions it was asked about, in the words of the commands that judge", () => {
    // A design measured and nothing accepted: no standing, and the measure of `requirements_report`
    // in the same words, without where the design stands to the text (a claim, which is the
    // snapshot's to say and which a later save can change without changing the design).
    const unaccepted = parseJudgment(replies.answers["read_judgment_unaccepted"]);
    expect(unaccepted.acceptance).toBeNull();
    const { basis, source_head, model_standing, requirements_standing, ...measure } = parseRequirementsReport(
      replies.answers["requirements_report"],
    );
    expect(unaccepted.report).toEqual({ said: true, value: measure });
    expect(unaccepted.basis).toEqual(basis);
    expect([source_head, model_standing, requirements_standing].every((v) => v !== undefined)).toBe(true);
    const said = replies.answers["read_judgment_unaccepted"] as { report: { said: Record<string, unknown> } };
    for (const claim of ["basis", "source_head", "model_standing", "requirements_standing"]) {
      expect(said.report.said[claim], claim).toBeUndefined();
    }
    // Accepted and unmoved: the standing of `read_acceptance`, the revisions the owner was shown.
    const accepted = parseJudgment(replies.answers["read_judgment_accepted"]);
    expect(accepted.acceptance).toEqual({ said: true, value: { standing: "holds", lapse: null } });
    expect(accepted.report).toEqual(unaccepted.report);
    expect(accepted.basis).toEqual(parseReadAcceptance(replies.answers["read_acceptance"]).acceptance?.basis);
    // The design moved under the acceptance. Asked of the design as it stands it lapsed, in SCE's
    // sentence, and the measure names that design; asked of the one the owner was shown it holds.
    const lapsed = parseJudgment(replies.answers["read_judgment_lapsed"]);
    expect(lapsed.acceptance).toEqual({
      said: true,
      value: { standing: "lapsed", lapse: "design/model.scxml moved" },
    });
    expect(lapsed.basis).toEqual(parseReadAcceptance(replies.answers["read_acceptance_lapsed"]).now);
    expect(lapsed.basis.model).not.toBe(accepted.basis.model);
    const earlier = parseJudgment(replies.answers["read_judgment_of_an_earlier_state"]);
    expect(earlier).toEqual(accepted);
    // SCE not answering is an answer, and says which it did not answer.
    const unanswered = parseJudgment(replies.answers["read_judgment_unanswered"]);
    expect(unanswered.basis).toEqual(accepted.basis);
    for (const part of [unanswered.acceptance, unanswered.report]) {
      expect(part).toMatchObject({ said: false, refusal: { kind: "sce-timeout", code: null } });
    }
  });

  it("refuse a judgment that contradicts itself", () => {
    const answer = (name: string): Record<string, unknown> =>
      structuredClone(replies.answers[name]) as Record<string, unknown>;
    const at = (value: unknown, ...path: string[]): Record<string, unknown> =>
      path.reduce((over, key) => over[key] as Record<string, unknown>, value as Record<string, unknown>);

    // A measure whose outcomes are not a list.
    const report = answer("read_judgment_accepted");
    at(report, "report", "said")["outcomes"] = "none";
    expect(() => parseJudgment(report)).toThrow(/read_judgment\.report\.said\.outcomes/);
    // An answer that is both, or neither, said and refused.
    const both = answer("read_judgment_accepted");
    at(both, "report")["refused"] = { kind: "sce-timeout", message: "m", code: null };
    expect(() => parseJudgment(both)).toThrow(/exactly one of "said" and "refused"/);
    const neither = answer("read_judgment_accepted");
    neither["report"] = {};
    expect(() => parseJudgment(neither)).toThrow(/exactly one of "said" and "refused"/);
    // A standing that lapsed without a sentence for what moved, and one that holds with a sentence.
    const mute = answer("read_judgment_lapsed");
    at(mute, "acceptance", "said")["lapse"] = null;
    expect(() => parseJudgment(mute)).toThrow(/lapse/);
    const stray = answer("read_judgment_accepted");
    at(stray, "acceptance", "said")["lapse"] = "something moved";
    expect(() => parseJudgment(stray)).toThrow(/lapse/);
    // A standing that is neither holds nor lapsed.
    const unknown = answer("read_judgment_accepted");
    at(unknown, "acceptance", "said")["standing"] = "none";
    expect(() => parseJudgment(unknown)).toThrow(/standing/);
  });

  it("give where each chain of a work stands, with what the model and the list were written for", () => {
    expect(parseWorkHeads(replies.answers["read_work_heads_empty"])).toEqual({
      source: null,
      model: null,
      answers: null,
      requirements: null,
      acceptance: null,
      bundle: null,
      request: null,
    });
    const door = parseWorkHeads(replies.answers["read_work_heads"]);
    const snapshot = parseWorkSnapshot(replies.answers["read_work_snapshot"]);
    expect(door.source).toBe(snapshot.source?.revision);
    expect(door.model).toEqual({ revision: snapshot.model?.revision, written_for: snapshot.model?.written_for });
    expect(door.answers).toBe(snapshot.answers?.revision);
    expect(door.requirements).toBeNull();
    const garage = parseWorkHeads(replies.answers["read_work_heads_accepted"]);
    const held = parseWorkSnapshot(replies.answers["read_work_snapshot_accepted"]);
    expect(garage.requirements).toEqual({
      revision: held.requirements?.revision,
      written_for: held.requirements?.written_for,
    });
    expect(garage.acceptance).toBe(held.acceptance?.revision);
  });

  it("give a request in the words a screen branches on, and say what the clock reads against what was written", () => {
    const made = parseRegisteredRequest(replies.answers["request_generation"]);
    expect(made.created).toBe(true);
    expect(made.request).toMatchObject({ state: "queued", stored_state: "queued", attempt: 0, origin: "gui", seq: 1 });
    expect(made.request.inputs.answers).toMatch(/^[0-9a-f]{64}$/);
    expect(made.request.lease).toBeNull();
    // The same press sent again is the request it already made.
    const again = parseRegisteredRequest(replies.answers["request_generation_again"]);
    expect(again).toMatchObject({ created: false });
    expect(again.request.id).toBe(made.request.id);

    const taken = parseRequestReply(replies.answers["claim_request"]);
    expect(taken).toMatchObject({ state: "running", attempt: 1 });
    expect(taken.lease).toMatchObject({ holder: "adapter-a", attempt: 1 });
    expect(parseRequestReply(replies.answers["heartbeat_request"]).lease?.expires_at).toBe("2026-10-03T09:02:00Z");
    expect(parseCompletedRequest(replies.answers["complete_request"]).request).toMatchObject({ state: "completed" });
    expect(parseRequestReply(replies.answers["fail_request"])).toMatchObject({
      state: "failed",
      note: "SCE refused the model",
    });
    expect(parseRequestReply(replies.answers["cancel_request"]).state).toBe("cancelled");
    // What an executor looks at to find work: the requests that can still produce a result.
    const open = parseRequestList(replies.answers["list_open_requests"]);
    expect(open.map((r) => [r.id, r.state])).toEqual([["<request-id>", "queued"]]);
    // Newest first, and one request of a work is open at most.
    const listed = parseRequestList(replies.answers["list_requests"]);
    expect(listed.map((r) => r.seq)).toEqual([...listed.map((r) => r.seq)].sort((a, b) => b - a));
    expect(listed.filter((r) => isOpenRequest(r.state))).toHaveLength(0);
  });

  it("say whether a shell hosts an executor and, when it does not, why", () => {
    expect(parseHostListing(replies.answers["read_host_status_none"]).hosts).toEqual([]);
    const listing = parseHostListing(replies.answers["read_host_status"]);
    expect(listing.hosts).toHaveLength(1);
    expect(listing.hosts[0]).toMatchObject({ name: "desktop", hosting: false, client_version: null, live: true });
    expect(listing.hosts[0]?.reason).toContain("SCE_CLAUDE");
  });

  it("say what an executor wrote for a request, and that it is not the work's until it completes", () => {
    const written = parseRequestReply(replies.answers["save_request_candidate"]);
    expect(written.candidate?.model).toMatch(/^[0-9a-f]{64}$/);
    expect(written.candidate?.requirements).toMatch(/^[0-9a-f]{64}$/);
    expect(written.outcome).toBeNull();
    const texts = parseRequestCandidateTexts(replies.answers["read_request_candidate"]);
    expect(texts.model?.revision).toBe(written.candidate?.model);
    expect(texts.model?.text).toContain("candidate");
    const refused = replies.refusals["no-candidate"] as { kind: string; detail: { missing: string[] } };
    expect(refused.kind).toBe("no-candidate");
    expect(refused.detail.missing).toEqual(["model", "requirements"]);
  });

  it("say which bundle a completed request made the work's, and what was checked of it", () => {
    const done = parseCompletedRequest(replies.answers["complete_request"]);
    // The contract file names a bundle's revision, which varies with the request that made it.
    expect(done.bundle).toBe("b".repeat(64));
    expect(done.request.outcome).toEqual({ bundle: done.bundle });
    const read = parseBundleRead(replies.answers["read_bundle"]);
    expect(read?.revision).toBe(done.bundle);
    expect(read?.bundle).toMatchObject({ executor: "adapter-a", attempt: 1, request: "<request-id>" });
    // The core's own check comes first and says which model it ran on; the executor's report is kept as reported.
    expect(read?.bundle.checks.map((c) => [c.by, c.name, c.verdict])).toEqual([
      ["core", "model", "accepted"],
      ["client", "decisions", "accepted"],
    ]);
    expect(read?.bundle.checks[0]?.subject).toBe(read?.bundle.model);
    // What the executor said it worked to is on the bundle, and a first bundle replaced none.
    expect(read?.bundle.instructions).toBe("claude-code/0123456789ab");
    expect(read?.bundle.replaces).toBeNull();
    expect(parseRequestReply(replies.answers["save_request_candidate"]).candidate?.instructions).toBe(
      "claude-code/0123456789ab",
    );
    // The work had a model before the first bundle, and the bundle says what it took over from.
    expect(read?.bundle.previous?.model).toMatch(/^[0-9a-f]{64}$/);
    expect(parseHistory(replies.answers["bundle_history"]).map((h) => h.revision)).toEqual([done.bundle]);
    expect(parseWorkHeads(replies.answers["read_work_heads_bundled"]).bundle).toBe(done.bundle);
    expect(parseWorkHeads(replies.answers["read_work_heads_bundled"]).model?.revision).toBe(read?.bundle.model);
    expect((replies.refusals["bundled-work"] as { kind: string }).kind).toBe("bundled-work");
    expect((replies.refusals["check-refused"] as { detail: { checks: string[] } }).detail.checks).toEqual(["model"]);
  });

  it("say where the latest request of a work stands in its heads", () => {
    expect(parseWorkHeads(replies.answers["read_work_heads"]).request).toBeNull();
    const heads = parseWorkHeads(replies.answers["read_work_heads_requested"]);
    expect(heads.request).toMatchObject({ state: "running", attempt: 1 });
    expect(heads.request?.id).toBe("<request-id>");
  });

  it("say which AI adapters are there and what each can do", () => {
    expect(parseAdapterListing(replies.answers["read_adapter_status_none"])).toEqual({ adapters: [], unreadable: [] });
    const reported = parseAdapterReport(replies.answers["report_adapter"]);
    expect(reported).toMatchObject({ name: "desktop", kind: "claude-code", live: true, version: "2.1" });
    expect(reported.capabilities).toEqual(["generate", "cancel"]);
    expect(parseAdapterListing(replies.answers["read_adapter_status"]).adapters).toEqual([reported]);
  });

  it("refuse a request that cannot be made or kept, and say who holds it or what is open", () => {
    expect(asCommandError(replies.refusals["active-request"])?.detail).toMatchObject({ state: "queued" });
    const held = asCommandError(replies.refusals["request-held"]);
    expect(held?.detail).toMatchObject({ holder: "adapter-a", until: "2026-10-03T09:01:00Z" });
    expect(asCommandError(replies.refusals["not-holder"])?.detail).toMatchObject({ holder: "adapter-a", attempt: 1 });
    expect(asCommandError(replies.refusals["request-ended"])?.detail).toMatchObject({ state: "completed" });
    expect(asCommandError(replies.refusals["key-reused"])?.kind).toBe("key-reused");
    expect(asCommandError(replies.refusals["bad-lease"])?.detail).toEqual({ min: 10, max: 900 });
    expect(asCommandError(replies.refusals["bad-adapter"])?.kind).toBe("bad-adapter");
  });

  it("refuse an acceptance of what moved, or of a design for an earlier text, and say which", () => {
    const moved = asCommandError(replies.refusals["moved"]);
    expect(moved?.kind).toBe("moved");
    expect(moved?.detail).toMatchObject({ moved: ["model"] });
    const notCurrent = asCommandError(replies.refusals["not-current"]);
    expect(notCurrent?.kind).toBe("not-current");
    expect(notCurrent?.detail).toMatchObject({ behind: ["model", "requirements"] });
    expect(asCommandError(replies.refusals["invalid-requirements"])?.message).toContain("not JSON");
    expect(asCommandError(replies.refusals["sce-timeout"])?.detail).toEqual({ seconds: 30 });
  });

  it("say which work was removed, and that it is no longer listed or readable", () => {
    expect(parseRemoved(replies.answers["remove_work"]).title).toBe("Window blind");
    const after = parseListing(replies.answers["list_works_after_removal"]);
    expect(after.works.map((w) => w.title)).toEqual(["Door lock"]);
    expect(after.unreadable).toEqual([]);
    const refusal = asCommandError(replies.refusals["removed-work"]);
    expect(refusal?.kind).toBe("not-found");
    expect(refusal?.message).toContain("removed");
  });

  it("include a refusal for every kind the screen handles, each in the shape of a refusal", () => {
    for (const kind of [
      "conflict",
      "not-found",
      "invalid-id",
      "invalid-title",
      "bad-request",
      "unknown-command",
      "sce-refused",
      "sce-unavailable",
      "sce-timeout",
      "moved",
      "not-current",
      "invalid-requirements",
      "active-request",
      "key-reused",
      "request-held",
      "request-ended",
      "not-holder",
      "bad-lease",
      "bad-adapter",
    ]) {
      const refusal = asCommandError(replies.refusals[kind]);
      expect(refusal, kind).not.toBeNull();
      expect(refusal?.kind).toBe(kind);
    }
  });

  it("give SCE's refusal the product's own code, for the screen to show beside its sentence", () => {
    const refusal = asCommandError(replies.refusals["sce-refused"]);
    expect(refusal?.detail).toEqual({ code: "cli/diagram-does-not-fit" });
    expect(refusal?.message).toContain("cli/diagram-does-not-fit");
  });

  it("give a conflict the two revisions the screen offers the person", () => {
    const refusal = asCommandError(replies.refusals["conflict"]);
    const { base, current } = conflictRevisions(refusal?.detail);
    expect(base).toMatch(/^[0-9a-f]{64}$/);
    expect(current).toMatch(/^[0-9a-f]{64}$/);
    expect(current).not.toBe(base);
  });

  it("name what moved when a request is refused as moved: the text, the answers, the connection", () => {
    expect(movedInRefusal(asCommandError(replies.refusals["request-connection-moved"])?.detail)).toEqual(["connection"]);
    expect(movedInRefusal({ moved: ["source", "answers"] })).toEqual(["source", "answers"]);
    // A refusal that names nothing, or names it in a shape that is not a list, names no part.
    expect(movedInRefusal(undefined)).toEqual([]);
    expect(movedInRefusal({})).toEqual([]);
    expect(movedInRefusal({ moved: "connection" })).toEqual([]);
    expect(movedInRefusal({ moved: ["source", 3] })).toEqual(["source"]);
  });

  it("say which entrance asked, and whether it may change the settings", () => {
    expect(parseDescribed(replies.answers["describe"])).toMatchObject({
      entrance: "tool",
      settings: false,
      writes_settings: false,
    });
    expect(parseDescribed(replies.answers["describe_desktop"])).toMatchObject({
      entrance: "desktop",
      settings: true,
      writes_settings: true,
    });
    for (const kind of ["not-allowed-here", "no-settings", "bad-connection", "connection-moved", "connection-conflict"]) {
      expect(asCommandError(replies.refusals[kind])?.kind, kind).toBeDefined();
    }
    expect(asCommandError(replies.refusals["not-allowed-here"])?.kind).toBe("not-allowed-here");
    expect(asCommandError(replies.refusals["no-settings"])?.kind).toBe("no-settings");
  });

  it("carry the connections a person made and the default among them", () => {
    expect(parseConnectionListing(replies.answers["list_connections_empty"])).toEqual({
      connections: [],
      unreadable: [],
      default: null,
    });
    const listed = parseConnectionListing(replies.answers["list_connections"]);
    expect(listed.default).toBe("main");
    expect(listed.connections.map((c) => c.connection.id)).toEqual(["main", "pc2"]);
    const local = listed.connections[1]?.connection;
    expect(local).toMatchObject({ adapter: "local", display_name: "pc2 (tunnel)", auth: "none" });
    expect(local?.server_url).toBe("http://127.0.0.1:11434/v1");
    expect(parseReadConnection(replies.answers["read_connection_none"])).toBeNull();
    // An earlier revision is what a request made then was made with.
    expect(parseReadConnection(replies.answers["read_connection_revision"])?.connection.model).toBe("opus");
    expect(parseReadConnection(replies.answers["read_connection"])?.connection.model).toBe("sonnet");
    expect(parseDeletedConnection(replies.answers["delete_connection"])).toBe("main");
    expect(parseDefaultConnection(replies.answers["set_default_connection"])).toBe("main");
  });

  it("carry the connection a request was made for, and refuse an executor it is not for", () => {
    const pinned = parseRegisteredRequest(replies.answers["request_generation_pinned"]).request;
    expect(pinned.pin).toMatchObject({ connection: "main", adapter: "claude-code", model: "sonnet" });
    expect(pinned.pin?.revision).toMatch(/^[0-9a-f]{64}$/);
    // A request nobody chose a connection for says so, in a field that is there.
    expect(parseRegisteredRequest(replies.answers["request_generation"]).request.pin).toBeNull();
    const wrong = asCommandError(replies.refusals["wrong-connection"]);
    expect(wrong?.kind).toBe("wrong-connection");
    expect(wrong?.detail).toMatchObject({ pinned: { id: "main" }, offered: null });
    expect(asCommandError(replies.refusals["request-connection-moved"])?.kind).toBe("moved");
  });

  it("carry the decision each way of signing in gets", () => {
    const policy = parseAuthPolicy(replies.answers["read_auth_policy"]);
    const decisionOf = (route: string) => policy.routes.find((r) => r.route === route)?.decision;
    expect(decisionOf("claude-official-login")).toEqual({ decision: "use", status: "conditional" });
    expect(decisionOf("claude-api-key")).toEqual({ decision: "use", status: "allowed" });
    expect(decisionOf("claude-login-screen-by-app")).toEqual({
      decision: "refuse",
      status: "forbidden",
      reason: "forbidden",
    });
    expect(decisionOf("unlisted")).toMatchObject({ decision: "refuse", reason: "unconfirmed" });
    expect(policy.switched_off).toEqual([]);
  });

  it("carry what the screen says of Claude Code: installed or not, who is signed in, how it is billed", () => {
    const status = (name: string) => parseClaudeStatus(replies.answers[name]);
    expect(status("read_claude_status_missing").client).toEqual({ state: "missing" });
    expect(status("read_claude_status_missing").account).toMatchObject({ state: "unknown" });
    // A program that could not be asked is not nobody: no sign-in is offered for it as such.
    expect(status("read_claude_status_unknown").account).toMatchObject({ state: "unknown" });
    expect(status("read_claude_status_signed_out").account).toEqual({ state: "signed-out" });
    expect(status("read_claude_status_subscription")).toMatchObject({
      client: { state: "installed", version: "2.1.291" },
      account: { state: "signed-in", billing: "subscription", environment: null, usable: true },
    });
    // A key from the environment is shown with the name of the variable, and billed by use.
    expect(status("read_claude_status_key_from_environment").account).toMatchObject({
      billing: "usage",
      environment: "ANTHROPIC_API_KEY",
      usable: true,
    });
    // A way of signing in the build does not use is still a login that is there.
    expect(status("read_claude_status_not_used").account).toMatchObject({
      state: "signed-in",
      billing: null,
      usable: false,
      decision: { decision: "refuse", reason: "unconfirmed" },
    });
  });

  it("carry which requests a shell left queued, and why, in ids and a sentence", () => {
    const none = parseHostListing(replies.answers["read_host_status"]);
    expect(none.hosts[0]?.waiting).toEqual([]);
    // Shells are listed by name; the one that hosts and left a request is the browser shell.
    const shells = parseHostListing(replies.answers["read_host_status_waiting"]).hosts;
    const left = shells.find((h) => h.name === "web-shell")?.waiting ?? [];
    expect(left).toHaveLength(1);
    expect(left[0]).toMatchObject({ connection: "claude", reason: expect.stringContaining("claude auth login") });
    expect(left[0]?.request).toBe("<request-id>");
    expect(() => parseHostListing({ hosts: [{ name: "d", hosting: true, live: true, seen_at: "x" }], unreadable: [] })).toThrow(
      /waiting/,
    );
  });

  it("carry the programs the application found for a connection to name, and where each was found", () => {
    expect(parseFindClients(replies.answers["find_clients_none"])).toEqual({ claude: [], codex: [] });
    const found = parseFindClients(replies.answers["find_clients_found"]);
    expect(found.claude.map((c) => [c.version, c.found])).toEqual([
      ["2.1.291", "search-path"],
      ["2.1.280", "known-location"],
    ]);
    expect(found.claude[0]?.path.endsWith("claude")).toBe(true);
    // The programs of the other client are listed apart: a connection names one of its own kind.
    expect(found.codex.map((c) => [c.version, c.found])).toEqual([["0.159.0", "search-path"]]);
    expect(found.codex[0]?.path.endsWith("codex")).toBe(true);
    expect(asCommandError(replies.refusals["not-allowed-to-find-clients"])?.kind).toBe("not-allowed-here");
  });

  it("carry what is known of a model server at an address: where it is, and what it answered", () => {
    const server = (name: string) => parseServerStatus(replies.answers[name]);
    expect(server("read_server_status_listed")).toEqual({
      address: "http://127.0.0.1:11434/v1",
      reach: "this-computer",
      tls: false,
      state: "listed",
      models: ["qwen3-coder:30b", "devstral:24b"],
    });
    // A server with nothing loaded is a server that answered, not one that did not.
    expect(server("read_server_status_listed_none")).toMatchObject({ state: "listed", models: [] });
    // Each way it can go wrong is a state of its own, with the core's words for why.
    expect(server("read_server_status_unreachable")).toMatchObject({
      state: "unreachable",
      reason: expect.stringContaining("could not be reached"),
    });
    expect(server("read_server_status_certificate")).toMatchObject({
      state: "certificate",
      reach: "network",
      tls: true,
    });
    expect(server("read_server_status_needs_key")).toMatchObject({ state: "needs-key", reach: "network", tls: false });
    expect(server("read_server_status_not_a_model_list")).toMatchObject({ state: "not-a-model-list" });
    expect(asCommandError(replies.refusals["not-allowed-to-read-server-status"])?.kind).toBe("not-allowed-here");
    expect(asCommandError(replies.refusals["bad-server-address"])?.kind).toBe("bad-connection");
    // A state the screen does not know is not read as one it does.
    expect(() =>
      parseServerStatus({ server: { address: "http://x", reach: "network", tls: false, state: "asleep" } }),
    ).toThrow(/state/);
    expect(() =>
      parseServerStatus({ server: { address: "http://x", reach: "network", tls: false, state: "listed", models: [1] } }),
    ).toThrow(/models\[0\]/);
  });

  it("carry what the screen says of Codex: which one, whether this build verified it, who is signed in by each source", () => {
    const status = (name: string) => parseCodexStatus(replies.answers[name]);
    const account = (name: string, source: string) =>
      status(name).accounts.find((a) => a.source === source);
    expect(status("read_codex_status_missing").client).toEqual({ state: "missing" });
    // A version that could not be asked about is not an unverified one.
    expect(status("read_codex_status_missing").support).toMatchObject({ state: "unknown" });
    expect(status("read_codex_status_unknown_support").support).toMatchObject({ state: "unknown" });
    expect(status("read_codex_status_verified").support).toEqual({ state: "verified" });
    expect(status("read_codex_status_unverified")).toMatchObject({
      client: { state: "installed", version: "0.159.0" },
      support: { state: "unverified", reason: expect.stringContaining("has not been verified") },
    });
    // Each of the three sources is answered on its own, in the same words as Claude Code's.
    expect(status("read_codex_status_verified").accounts.map((a) => a.source)).toEqual([
      "official-login",
      "app-store",
      "env-api-key",
    ]);
    expect(account("read_codex_status_verified", "official-login")).toMatchObject({
      state: "signed-in",
      billing: "subscription",
      usable: true,
    });
    expect(account("read_codex_status_verified", "app-store")).toMatchObject({ state: "signed-out" });
    // A key from the environment is named by its variable and billed by use, and its value is nowhere.
    expect(account("read_codex_status_key_from_environment", "env-api-key")).toMatchObject({
      state: "signed-in",
      billing: "usage",
      environment: "CODEX_API_KEY",
    });
    expect(status("read_codex_status_key_from_environment").key_variable).toBe("CODEX_API_KEY");
    expect(account("read_codex_status_not_used", "official-login")).toMatchObject({
      usable: false,
      decision: { decision: "refuse", reason: "unconfirmed" },
    });
    expect(account("read_codex_status_missing", "official-login")).toMatchObject({ state: "unknown" });
  });

  it("carry the commands that sign in to Codex, and the folder the application's own login is made in", () => {
    const signIn = parseCodexStatus(replies.answers["read_codex_status_verified"]).sign_in;
    const find = (source: string, billing: string) => signIn.find((s) => s.source === source && s.billing === billing);
    // Each says where the login is kept, so that one the person makes is one a generation finds.
    expect(find("official-login", "subscription")).toMatchObject({
      command: "codex login -c cli_auth_credentials_store=file",
      home: null,
    });
    expect(find("official-login", "usage")).toMatchObject({
      command: "codex login --with-api-key -c cli_auth_credentials_store=file",
      home: null,
    });
    // A login made anywhere else is not the one a generation uses, so the folder is named.
    expect(find("app-store", "subscription")?.home).toMatch(/codex-home$/);
  });

  it("say which program answered when Claude Code is installed", () => {
    const installed = parseClaudeStatus(replies.answers["read_claude_status_subscription"]).client;
    expect(installed).toMatchObject({ state: "installed", version: "2.1.291" });
    expect(installed.state === "installed" ? installed.path : "").toMatch(/claude$/);
  });

  it("carry the two commands that sign in, one for each way it is billed, and no path", () => {
    const signIn = parseClaudeStatus(replies.answers["read_claude_status_signed_out"]).sign_in;
    expect(signIn).toEqual([
      { billing: "subscription", command: "claude auth login" },
      { billing: "usage", command: "claude auth login --console" },
    ]);
  });

  it("carry whether an entrance may start a program, and refuse the one that may not", () => {
    expect(parseDescribed(replies.answers["describe_desktop"]).starts_programs).toBe(true);
    expect(parseDescribed(replies.answers["describe"]).starts_programs).toBe(false);
    expect(asCommandError(replies.refusals["not-allowed-to-start-a-program"])?.kind).toBe("not-allowed-here");
  });
});

describe("a reply that is not the promised shape", () => {
  const listing = replies.answers["list_works"] as { works: Record<string, unknown>[] };

  it("is refused with the place it went wrong", () => {
    expect(() => parseListing({ works: [{ id: "a", title: 1, created_at: "x" }], unreadable: [] })).toThrow(
      /listing\.works\[0\]\.title: expected a string/,
    );
    expect(() => parseListing({ works: [] })).toThrow(/listing\.unreadable: expected a list/);
    expect(() => parseListing(null)).toThrow(ContractError);
    expect(() => parseListing([])).toThrow(ContractError);
  });

  it("is refused when a revision is not a digest", () => {
    expect(() => parseSaved({ outcome: "saved", revision: "abc", parent: null })).toThrow(/revision/);
    expect(() => parseSaved({ outcome: "saved", revision: "A".repeat(64), parent: null })).toThrow(/revision/);
    expect(() => parseSaved({ outcome: "merged", revision: "a".repeat(64) })).toThrow(/outcome/);
  });

  it("is refused when a field the screen reads is missing", () => {
    const work = { ...(listing.works[0] ?? {}) };
    delete work["title"];
    expect(() => parseWork(work)).toThrow(/work\.title/);
    expect(() => parseWorkAndHead({ work: listing.works[0] })).toThrow(/read_work\.head/);
    expect(() => parseReadSource({})).toThrow(/read_source\.source/);
    expect(() => parseRemoved({})).toThrow(/remove_work\.removed/);
    // A request in a state the core does not have is refused by name, and so is one without a count.
    const request = (replies.answers["claim_request"] as { request: Record<string, unknown> }).request;
    expect(() => parseGenerationRequest({ ...request, state: "paused" })).toThrow(/request\.state/);
    expect(() => parseGenerationRequest({ ...request, attempt: -1 })).toThrow(/request\.attempt: expected a count/);
    expect(() => parseGenerationRequest({ ...request, lease: { holder: "a" } })).toThrow(/request\.lease/);
    expect(() => parseRegisteredRequest({ request })).toThrow(/request_generation\.created/);
    // A snapshot that leaves a chain out says which, and a standing must agree with
    // whether the part it is about is there.
    const snapshot = replies.answers["read_work_snapshot"] as Record<string, unknown>;
    const { answers: _answers, ...withoutAnswers } = snapshot;
    expect(() => parseWorkSnapshot(withoutAnswers)).toThrow(/read_work_snapshot\.answers/);
    expect(() => parseWorkSnapshot({ ...snapshot, model_standing: null })).toThrow(
      /read_work_snapshot\.model_standing/,
    );
    expect(() => parseWorkSnapshot({ ...snapshot, requirements_standing: "current" })).toThrow(
      /read_work_snapshot\.requirements_standing: expected null/,
    );
    const set = replies.answers["read_model_set"] as { model: Record<string, unknown> } & Record<string, unknown>;
    expect(() => parseReadModel({ ...set, model: { ...set.model, entry: "gone.scxml" } })).toThrow(
      /read_model\.model\.entry/,
    );
    expect(() => parseReadModel({ ...set, model: { ...set.model, documents: [{ name: "a" }] } })).toThrow(
      /documents\[0\]\.text/,
    );
    expect(() => parseReadAnswers({})).toThrow(/read_answers/);
    expect(() => parseReadAnswers({ answers: { revision: "abc", entries: {} } })).toThrow(/revision/);
    expect(() =>
      parseReadAnswers({ answers: { revision: "a".repeat(64), entries: { q: { answer: 3, answered_at: "t" } } } }),
    ).toThrow(/entries\.q\.answer/);
  });

  it("is refused when a review's verdict is not a word the screen knows, or a record has no code", () => {
    const review = replies.answers["review"] as { check: Record<string, unknown> } & Record<string, unknown>;
    expect(() => parseReview({ ...review, check: { ...review.check, verdict: "maybe" } })).toThrow(
      /review\.check\.verdict/,
    );
    expect(() =>
      parseReview({ ...review, check: { ...review.check, records: [{ message: "m", stage: null, line: null }] } }),
    ).toThrow(/review\.check\.records\[0\]\.code/);
    expect(() => parseReview({ ...review, page: 3 })).toThrow(/review\.page/);
    expect(() => parseReview({ ...review, page_refusal: { code: "c" } })).toThrow(/review\.page_refusal\.message/);
  });

  it("is refused when a model's standing is a word the screen does not know, or a sheet has no svg", () => {
    const read = replies.answers["read_model"] as Record<string, unknown>;
    expect(() => parseReadModel({ ...read, standing: "stale" })).toThrow(/read_model\.standing/);
    expect(() => parseReadModel({ ...read, model: null })).toThrow(/null when there is no model/);
    const figures = replies.answers["figures"] as { sheets: Record<string, unknown>[] };
    expect(() => parseFigures({ ...figures, sheets: [{ name: "a.svg" }] })).toThrow(/figures\.sheets\[0\]\.svg/);
    expect(() => parseFigures({ ...figures, generator: 3 })).toThrow(/figures\.generator/);
  });

  it("is refused when an acceptance and its standing disagree, or a measure is not the shape the screen reads", () => {
    const held = replies.answers["read_acceptance"] as Record<string, unknown>;
    expect(() => parseReadAcceptance({ ...held, standing: "stale" })).toThrow(/read_acceptance\.standing/);
    expect(() => parseReadAcceptance({ ...held, standing: "none" })).toThrow(/holds or lapsed/);
    expect(() => parseReadAcceptance({ ...held, acceptance: null })).toThrow(/an acceptance unless/);
    // A lapse is SCE's sentence, and only a lapsed acceptance has one.
    expect(() => parseReadAcceptance({ ...held, standing: "lapsed", lapse: null })).toThrow(/read_acceptance\.lapse/);
    expect(() => parseReadAcceptance({ ...held, lapse: "design/model.scxml moved" })).toThrow(/read_acceptance\.lapse/);

    const report = replies.answers["requirements_report"] as Record<string, unknown>;
    expect(() => parseRequirementsReport({ ...report, basis: { source: "a".repeat(64) } })).toThrow(/basis\.model/);
    expect(() => parseRequirementsReport({ ...report, model_standing: "stale" })).toThrow(/model_standing/);
    expect(() => parseRequirementsReport({ ...report, outcomes: [{ id: "R1", outcome: "ok", node_paths: [3] }] })).toThrow(
      /outcomes\[0\]\.node_paths\[0\]/,
    );
    expect(() => parseRequirementsReport({ ...report, page_refusal: { code: "c" } })).toThrow(/page_refusal\.message/);
    const list = replies.answers["read_requirements"] as Record<string, unknown>;
    expect(() => parseReadRequirements({ ...list, requirements: null })).toThrow(/null when there is no list/);
  });

  it("is refused when a connection is of an adapter or a credential source the screen does not know", () => {
    const listed = replies.answers["list_connections"] as { connections: Record<string, unknown>[] };
    const first = listed.connections[0] as { connection: Record<string, unknown>; revision: string };
    const withConnection = (patch: Record<string, unknown>) => ({
      ...listed,
      connections: [{ ...first, connection: { ...first.connection, ...patch } }],
    });
    expect(() => parseConnectionListing(withConnection({ adapter: "gemini" }))).toThrow(/adapter/);
    expect(() => parseConnectionListing(withConnection({ auth: "password" }))).toThrow(/auth/);
    expect(() => parseConnectionListing(withConnection({ limits: { turns: 1.5 } }))).toThrow(/turns/);
    expect(() => parseAuthPolicy({ routes: [{ route: "x", status: "maybe", decision: {} }], switched_off: [] })).toThrow(
      /status/,
    );
    expect(() => parseDescribed({ command_set_version: 14, commands: [], root: "r" })).toThrow(/entrance/);
  });

  it("is refused when what is said of Claude Code is a state or a billing the screen does not know", () => {
    const said = replies.answers["read_claude_status_subscription"] as {
      claude: { client: Record<string, unknown>; account: Record<string, unknown>; sign_in: unknown[] };
    };
    const withAccount = (patch: Record<string, unknown>) => ({
      claude: { ...said.claude, account: { ...said.claude.account, ...patch } },
    });
    expect(() => parseClaudeStatus(withAccount({ state: "asleep" }))).toThrow(/state/);
    expect(() => parseClaudeStatus(withAccount({ billing: "free" }))).toThrow(/billing/);
    expect(() => parseClaudeStatus(withAccount({ usable: "yes" }))).toThrow(/usable/);
    expect(() => parseClaudeStatus({ claude: { ...said.claude, client: { state: "broken" } } })).toThrow(/client/);
    expect(() => parseClaudeStatus({ claude: { ...said.claude, sign_in: [{ billing: "free", command: "x" }] } })).toThrow(
      /billing/,
    );
    expect(() => parseClaudeStatus({})).toThrow(/read_claude_status/);
    // An installed client that does not say which program answered is not the shape promised.
    expect(() =>
      parseClaudeStatus({ claude: { ...said.claude, client: { state: "installed", version: "2.1.291" } } }),
    ).toThrow(/path/);
    expect(() =>
      parseFindClients({ claude: [{ path: "/x/claude", version: "1", found: "elsewhere" }], codex: [] }),
    ).toThrow(/found/);
    // A core that does not list Codex's programs is not of this command set.
    expect(() => parseFindClients({ claude: [] })).toThrow(/codex/);
  });

  it("refuses a Codex status that is not in the shape the core gives", () => {
    const said = replies.answers["read_codex_status_verified"] as {
      codex: { accounts: Record<string, unknown>[]; sign_in: Record<string, unknown>[] } & Record<string, unknown>;
    };
    const withSupport = (support: unknown) => ({ codex: { ...said.codex, support } });
    expect(() => parseCodexStatus(withSupport({ state: "trusted" }))).toThrow(/state/);
    expect(() => parseCodexStatus(withSupport({ state: "unverified" }))).toThrow(/reason/);
    expect(() =>
      parseCodexStatus({ codex: { ...said.codex, accounts: [{ ...said.codex.accounts[0], source: "browser" }] } }),
    ).toThrow(/source/);
    expect(() =>
      parseCodexStatus({ codex: { ...said.codex, accounts: [{ ...said.codex.accounts[0], usable: "yes" }] } }),
    ).toThrow(/usable/);
    expect(() =>
      parseCodexStatus({ codex: { ...said.codex, sign_in: [{ ...said.codex.sign_in[0], billing: "free" }] } }),
    ).toThrow(/billing/);
    expect(() => parseCodexStatus({ codex: { ...said.codex, key_variable: undefined } })).toThrow(/key_variable/);
    expect(() => parseCodexStatus({})).toThrow(/read_codex_status/);
  });

  it("is accepted when the core has added a field the screen does not read", () => {
    expect(() => parseWork({ ...(listing.works[0] ?? {}), colour: "blue" })).not.toThrow();
  });

  it("is not mistaken for a refusal unless it has a kind and a message", () => {
    expect(asCommandError({ kind: "conflict" })).toBeNull();
    expect(asCommandError("conflict")).toBeNull();
    expect(asCommandError(null)).toBeNull();
    expect(asCommandError({ kind: "busy", message: "wait" })).toEqual({ kind: "busy", message: "wait" });
  });
});
