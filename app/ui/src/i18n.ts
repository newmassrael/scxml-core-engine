// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen's words, in two languages. `ko` is typed as `Record<Key, string>`, so
// a string added to `en` and not translated is a compile error, not a blank label.

const en = {
  appTitle: "SCE Workbench",
  worksFolder: "Works folder",
  works: "Works",
  noWorks: "No works yet. Name one below to start.",
  newWork: "New work",
  newWorkTitle: "Title",
  newWorkCreate: "Create",
  unreadableHeading: "Folders that could not be read",
  pickAWork: "Pick a work, or create one.",
  noTextYet: "This work has no text yet. Write the specification below and save.",
  editorLabel: "Specification text",
  save: "Save",
  saving: "Saving...",
  saved: "Saved",
  unsaved: "Unsaved changes",
  unchanged: "Nothing to save: the text is the current revision.",
  revision: "Revision",
  noRevision: "none yet",
  history: "History",
  noHistory: "No revisions yet.",
  historyView: "View",
  historyRestore: "Load into editor",
  restoreNeedsSave: "Save your changes first: loading replaces the text in the editor.",
  viewingOld: "Viewing revision {rev}. This is read-only.",
  backToCurrent: "Back to the current text",
  conflictTitle: "The text changed while you were editing",
  conflictBody:
    "Another entrance saved revision {current} after you opened this work. Nothing of yours has been lost, and the version saved elsewhere has not been overwritten.",
  conflictTakeTheirs: "Load the version saved elsewhere (discard my edits)",
  conflictKeepMine: "Keep mine and save on top of it",
  conflictKeepMineHint: "The version saved elsewhere stays in the history as its own revision.",
  failureTitle: "The save failed",
  retry: "Try again",
  switchTitle: "The editor holds text that is not saved",
  switchBody:
    "Opening \"{title}\" would replace what is in the editor. Save it first, discard it, or stay here.",
  saveAndSwitch: "Save, then open it",
  discardAndSwitch: "Discard my changes and open it",
  cancelSwitch: "Stay here",
  restoreSkipped:
    "The editor changed while the older text was being read, so it was not loaded.",
  language: "Language",
  versionMismatch:
    "This screen speaks command set {screen}, the application speaks {core}. Update the one that is older.",
  unauthorized:
    "The server wants its token. Open the address that sce-web-shell printed; it ends in #token=...",
  tokenTitle: "Access token needed",
  tokenBody:
    "This server only answers callers that hold its token. sce-web-shell printed it when it started, in the address after #token=. Paste the token, or the whole address, here.",
  tokenLabel: "Access token",
  tokenConnect: "Connect",
  tokenRefused: "The server did not accept that token. Check that all of it was copied.",
  unreachable: "The application could not be reached.",
  contractBroken:
    "The application answered in a shape this screen does not know ({detail}). The two are probably different versions.",
  loading: "Loading...",
  modelTitle: "Model, as SCE draws it",
  modelNone:
    "No model yet. When an authoring client saves a model for this work, the figures SCE draws of it appear here.",
  modelReading: "Reading the model...",
  modelDrawing: "SCE is drawing the model...",
  modelRead: "Read again",
  modelZoom: "Size",
  modelCurrent: "This model was written for the text as it is now.",
  modelBehind:
    "This model was written for an earlier text ({written}). The text is now at {now}. Ask the authoring client to read it again and save the model.",
  modelBehindUnknown:
    "This model was written for an earlier text ({written}), and the text has since moved on.",
  modelUnstated: "Nothing records which text this model was written for.",
  modelRevision: "Model revision",
  modelGenerator: "Drawn by {generator}",
  modelScxml: "The model's SCXML",
  modelNotDrawn: "SCE did not draw this model.",
  modelNotDrawnHint:
    "The model is saved and is shown below as text. What SCE refused is its own answer, not a fault of this screen.",
  workRemove: "Remove this work",
  removeTitle: "Remove \"{title}\" from the list?",
  removeBody:
    "The work leaves the list and cannot be opened here. Its files stay in the works folder, so it can be brought back by deleting the file removed.json in its folder.",
  removeBodyUnsaved: "Text in the editor that is not saved is lost with it.",
  removeConfirm: "Remove",
  removeCancel: "Keep the work",
  removedNotice: "\"{title}\" was removed from the list.",
  reviewTitle: "What SCE says of the model",
  reviewReading: "Reading what SCE says of the model...",
  reviewFailed: "SCE could not read the model: {detail}",
  reviewAccepted: "SCE accepted the model as a {kind}.",
  reviewAcceptedUnknownKind: "SCE accepted the model.",
  reviewRefused: "SCE refused the model, so there is no pseudocode page.",
  reviewNote:
    "SCE's check says the model is well formed. It does not say the model agrees with your text: read the pseudocode against it.",
  reviewOpenTitle: "Left open",
  reviewNothingOpen: "SCE found nothing the model leaves open.",
  reviewUnresolvedAt: "{id} (line {line})",
  reviewUnresolvedNoLine: "{id}",
  reviewRecordsTitle: "What SCE reported",
  reviewRecordLine: "line {line}",
  reviewPageTitle: "Pseudocode: read it against your text",
  reviewPageRefused: "SCE accepted the model and did not write its pseudocode page: {detail}",
} as const;

