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
    "Another entrance saved revision {current} after you opened this work. Nothing of yours has been lost, and nothing of theirs has been overwritten.",
  conflictTakeTheirs: "Load theirs (discard my edits)",
  conflictKeepMine: "Keep mine and save on top of theirs",
  conflictKeepMineHint: "Their text stays in the history as its own revision.",
  failureTitle: "The save failed",
  retry: "Try again",
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
    "이 작업을 연 뒤에 다른 곳에서 리비전 {current}을(를) 저장했습니다. 내 글은 잃지 않았고, 그쪽 글도 덮어쓰지 않았습니다.",
  conflictTakeTheirs: "그쪽 글 불러오기 (내 수정 버리기)",
  conflictKeepMine: "내 글을 그 위에 저장",
  conflictKeepMineHint: "그쪽 글은 이력에 별도 리비전으로 남습니다.",
  failureTitle: "저장하지 못했습니다",
  retry: "다시 시도",
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
