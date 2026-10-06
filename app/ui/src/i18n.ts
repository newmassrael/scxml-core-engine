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
  switchTitle: "There are changes that are not saved",
  switchBody:
    "Opening \"{title}\" would replace what is on screen: the text in the editor, or the answers you typed. Save them first, discard them, or stay here.",
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
  modelOtherText:
    "This model was read beside another text ({basis}) than the one on screen ({shown}). The screen reads the text again; until it has, do not read this model against the text shown.",
  modelRevision: "Model revision",
  modelGenerator: "Drawn by {generator}",
  modelScxml: "The model's SCXML",
  modelScxmlSet: "The model's SCXML: {count} documents",
  modelEntry: "(the document SCE is asked about)",
  modelNotDrawn: "SCE did not draw this model.",
  modelNotDrawnHint:
    "The model is saved and is shown below as text. What SCE refused is its own answer, not a fault of this screen.",
  workRemove: "Remove this work",
  removeTitle: "Remove \"{title}\" from the list?",
  removeBody:
    "The work leaves the list and cannot be opened here. Its files stay in the works folder, so it can be brought back by deleting the file removed.json in its folder.",
  removeBodyUnsaved: "Anything that is not saved is lost with it: the editor's text, and answers you typed.",
  closeTitle: "There are changes that are not saved",
  closeBody:
    "Closing the window would lose what is on screen that is not saved: the text in the editor, or the answers you typed. Save them first, discard them, or stay here.",
  saveAndClose: "Save, then close",
  discardAndClose: "Discard my changes and close",
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
  answersTitle: "Your answers",
  answersHint:
    "Each question is something your text does not say. Your answer goes to the authoring client the next time it reads this work, and it applies it in the model. Nothing here changes the model.",
  answersReading: "Reading your answers...",
  answersFailed: "Your answers could not be read: {detail}",
  answerLabel: "Your answer to {id}",
  answerNoWording: "The model gave no wording for this question.",
  answerAt: "Said {time}",
  answersSave: "Save answers",
  answersSaved: "Answers saved",
  answersUnsaved: "Unsaved answers",
  answersSaveFailed: "The answers were not saved: {detail}",
  answersConflict:
    "The answers were saved elsewhere while you were typing. What is saved now is loaded and what you typed is kept: look, and save again to put yours on top.",
  answersOrphanTitle: "Answers to questions this model does not ask",
  answersOrphanNote:
    "The model no longer asks these, and they stay for the next draft, which may ask them again. Clear a field and save to take its answer back.",
  acceptTitle: "Accepting the design",
  acceptReading: "Reading the requirements and what was accepted...",
  acceptFailed: "The requirements and the acceptance could not be read: {detail}",
  acceptNoList:
    "No requirement list yet. When an authoring client reads your text into a list of requirements and saves it, you can accept a design against that list here.",
  requirementsBehind:
    "The requirement list was written for an earlier text. Ask the authoring client to read the text again and save the list.",
  requirementsUnstated: "Nothing records which text the requirement list was written for.",
  requirementsCount: "SCE measured the design against {count} requirements ({denominator}).",
  requirementsCountUnstated: "SCE measured the design against {count} requirements.",
  measureFailed: "SCE did not measure the design against the list: {detail}",
  sceAsksAgain: "The screen asks SCE again by itself.",
  sceRefusedTheDesign: "SCE refused the design, so it is not asked again until the work changes.",
  requirementsTable: "The requirements",
  requirementId: "Requirement",
  requirementOutcome: "SCE finds it",
  requirementSection: "Place in the text",
  requirementCarried: "Carried in the design at",
  requirementNowhere: "nowhere",
  outcomeImplemented: "implemented: a node of the design carries it",
  outcomeScenarioPassed: "scenario-passed: every scenario that names it passed",
  outcomeMissing: "missing: nothing in the design carries it",
  outcomeUnresolved: "unresolved: only nodes still marked as open carry it",
  outcomeDangling: "dangling: the design cites it and the list has no such requirement",
  outcomeContradicted: "contradicted: the list places it elsewhere and the design cites it anyway",
  outcomeScenarioFailed: "scenario-failed: a scenario that names it failed",
  outcomeNeedsScenario: "needs-scenario: only a test can settle it",
  outcomeDelegated: "delegated: the list says another document carries it",
  outcomeOutOfScope: "out-of-scope: the list says SCE cannot carry it",
  outcomeSystemLevel: "system-level: the list says the deployment carries it",
  acceptPageTitle: "SCE's acceptance page",
  acceptPageRefused: "SCE measured the design and did not write its acceptance page: {detail}",
  acceptGapsHint:
    "The marked lines are requirements the design leaves unsettled. Accepting records them as they are.",
  acceptGapOpen: "Matters SCE lists as left open: {count}",
  acceptNote:
    "Accepting records that you read this design against this text and this list, knowing what it leaves open. It does not close a question or supply a missing requirement. It holds for as long as the text, the list, the design and your answers are unchanged.",
  acceptButton: "Accept this design",
  acceptAgainButton: "Accept the design as it is now",
  acceptBusy: "Accepting...",
  withheldUnsaved: "Save the text and your answers first: what is accepted is what is saved.",
  withheldNotMeasured: "SCE did not measure the design, so you have not been shown what you would accept.",
  withheldBehind:
    "The design or the requirement list was written for an earlier text, so it cannot be accepted as an answer to this one.",
  withheldUnread:
    "The text, the design or your saved answers are still being read onto the screen, or could not be read: you have not been shown all of what you would accept.",
  withheldDiffers:
    "What is on screen is not the one the design was measured against. Open the work again to read what is saved.",
  withheldAlready: "This design is accepted as it is.",
  withheldUnjudged: "SCE has not said whether the acceptance holds, so whether this design is accepted already is not known.",
  acceptRefused: "Nothing was accepted: {detail}",
  acceptedNone: "Nothing has been accepted yet.",
  acceptedHolds:
    "Accepted {time}. It holds: the text, the list, the design and your answers are as they were.",
  acceptedLapsed: "Accepted {time}, and it no longer holds. SCE says: {lapse}",
  acceptedUnsaid: "Accepted {time}. SCE could not say whether it still holds: {detail}",
  acceptedUnchecked:
    "Accepted {time}. Whether it holds for the design shown is not known yet: the work changed while it was being read. The screen reads it again.",
  channelDirect: "Accepted here, in this application.",
  channelRelayed: "Relayed by an authoring client: it was not accepted in this application.",
  channelUnknown: "Accepted through: {channel}",
  acceptedOpenTitle: "What SCE listed as left open when it was accepted",
  generationTitle: "Pseudocode",
  generateFirst: "Generate pseudocode",
  generateAgain: "Generate again",
  generateReplace: "Replace the request and generate again",
  generateRegistering: "Registering the request...",
  generateCancel: "Cancel the request",
  generateCancelling: "Cancelling...",
  generationIdle: "There is no pseudocode yet. Press the button to have the AI write it from the text.",
  generationNoAi:
    "No AI is connected. A request waits for an authoring client of your own: ask it to write the model for this work.",
  generationAiHere: "Connected: {names}",
  generationNoAiBecause: "No AI is connected. {reasons}",
  generationQueued: "The request is registered. Waiting for the AI to take it.",
  generationQueuedNoAi:
    "The request is waiting, but no AI is connected to take it. Start your authoring client and ask it to write the model for this work, or cancel the request.",
  generationRunning: "The AI is writing the model (attempt {attempt}, {holder}).",
  generationRunningUnnamed: "The AI is writing the model (attempt {attempt}).",
  generationInterrupted:
    "The AI that was writing the model stopped answering. Nothing was published. You can generate again.",
  generationFailed: "The AI could not write the model: {reason}",
  generationFailedUnsaid: "The AI could not write the model, and did not say why.",
  generationCancelled: "You cancelled the last request.",
  generationSuperseded:
    "The last request ended because the text or the answers it was about were saved. Generate again to write from what is saved now.",
  generationCompleted: "The last request finished.",
  generationMoved:
    "The text or the answers changed while the request was being made, so nothing was asked. What is saved now is shown; press the button again.",
  generationActive: "A request is already open for this work. Replace it with a new one?",
  generationRefused: "The request was refused: {detail}",
  guardTitle: "A model is being written for this text",
  guardBody:
    "Saving changes the text or the answers the request is about, so what it writes will not be published. Save and cancel the request, or leave this unsaved.",
  guardSave: "Save and cancel the request",
  guardLeave: "Do not save",
  answerStateUnsaved: "Typed, not saved yet.",
  answerStateSaved: "Saved. It is not in the model shown yet.",
  answerStateWriting: "Saved. The AI is writing it into a new model.",
  answerStateInModel:
    "In the model shown: the model was made after you answered, and it no longer asks this. Read the behaviour it led to.",
  answerStateIgnored:
    "The model shown was made after you answered and still asks this question. Read what it did, or generate again.",
  answersRegenerate: "Generate again from these answers",
  groundRelated: "The text this is about (requirement {id}):",
  groundShow: "Show in the text",
  groundNotInText:
    "That sentence is not in the text as it is now: the requirement quotes an earlier text. Read the text again, or generate again.",
  requirementGo: "Go to",
  markInPage: "Mark in the pseudocode",
  markClear: "Clear the mark",
  markedLines: "Lit: the lines of the page that name the states requirement {id} is carried by ({states}).",
  markNoLines: "No line of the page names {states}: the page does not show where this requirement is carried.",
  markNoStates: "Requirement {id} is carried by places that name no state, so no line of the page can be tied to it.",
  answerAddToText: "Add to the text",
  answerAddToTextHint: "Adds the answer at the end of the text without saving it: put it where it belongs, then save.",
  changeReading: "Comparing with the model before...",
  changeNone: "The pseudocode is the same as in the model before.",
  changeNoPageBefore: "No comparison: SCE did not write a page for the model before.",
  changeNoPageAfter: "No comparison: SCE did not write a page for this model.",
  changeFailed: "The model before could not be compared: {detail}",
  changeTitle: "What changed from the model before ({added} added, {removed} removed)",
  changeNote:
    "Lines of the pseudocode that differ, with the lines around them. A condition, a signal, a value or a time that moved is in the lines that hold it; read them against the text.",
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
  switchTitle: "저장하지 않은 변경이 있습니다",
  switchBody:
    "\"{title}\"을(를) 열면 지금 화면의 편집기 글과 적어 둔 답이 사라집니다. 먼저 저장하거나, 버리거나, 그대로 머물 수 있습니다.",
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
  modelOtherText:
    "이 모델은 화면의 원문({shown})이 아니라 다른 원문({basis})과 함께 읽은 것입니다. 화면이 원문을 다시 읽는 중이며, 다 읽기 전에는 이 모델을 화면의 원문과 대조해 읽지 마세요.",
  modelRevision: "모델 리비전",
  modelGenerator: "그린 곳: {generator}",
  modelScxml: "모델의 SCXML 전문",
  modelScxmlSet: "모델의 SCXML 전문: 문서 {count}개",
  modelEntry: "(SCE가 묻는 기준 문서)",
  modelNotDrawn: "SCE가 이 모델을 그리지 않았습니다.",
  modelNotDrawnHint:
    "모델은 저장되어 있고 아래에 글로 보입니다. SCE가 거절한 것은 SCE 자신의 답이며 이 화면의 결함이 아닙니다.",
  workRemove: "이 작업 지우기",
  removeTitle: "\"{title}\"을(를) 목록에서 지울까요?",
  removeBody:
    "작업이 목록에서 사라지고 여기서는 열 수 없습니다. 파일은 작업 폴더에 그대로 있으므로, 그 폴더의 removed.json 파일을 지우면 되살릴 수 있습니다.",
  removeBodyUnsaved: "저장하지 않은 것(편집기의 글, 적어 둔 답)은 함께 사라집니다.",
  closeTitle: "저장하지 않은 변경이 있습니다",
  closeBody:
    "창을 닫으면 저장하지 않은 편집기의 글과 적어 둔 답이 사라집니다. 먼저 저장하거나, 버리거나, 그대로 머물 수 있습니다.",
  saveAndClose: "저장하고 닫기",
  discardAndClose: "내 변경을 버리고 닫기",
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
  answersTitle: "내 답",
  answersHint:
    "각 질문은 내 글이 말하지 않은 것입니다. 내 답은 AI 클라이언트가 이 작업을 다음에 읽을 때 전달되고, 클라이언트가 모델에 반영합니다. 여기서 모델이 바뀌지는 않습니다.",
  answersReading: "내 답을 읽는 중...",
  answersFailed: "내 답을 읽지 못했습니다: {detail}",
  answerLabel: "{id}에 대한 내 답",
  answerNoWording: "모델이 이 질문의 문장을 적지 않았습니다.",
  answerAt: "답한 때 {time}",
  answersSave: "답 저장",
  answersSaved: "답 저장됨",
  answersUnsaved: "저장하지 않은 답",
  answersSaveFailed: "답을 저장하지 못했습니다: {detail}",
  answersConflict:
    "답을 적는 동안 다른 곳에서 답이 저장되었습니다. 지금 저장된 답을 불러왔고 내가 적은 것은 그대로 두었습니다. 살펴보고, 다시 저장하면 내 답이 그 위에 저장됩니다.",
  answersOrphanTitle: "이 모델이 묻지 않는 질문의 답",
  answersOrphanNote:
    "모델이 더 이상 묻지 않는 질문의 답입니다. 다음 초안이 다시 물을 수 있어서 남겨 둡니다. 칸을 비우고 저장하면 그 답을 거둡니다.",
  acceptTitle: "설계 수락",
  acceptReading: "요구사항과 수락 내역을 읽는 중...",
  acceptFailed: "요구사항과 수락 내역을 읽지 못했습니다: {detail}",
  acceptNoList:
    "아직 요구사항 목록이 없습니다. AI 클라이언트가 내 글을 요구사항 목록으로 읽어 저장하면, 여기서 그 목록에 대해 설계를 수락할 수 있습니다.",
  requirementsBehind:
    "요구사항 목록이 이전 원문을 보고 만들어졌습니다. AI 클라이언트가 원문을 다시 읽고 목록을 저장하게 하세요.",
  requirementsUnstated: "요구사항 목록이 어느 원문을 보고 만들어졌는지 기록이 없습니다.",
  requirementsCount: "SCE가 설계를 요구사항 {count}개({denominator})에 대해 쟀습니다.",
  requirementsCountUnstated: "SCE가 설계를 요구사항 {count}개에 대해 쟀습니다.",
  measureFailed: "SCE가 설계를 목록에 대해 재지 못했습니다: {detail}",
  sceAsksAgain: "화면이 SCE에 저절로 다시 묻습니다.",
  sceRefusedTheDesign: "SCE가 설계를 거절해서, 작업이 바뀌기 전에는 다시 묻지 않습니다.",
  requirementsTable: "요구사항",
  requirementId: "요구사항",
  requirementOutcome: "SCE의 판정",
  requirementSection: "원문의 위치",
  requirementCarried: "설계에서 담은 곳",
  requirementNowhere: "없음",
  outcomeImplemented: "implemented: 설계의 노드가 담고 있음",
  outcomeScenarioPassed: "scenario-passed: 이 요구사항을 가리키는 시나리오가 모두 통과함",
  outcomeMissing: "missing: 설계에 담은 곳이 없음",
  outcomeUnresolved: "unresolved: 미결로 표시된 노드만 담고 있음",
  outcomeDangling: "dangling: 설계가 인용했지만 목록에 그런 요구사항이 없음",
  outcomeContradicted: "contradicted: 목록은 다른 곳에 둔다고 했는데 설계가 그래도 인용함",
  outcomeScenarioFailed: "scenario-failed: 이 요구사항을 가리키는 시나리오가 실패함",
  outcomeNeedsScenario: "needs-scenario: 시나리오로만 확인할 수 있음",
  outcomeDelegated: "delegated: 목록이 다른 문서가 담는다고 함",
  outcomeOutOfScope: "out-of-scope: 목록이 SCE가 담을 수 없다고 함",
  outcomeSystemLevel: "system-level: 목록이 배포 환경이 담는다고 함",
  acceptPageTitle: "SCE의 수락 페이지",
  acceptPageRefused: "SCE가 설계를 쟀지만 수락 페이지는 쓰지 않았습니다: {detail}",
  acceptGapsHint: "표시된 줄은 설계가 매듭짓지 않은 요구사항입니다. 수락은 그대로 기록할 뿐입니다.",
  acceptGapOpen: "SCE가 미결로 남았다고 한 것: {count}건",
  acceptNote:
    "수락은 내가 이 원문과 이 목록에 대해 이 설계를 읽었고, 설계가 미결로 남긴 것을 알았다는 기록입니다. 질문을 닫거나 빠진 요구사항을 채워 주지는 않습니다. 원문, 목록, 설계, 내 답이 그대로인 동안만 유효합니다.",
  acceptButton: "이 설계 수락",
  acceptAgainButton: "지금의 설계를 다시 수락",
  acceptBusy: "수락하는 중...",
  withheldUnsaved: "먼저 글과 답을 저장하세요. 수락되는 것은 저장된 것입니다.",
  withheldNotMeasured: "SCE가 설계를 재지 못해서, 수락할 것을 아직 보지 못했습니다.",
  withheldBehind:
    "설계나 요구사항 목록이 이전 원문을 보고 만들어져서, 지금 원문에 대한 답으로 수락할 수 없습니다.",
  withheldUnread:
    "글, 설계, 저장된 답을 아직 화면에 읽는 중이거나 읽지 못해서, 수락할 것을 모두 보지 못했습니다.",
  withheldDiffers:
    "화면에 있는 것이 설계를 잰 기준과 다릅니다. 작업을 다시 열어 저장된 것을 읽으세요.",
  withheldAlready: "이 설계는 이미 지금 그대로 수락되어 있습니다.",
  withheldUnjudged: "SCE가 수락이 유효한지 말하지 않아서, 이 설계가 이미 수락되었는지 알 수 없습니다.",
  acceptRefused: "아무것도 수락되지 않았습니다: {detail}",
  acceptedNone: "아직 수락한 것이 없습니다.",
  acceptedHolds: "{time}에 수락. 유효합니다: 원문, 목록, 설계, 내 답이 수락 때와 같습니다.",
  acceptedLapsed: "{time}에 수락했지만 더는 유효하지 않습니다. SCE의 말: {lapse}",
  acceptedUnsaid: "{time}에 수락했습니다. SCE가 아직 유효한지 말하지 못했습니다: {detail}",
  acceptedUnchecked:
    "{time}에 수락했습니다. 지금 보이는 설계에 대해 유효한지는 아직 알 수 없습니다. 읽는 동안 작업이 바뀌었습니다. 화면이 다시 읽는 중입니다.",
  channelDirect: "이 애플리케이션에서 직접 수락했습니다.",
  channelRelayed: "AI 클라이언트가 전한 것으로, 이 애플리케이션에서 직접 수락한 것이 아닙니다.",
  channelUnknown: "수락 경로: {channel}",
  acceptedOpenTitle: "수락할 때 SCE가 미결로 남았다고 한 것",
  generationTitle: "의사코드",
  generateFirst: "의사코드 생성",
  generateAgain: "다시 생성",
  generateReplace: "요청을 대체하고 다시 생성",
  generateRegistering: "요청을 등록하는 중...",
  generateCancel: "요청 취소",
  generateCancelling: "취소하는 중...",
  generationIdle: "아직 의사코드가 없습니다. 버튼을 누르면 AI가 이 글로 작성합니다.",
  generationNoAi:
    "AI가 연결되지 않았습니다. 요청은 직접 쓰는 작성 도구가 가져갈 때까지 기다립니다. 그 도구에게 이 작업의 모델을 쓰라고 하세요.",
  generationAiHere: "연결됨: {names}",
  generationNoAiBecause: "AI가 연결되지 않았습니다. {reasons}",
  generationQueued: "생성 요청을 등록했습니다. AI가 가져가기를 기다리는 중입니다.",
  generationQueuedNoAi:
    "요청이 대기 중이지만 실행할 AI 연결이 없습니다. 작성 도구를 시작해 이 작업의 모델을 쓰라고 하거나, 요청을 취소하세요.",
  generationRunning: "AI가 모델을 작성하고 있습니다 ({attempt}번째 시도, {holder}).",
  generationRunningUnnamed: "AI가 모델을 작성하고 있습니다 ({attempt}번째 시도).",
  generationInterrupted:
    "모델을 쓰던 AI가 응답을 멈췄습니다. 공개된 것은 없습니다. 다시 생성할 수 있습니다.",
  generationFailed: "AI가 모델을 작성하지 못했습니다: {reason}",
  generationFailedUnsaid: "AI가 모델을 작성하지 못했고, 이유를 말하지 않았습니다.",
  generationCancelled: "마지막 요청을 취소했습니다.",
  generationSuperseded:
    "요청이 기준으로 삼은 사양이나 답변이 저장되어 마지막 요청이 끝났습니다. 지금 저장된 내용으로 다시 생성하세요.",
  generationCompleted: "마지막 요청이 끝났습니다.",
  generationMoved:
    "요청하는 동안 사양이나 답변이 바뀌어 아무것도 요청하지 않았습니다. 지금 저장된 내용을 보여 드렸으니 버튼을 다시 누르세요.",
  generationActive: "이 작업에는 이미 열린 요청이 있습니다. 새 요청으로 대체할까요?",
  generationRefused: "요청이 거절되었습니다: {detail}",
  guardTitle: "이 사양으로 모델을 작성하는 중입니다",
  guardBody:
    "저장하면 요청이 기준으로 삼은 사양이나 답변이 바뀌어, AI가 쓰는 결과는 공개되지 않습니다. 저장하고 요청을 취소하거나, 저장하지 않고 두세요.",
  guardSave: "저장하고 요청 취소",
  guardLeave: "저장하지 않음",
  answerStateUnsaved: "입력했지만 아직 저장하지 않았습니다.",
  answerStateSaved: "저장했습니다. 지금 보이는 모델에는 아직 없습니다.",
  answerStateWriting: "저장했습니다. AI가 새 모델에 반영하고 있습니다.",
  answerStateInModel:
    "지금 보이는 모델에 반영됨: 답한 뒤에 만든 모델이고 더는 이 질문을 하지 않습니다. 이어진 동작을 읽어 보세요.",
  answerStateIgnored:
    "지금 보이는 모델은 답한 뒤에 만들었는데도 이 질문을 여전히 합니다. 모델이 한 일을 읽거나 다시 생성하세요.",
  answersRegenerate: "이 답으로 다시 생성",
  groundRelated: "이 질문이 다루는 원문 (요구사항 {id}):",
  groundShow: "원문에서 보기",
  groundNotInText:
    "그 문장은 지금 원문에 없습니다. 요구사항이 이전 원문을 인용하고 있습니다. 원문을 다시 읽거나 다시 생성하세요.",
  requirementGo: "이동",
  markInPage: "의사코드에 표시",
  markClear: "표시 지우기",
  markedLines: "강조: 요구사항 {id}이(가) 담긴 상태({states})를 이름으로 부르는 페이지의 줄입니다.",
  markNoLines: "페이지에 {states}을(를) 부르는 줄이 없습니다. 이 페이지로는 이 요구사항이 어디에 담겼는지 보이지 않습니다.",
  markNoStates: "요구사항 {id}은(는) 상태 이름이 없는 위치에 담겨 있어, 페이지의 어느 줄과도 이을 수 없습니다.",
  answerAddToText: "원문에 추가",
  answerAddToTextHint: "답을 저장하지 않고 원문 끝에 붙입니다. 알맞은 자리로 옮긴 뒤 저장하세요.",
  changeReading: "이전 모델과 비교하는 중...",
  changeNone: "의사코드가 이전 모델과 같습니다.",
  changeNoPageBefore: "비교할 수 없습니다: SCE가 이전 모델의 페이지를 쓰지 않았습니다.",
  changeNoPageAfter: "비교할 수 없습니다: SCE가 이 모델의 페이지를 쓰지 않았습니다.",
  changeFailed: "이전 모델과 비교하지 못했습니다: {detail}",
  changeTitle: "이전 모델에서 바뀐 것 ({added}줄 추가, {removed}줄 삭제)",
  changeNote:
    "의사코드에서 달라진 줄과 그 앞뒤 줄입니다. 바뀐 조건, 신호, 값, 시간은 그것을 담은 줄에 있습니다. 원문과 견주어 읽으세요.",
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