export type Key = keyof typeof en;

const ko: Record<Key, string> = {
  appTitle: "SCE 워크벤치",
  worksFolder: "작업 폴더",
  works: "작업",
  noWorks: "아직 작업이 없습니다. 아래에 이름을 적어 시작하세요.",
  newWork: "새 작업",
  newWorkTitle: "제목",
  newWorkCreate: "만들기",
  unreadableHeading: "읽지 못한 폴더",
  pickAWork: "작업을 고르거나 새로 만드세요.",
  noTextYet: "이 작업에는 아직 글이 없습니다. 아래에 사양을 적고 저장하세요.",
  editorLabel: "사양 본문",
  save: "저장",
  saving: "저장 중...",
  saved: "저장됨",
  unsaved: "저장하지 않은 변경",
  unchanged: "저장할 것이 없습니다. 글이 현재 리비전과 같습니다.",
  revision: "리비전",
  noRevision: "아직 없음",
  history: "이력",
  noHistory: "아직 리비전이 없습니다.",
  historyView: "보기",
  historyRestore: "편집기로 불러오기",
  restoreNeedsSave: "먼저 저장하세요. 불러오면 편집기의 글이 바뀝니다.",
  viewingOld: "리비전 {rev}을(를) 보고 있습니다. 읽기 전용입니다.",
  backToCurrent: "현재 글로 돌아가기",
  conflictTitle: "편집하는 동안 글이 바뀌었습니다",
  conflictBody:
    "이 작업을 연 뒤에 다른 곳에서 리비전 {current}을(를) 저장했습니다. 내 글은 잃지 않았고, 다른 곳에서 저장된 사양도 덮어쓰지 않았습니다.",
  conflictTakeTheirs: "다른 곳에서 저장된 사양 불러오기 (내 수정 버리기)",
  conflictKeepMine: "내 글을 그 위에 저장",
  conflictKeepMineHint: "다른 곳에서 저장된 사양은 이력에 별도 리비전으로 남습니다.",
  failureTitle: "저장하지 못했습니다",
  retry: "다시 시도",
  switchTitle: "저장하지 않은 글이 있습니다",
  switchBody:
    "\"{title}\"을(를) 열면 지금 편집기의 글이 사라집니다. 먼저 저장하거나, 버리거나, 그대로 머물 수 있습니다.",
  saveAndSwitch: "저장하고 열기",
  discardAndSwitch: "내 수정을 버리고 열기",
  cancelSwitch: "여기 머물기",
  restoreSkipped: "옛 글을 읽는 동안 편집기가 바뀌어서 불러오지 않았습니다.",
  language: "언어",
  versionMismatch:
    "이 화면은 명령 집합 {screen}을(를), 애플리케이션은 {core}을(를) 씁니다. 더 오래된 쪽을 갱신하세요.",
  unauthorized:
    "서버가 토큰을 요구합니다. sce-web-shell 이 출력한 주소(끝이 #token=... 입니다)로 여세요.",
  tokenTitle: "접속 토큰이 필요합니다",
  tokenBody:
    "이 서버는 토큰을 가진 쪽에만 답합니다. sce-web-shell 이 시작할 때 주소의 #token= 뒤에 출력한 값입니다. 토큰이나 주소 전체를 여기에 붙여 넣으세요.",
  tokenLabel: "접속 토큰",
  tokenConnect: "연결",
  tokenRefused: "서버가 그 토큰을 받지 않았습니다. 끝까지 복사했는지 확인하세요.",
  unreachable: "애플리케이션에 닿지 못했습니다.",
  contractBroken:
    "애플리케이션이 이 화면이 모르는 모양으로 답했습니다 ({detail}). 둘의 버전이 다를 가능성이 큽니다.",
  loading: "불러오는 중...",
  modelTitle: "모델 (SCE가 그린 그림)",
  modelNone:
    "아직 모델이 없습니다. AI 클라이언트가 이 작업의 모델을 저장하면 SCE가 그린 그림이 여기에 나타납니다.",
  modelReading: "모델을 읽는 중...",
  modelDrawing: "SCE가 모델을 그리는 중...",
  modelRead: "다시 읽기",
  modelZoom: "크기",
  modelCurrent: "이 모델은 지금의 원문을 보고 쓴 것입니다.",
  modelBehind:
    "이 모델은 이전 원문({written})을 보고 쓴 것입니다. 지금 원문은 {now}입니다. AI 클라이언트가 원문을 다시 읽고 모델을 저장하게 하세요.",
  modelBehindUnknown:
    "이 모델은 이전 원문({written})을 보고 쓴 것이며, 그 뒤로 원문이 바뀌었습니다.",
  modelUnstated: "이 모델이 어느 원문을 보고 쓰였는지 기록이 없습니다.",
  modelRevision: "모델 리비전",
  modelGenerator: "그린 곳: {generator}",
  modelScxml: "모델의 SCXML 전문",
  modelNotDrawn: "SCE가 이 모델을 그리지 않았습니다.",
  modelNotDrawnHint:
    "모델은 저장되어 있고 아래에 글로 보입니다. SCE가 거절한 것은 SCE 자신의 답이며 이 화면의 결함이 아닙니다.",
  workRemove: "이 작업 지우기",
  removeTitle: "\"{title}\"을(를) 목록에서 지울까요?",
  removeBody:
    "작업이 목록에서 사라지고 여기서는 열 수 없습니다. 파일은 작업 폴더에 그대로 있으므로, 그 폴더의 removed.json 파일을 지우면 되살릴 수 있습니다.",
  removeBodyUnsaved: "편집기에 저장하지 않은 글은 함께 사라집니다.",
  removeConfirm: "지우기",
  removeCancel: "그대로 두기",
  removedNotice: "\"{title}\"을(를) 목록에서 지웠습니다.",
  reviewTitle: "SCE가 모델에 대해 말하는 것",
  reviewReading: "SCE가 모델에 대해 말하는 것을 읽는 중...",
  reviewFailed: "SCE가 모델을 읽지 못했습니다: {detail}",
  reviewAccepted: "SCE가 모델을 {kind}(으)로 받아들였습니다.",
  reviewAcceptedUnknownKind: "SCE가 모델을 받아들였습니다.",
  reviewRefused: "SCE가 모델을 거절했으므로 의사코드 페이지가 없습니다.",
  reviewNote:
    "SCE의 검사는 모델의 형식이 맞다는 뜻입니다. 모델이 내 글과 일치한다는 뜻은 아닙니다. 의사코드를 내 글과 대조해 읽으세요.",
  reviewOpenTitle: "미결로 남은 것",
  reviewNothingOpen: "SCE가 모델에서 미결로 남은 것을 찾지 못했습니다.",
  reviewUnresolvedAt: "{id} ({line}행)",
  reviewUnresolvedNoLine: "{id}",
  reviewRecordsTitle: "SCE가 보고한 것",
  reviewRecordLine: "{line}행",
  reviewPageTitle: "의사코드: 내 글과 대조해 읽으세요",
  reviewPageRefused: "SCE가 모델을 받아들였지만 의사코드 페이지는 쓰지 않았습니다: {detail}",
};

export type Locale = "en" | "ko";

export const LOCALES: readonly Locale[] = ["en", "ko"];

const TABLES: Record<Locale, Record<Key, string>> = { en, ko };

/** The language to start in: the saved choice, else the browser's, else English. */
export function initialLocale(saved: string | null, browser: string | undefined): Locale {
  if (saved === "en" || saved === "ko") return saved;
  return browser?.toLowerCase().startsWith("ko") ? "ko" : "en";
}

/** `{name}` placeholders in the string are replaced from `values`; one without a value is left visible. */
export function translate(
  locale: Locale,
  key: Key,
  values: Readonly<Record<string, string>> = {},
): string {
  return TABLES[locale][key].replace(/\{(\w+)\}/g, (whole, name: string) => values[name] ?? whole);
}

export function languageName(locale: Locale): string {
  return locale === "ko" ? "한국어" : "English";
}

/** Every key with its text in every language, for tests. */
export function allStrings(): ReadonlyArray<readonly [Locale, Key, string]> {
  return LOCALES.flatMap((locale) =>
    (Object.keys(en) as Key[]).map((key) => [locale, key, TABLES[locale][key]] as const),
  );
}
