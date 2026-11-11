//! UI language. Resolved once at startup: env CHAUDE_LANG > settings `language` > OS locale.
//! Auto maps a Korean locale to ko and anything else to en. The resolved value is exported as
//! CHAUDE_LANG so child claude processes (main, coders, prompter) follow the same language.

use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};

pub const ENV_KEY: &str = "CHAUDE_LANG";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
        }
    }

    // The name Claude Code reads from its `language` setting (passed as --settings to every child, so a user-level
    // "korean" cannot pin the reply language).
    pub fn claude_name(self) -> &'static str {
        match self {
            Lang::Ko => "korean",
            Lang::En => "english",
        }
    }
}

// A user preference: `auto` defers to the next source, a language pins it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pref {
    Auto,
    Fixed(Lang),
}

// Pure: "auto", "ko", "en" (case-insensitive, region suffix allowed like "ko-KR"). Unknown or empty -> None.
pub fn parse_pref(raw: &str) -> Option<Pref> {
    let s = raw.trim().to_ascii_lowercase();
    let base = s.split(['-', '_', '.']).next().unwrap_or_default();
    match base {
        "auto" => Some(Pref::Auto),
        "ko" => Some(Pref::Fixed(Lang::Ko)),
        "en" => Some(Pref::Fixed(Lang::En)),
        _ => None,
    }
}

// Pure: a locale string (e.g. "ko_KR.UTF-8", "ko-KR") to a language. Korean -> ko, everything else -> en.
pub fn lang_from_locale(locale: &str) -> Lang {
    if locale.trim().to_ascii_lowercase().starts_with("ko") {
        Lang::Ko
    } else {
        Lang::En
    }
}

// Pure resolve: env > config > locale. An unknown env or config value is skipped, not fatal.
pub fn resolve(env: Option<&str>, config: Option<&str>, locale: Option<&str>) -> Lang {
    for raw in [env, config].into_iter().flatten() {
        match parse_pref(raw) {
            Some(Pref::Fixed(l)) => return l,
            Some(Pref::Auto) | None => {}
        }
    }
    lang_from_locale(locale.unwrap_or_default())
}

// The OS locale string. Unix: LC_ALL, LC_MESSAGES, LANG (first non-empty). Windows: GetUserDefaultLocaleName.
#[cfg(not(windows))]
pub fn os_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.trim().is_empty())
}

#[cfg(windows)]
pub fn os_locale() -> Option<String> {
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(buf: *mut u16, len: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    // SAFETY: the buffer length is passed exactly; the call writes at most that many UTF-16 units.
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..n as usize - 1]))
}

// Process-wide language. Defaults to Ko so unit tests that assert Korean text keep passing.
static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::Ko,
    }
}

pub fn set_current(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::Ko => 0,
            Lang::En => 1,
        },
        Ordering::Relaxed,
    );
}

// Catalog of user-visible strings. Add a variant, then one arm per language in `text`
// (the matches are exhaustive, so a new language fails to compile until every arm exists).
// Templates use {0}, {1}... filled by `fill`/`tf`; placeholder order may differ per language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    StateRunning,
    StateDone,
    StateFailed,
    StateQueued,
    StateInterrupted,
    StateTimeout,
    NewMessage,
    NoAgents,
    Error,
    EmptyResponse,
    TimedOut,
    DefaultSession,
    PrompterFailed,
    TuiFailed,
    AgentRuleMissing,
    AgentDoneTag,
    AgentFailedTag,
    Inherit,
    Measure,
    RecordTitle,
    RecordMatching,
    RecordRow,
    WorkerCap,
    BusySlash,
    UsageLoading,
    UsageEmpty,
    UsageFailed,
    InputBusy,
    InputIdle,
    SettingsNoPath,
    SettingsSaved,
    SettingsSaveFailed,
    SettingsNextSession,
    RetainDays,
    ExtraCoders,
    ScrollHint,
    BusyRow,
    QueueMore,
    QueueItem,
    TurnTokens,
    TokensUnit,
    StageStart,
    StageCall,
    StageTranslate,
    StageDone,
    StageFailed,
    CopyDone,
    CopyFailed,
    StallNote,
    TasksTitle,
    TaskRunning,
    AgentsWaiting,
    TasksMore,
    LangCurrent,
    LangSet,
    LangInvalid,
    LangPickTitle,
    LangSaveFailed,
    LangHint,
    DoctorTitle,
    DoctorChecking,
    DoctorOk,
    DoctorWarn,
    DoctorFail,
    DoctorFixLabel,
    DoctorNoFix,
    DoctorSummary,
    DoctorCheckOld,
    DoctorCheckPath,
    DoctorCheckUchg,
    DoctorCheckHooks,
    DoctorCheckLogin,
    DoctorCheckTarget,
    DoctorOldNone,
    DoctorOldClean,
    DoctorOldFound,
    DoctorOldUnknown,
    DoctorOldFix,
    DoctorPathOk,
    DoctorPathNone,
    DoctorPathNoneFix,
    DoctorPathShadow,
    DoctorPathShadowFix,
    DoctorUchgClear,
    DoctorUchgLocked,
    DoctorUchgFix,
    DoctorUchgStatFailed,
    DoctorUchgNa,
    DoctorHooksOk,
    DoctorHooksMissing,
    DoctorHooksNoFile,
    DoctorHooksBad,
    DoctorHooksFix,
    DoctorLoginOk,
    DoctorLoginNo,
    DoctorLoginUnknown,
    DoctorLoginFix,
    DoctorTargetNone,
    DoctorTargetOk,
    DoctorTargetBig,
    DoctorTargetFix,
}

pub fn text(lang: Lang, msg: Msg) -> &'static str {
    match (lang, msg) {
        (Lang::Ko, Msg::StateRunning) => "실행 중",
        (Lang::En, Msg::StateRunning) => "running",
        (Lang::Ko, Msg::StateDone) => "완료",
        (Lang::En, Msg::StateDone) => "done",
        (Lang::Ko, Msg::StateFailed) => "실패",
        (Lang::En, Msg::StateFailed) => "failed",
        (Lang::Ko, Msg::StateQueued) => "대기",
        (Lang::En, Msg::StateQueued) => "queued",
        (Lang::Ko, Msg::StateInterrupted) => "중단",
        (Lang::En, Msg::StateInterrupted) => "interrupted",
        (Lang::Ko, Msg::NewMessage) => "↓ 새 메시지",
        (Lang::En, Msg::NewMessage) => "↓ new message",
        (Lang::Ko, Msg::NoAgents) => "에이전트 없음",
        (Lang::En, Msg::NoAgents) => "no agents yet",
        (Lang::Ko, Msg::StateTimeout) => "타임아웃",
        (Lang::En, Msg::StateTimeout) => "timeout",
        (Lang::Ko, Msg::Error) => "오류: {0}",
        (Lang::En, Msg::Error) => "error: {0}",
        (Lang::Ko, Msg::EmptyResponse) => "빈 응답",
        (Lang::En, Msg::EmptyResponse) => "empty response",
        (Lang::Ko, Msg::TimedOut) => "{0}초 시간초과",
        (Lang::En, Msg::TimedOut) => "timed out after {0}s",
        (Lang::Ko, Msg::DefaultSession) => "{0}: 기본 세션으로 시작",
        (Lang::En, Msg::DefaultSession) => "{0}: starting a default session",
        (Lang::Ko, Msg::PrompterFailed) => "prompter 실패({0}): 원문 그대로 메인에 보냄",
        (Lang::En, Msg::PrompterFailed) => "prompter failed ({0}): original sent to main as is",
        (Lang::Ko, Msg::TuiFailed) => "haiku-tui 실패({0}): 메인 원문 표시",
        (Lang::En, Msg::TuiFailed) => "haiku-tui failed ({0}): showing main's original",
        (Lang::Ko, Msg::AgentRuleMissing) => "에이전트 규칙 없음({0})",
        (Lang::En, Msg::AgentRuleMissing) => "no agent rule ({0})",
        (Lang::Ko, Msg::AgentDoneTag) => "[agent 완료]",
        (Lang::En, Msg::AgentDoneTag) => "[agent done]",
        (Lang::Ko, Msg::AgentFailedTag) => "[agent 실패]",
        (Lang::En, Msg::AgentFailedTag) => "[agent failed]",
        (Lang::Ko, Msg::Inherit) => "상속",
        (Lang::En, Msg::Inherit) => "inherit",
        (Lang::Ko, Msg::Measure) => "[측정] {0}",
        (Lang::En, Msg::Measure) => "[measure] {0}",
        (Lang::Ko, Msg::RecordTitle) => "record projects (/resume <번호|이름|검색어> opens one):",
        (Lang::En, Msg::RecordTitle) => "record projects (/resume <number|name|search> opens one):",
        (Lang::Ko, Msg::RecordMatching) => {
            "record projects matching {0} (/resume <이름> opens one):"
        }
        (Lang::En, Msg::RecordMatching) => {
            "record projects matching {0} (/resume <name> opens one):"
        }
        (Lang::Ko, Msg::RecordRow) => "{0} · 파일 {1}개 · {2}",
        (Lang::En, Msg::RecordRow) => "{0} · {1} files · {2}",
        (Lang::Ko, Msg::WorkerCap) => {
            "워커 세션 상한({0}개)에 도달해 새 세션을 시작하지 않습니다. 기존 세션을 정리하거나 -max-workers 값을 올려 주세요."
        }
        (Lang::En, Msg::WorkerCap) => {
            "Worker session cap ({0}) reached; no new session started. Clean up sessions or raise -max-workers."
        }
        (Lang::Ko, Msg::BusySlash) => {
            "생각 중에는 /usage, /version, /help, /status, /mode, /mouse, /view 만 사용할 수 있습니다. 답변이 끝난 뒤 다시 입력해 주세요."
        }
        (Lang::En, Msg::BusySlash) => {
            "While a reply is in progress only /usage, /version, /help, /status, /mode, /mouse, /view work. Enter it again after the reply."
        }
        (Lang::Ko, Msg::UsageLoading) => "조회 중…",
        (Lang::En, Msg::UsageLoading) => "loading…",
        (Lang::Ko, Msg::UsageEmpty) => "/usage 결과를 읽지 못했습니다.",
        (Lang::En, Msg::UsageEmpty) => "could not read the /usage result.",
        (Lang::Ko, Msg::UsageFailed) => "/usage 조회에 실패했습니다: {0}",
        (Lang::En, Msg::UsageFailed) => "/usage failed: {0}",
        (Lang::Ko, Msg::InputBusy) => "생각중 - 추가 입력은 한 번에 합쳐 처리",
        (Lang::En, Msg::InputBusy) => "thinking - extra input is merged into one",
        (Lang::Ko, Msg::InputIdle) => "대기 - 프롬프트 입력",
        (Lang::En, Msg::InputIdle) => "waiting - enter a prompt",
        (Lang::Ko, Msg::SettingsNoPath) => "설정 경로를 알 수 없음 (작업 폴더 없음)",
        (Lang::En, Msg::SettingsNoPath) => "settings path unknown (no working folder)",
        (Lang::Ko, Msg::SettingsSaved) => {
            "설정 저장: {0} · 보존 기간·추가 coder 수·메인 모델은 다음 세션 시작 시 적용"
        }
        (Lang::En, Msg::SettingsSaved) => {
            "settings saved: {0} · retention, extra coders and main model apply at the next session start"
        }
        (Lang::Ko, Msg::SettingsSaveFailed) => "설정 저장 실패: {0}",
        (Lang::En, Msg::SettingsSaveFailed) => "settings save failed: {0}",
        (Lang::Ko, Msg::SettingsNextSession) => "  변경은 다음 세션 시작 시 적용됩니다.",
        (Lang::En, Msg::SettingsNextSession) => "  Changes apply at the next session start.",
        (Lang::Ko, Msg::RetainDays) => "{0}일 (0 = 영구 보존)",
        (Lang::En, Msg::RetainDays) => "{0} days (0 = keep forever)",
        (Lang::Ko, Msg::ExtraCoders) => "{0} (추가 coder ({1}초) 수, 0-8)",
        (Lang::En, Msg::ExtraCoders) => "{0} (extra coders, idle {1}s, 0-8)",
        (Lang::Ko, Msg::ScrollHint) => "  … {0}-{1}/{2} (마우스 휠로 스크롤)",
        (Lang::En, Msg::ScrollHint) => "  … {0}-{1}/{2} (scroll with mouse wheel)",
        (Lang::Ko, Msg::BusyRow) => "생각중-{0}-메인",
        (Lang::En, Msg::BusyRow) => "thinking-{0}-main",
        (Lang::Ko, Msg::QueueMore) => "  … +{0} 대기",
        (Lang::En, Msg::QueueMore) => "  … +{0} queued",
        (Lang::Ko, Msg::QueueItem) => "  대기 {0}",
        (Lang::En, Msg::QueueItem) => "  queued {0}",
        (Lang::Ko, Msg::TasksTitle) => "작업 목록",
        (Lang::En, Msg::TasksTitle) => "Tasks",
        (Lang::Ko, Msg::TaskRunning) => "진행 중",
        (Lang::En, Msg::TaskRunning) => "in progress",
        (Lang::Ko, Msg::AgentsWaiting) => "에이전트 {0}개 대기 중",
        (Lang::En, Msg::AgentsWaiting) => "{0} agents waiting",
        (Lang::Ko, Msg::TasksMore) => "  … +{0}",
        (Lang::En, Msg::TasksMore) => "  … +{0}",
        (Lang::Ko, Msg::TurnTokens) => "턴: {0} 입력 · {1} 출력 · {2} 캐시",
        (Lang::En, Msg::TurnTokens) => "turn: {0} in · {1} out · {2} cache",
        (Lang::Ko, Msg::TokensUnit) => "토큰",
        (Lang::En, Msg::TokensUnit) => "tok",
        (Lang::Ko, Msg::StageStart) => "· {0} 처리중",
        (Lang::En, Msg::StageStart) => "· {0} working",
        (Lang::Ko, Msg::StageCall) => "· {0} 호출",
        (Lang::En, Msg::StageCall) => "· {0} calling",
        (Lang::Ko, Msg::StageTranslate) => "· {0} 번역중",
        (Lang::En, Msg::StageTranslate) => "· {0} translating",
        (Lang::Ko, Msg::StageDone) => "· {0} 완료",
        (Lang::En, Msg::StageDone) => "· {0} done",
        (Lang::Ko, Msg::StageFailed) => "· {0} 실패: {1}",
        (Lang::En, Msg::StageFailed) => "· {0} failed: {1}",
        (Lang::Ko, Msg::CopyDone) => "복사됨 ({0}자)",
        (Lang::En, Msg::CopyDone) => "copied ({0} chars)",
        (Lang::Ko, Msg::CopyFailed) => "복사 실패: {0}",
        (Lang::En, Msg::CopyFailed) => "copy failed: {0}",
        (Lang::Ko, Msg::StallNote) => " · 멈췄을 수 있음 {0}분 무응답",
        (Lang::En, Msg::StallNote) => " · may be stalled, silent for {0} min",
        (Lang::Ko, Msg::LangCurrent) => "언어: {0} (적용 중: {1})",
        (Lang::En, Msg::LangCurrent) => "language: {0} (in use: {1})",
        (Lang::Ko, Msg::LangSet) => {
            "언어 {0} 저장됨. 화면은 바로 바뀌고, 실행 중인 메인과 에이전트는 다음 프롬프트부터 이 언어로 답합니다."
        }
        (Lang::En, Msg::LangSet) => {
            "language {0} saved. The screen switched now; the running main and agents reply in it from the next prompt."
        }
        (Lang::Ko, Msg::LangInvalid) => "알 수 없는 언어 {0} (ko | en | auto)",
        (Lang::En, Msg::LangInvalid) => "unknown language {0} (ko | en | auto)",
        (Lang::Ko, Msg::LangPickTitle) => "언어 선택  (↑/↓ 또는 1-2 · enter 적용 · esc 취소)",
        (Lang::En, Msg::LangPickTitle) => "Language  (↑/↓ or 1-2 · enter apply · esc cancel)",
        (Lang::Ko, Msg::LangSaveFailed) => "언어 저장 실패: {0}",
        (Lang::En, Msg::LangSaveFailed) => "language save failed: {0}",
        (Lang::Ko, Msg::LangHint) => "/lang ko | en | auto 로 바꿉니다",
        (Lang::En, Msg::LangHint) => "change with /lang ko | en | auto",
        (Lang::Ko, Msg::DoctorTitle) => "chaude 점검",
        (Lang::En, Msg::DoctorTitle) => "chaude doctor",
        (Lang::Ko, Msg::DoctorChecking) => "점검 중…",
        (Lang::En, Msg::DoctorChecking) => "checking…",
        (Lang::Ko, Msg::DoctorOk) => "OK",
        (Lang::En, Msg::DoctorOk) => "OK",
        (Lang::Ko, Msg::DoctorWarn) => "경고",
        (Lang::En, Msg::DoctorWarn) => "WARN",
        (Lang::Ko, Msg::DoctorFail) => "실패",
        (Lang::En, Msg::DoctorFail) => "FAIL",
        (Lang::Ko, Msg::DoctorFixLabel) => "조치",
        (Lang::En, Msg::DoctorFixLabel) => "fix",
        (Lang::Ko, Msg::DoctorNoFix) => "조치 없음",
        (Lang::En, Msg::DoctorNoFix) => "nothing to do",
        (Lang::Ko, Msg::DoctorSummary) => "OK {0} · 경고 {1} · 실패 {2}",
        (Lang::En, Msg::DoctorSummary) => "OK {0} · WARN {1} · FAIL {2}",
        (Lang::Ko, Msg::DoctorCheckOld) => "다른 chaude 설치",
        (Lang::En, Msg::DoctorCheckOld) => "other chaude installs",
        (Lang::Ko, Msg::DoctorCheckPath) => "PATH 순서",
        (Lang::En, Msg::DoctorCheckPath) => "PATH order",
        (Lang::Ko, Msg::DoctorCheckUchg) => "설치본 잠금(uchg)",
        (Lang::En, Msg::DoctorCheckUchg) => "install lock (uchg)",
        (Lang::Ko, Msg::DoctorCheckHooks) => "프로젝트 훅 연결",
        (Lang::En, Msg::DoctorCheckHooks) => "project hook wiring",
        (Lang::Ko, Msg::DoctorCheckLogin) => "Claude 로그인",
        (Lang::En, Msg::DoctorCheckLogin) => "claude login",
        (Lang::Ko, Msg::DoctorCheckTarget) => "target/ 크기",
        (Lang::En, Msg::DoctorCheckTarget) => "target/ size",
        (Lang::Ko, Msg::DoctorOldNone) => "0.12.0 미만 chaude 없음",
        (Lang::En, Msg::DoctorOldNone) => "no chaude older than 0.12.0",
        (Lang::Ko, Msg::DoctorOldClean) => "0.12.0 이상: {0} (버전 {1})",
        (Lang::En, Msg::DoctorOldClean) => "0.12.0 or newer: {0} (version {1})",
        (Lang::Ko, Msg::DoctorOldFound) => "0.12.0 미만 설치: {0} (버전 {1})",
        (Lang::En, Msg::DoctorOldFound) => "older install: {0} (version {1})",
        (Lang::Ko, Msg::DoctorOldUnknown) => "버전을 알 수 없는 설치: {0}",
        (Lang::En, Msg::DoctorOldUnknown) => "version unknown: {0}",
        (Lang::Ko, Msg::DoctorOldFix) => "오래된 복사본이면 지우세요: rm {0}",
        (Lang::En, Msg::DoctorOldFix) => "remove it if it is an old copy: rm {0}",
        (Lang::Ko, Msg::DoctorPathOk) => "PATH 첫 항목: {0}",
        (Lang::En, Msg::DoctorPathOk) => "first on PATH: {0}",
        (Lang::Ko, Msg::DoctorPathNone) => "PATH에 chaude가 없습니다",
        (Lang::En, Msg::DoctorPathNone) => "chaude is not on PATH",
        (Lang::Ko, Msg::DoctorPathNoneFix) => {
            "설치 폴더를 PATH에 넣으세요. 예: export PATH=\"$HOME/.local/bin:$PATH\""
        }
        (Lang::En, Msg::DoctorPathNoneFix) => {
            "add the install dir to PATH, e.g. export PATH=\"$HOME/.local/bin:$PATH\""
        }
        (Lang::Ko, Msg::DoctorPathShadow) => "PATH 첫 항목은 {0}, 실행 중인 파일은 {1}",
        (Lang::En, Msg::DoctorPathShadow) => "first on PATH is {0}, this binary is {1}",
        (Lang::Ko, Msg::DoctorPathShadowFix) => {
            "이 설치 폴더를 {0}보다 PATH 앞에 두거나 {0}를 지우세요"
        }
        (Lang::En, Msg::DoctorPathShadowFix) => {
            "put this install dir before {0} in PATH, or remove {0}"
        }
        (Lang::Ko, Msg::DoctorUchgClear) => "uchg 플래그 없음: {0}",
        (Lang::En, Msg::DoctorUchgClear) => "no uchg flag: {0}",
        (Lang::Ko, Msg::DoctorUchgLocked) => "uchg 플래그 있음(잠김): {0}",
        (Lang::En, Msg::DoctorUchgLocked) => "uchg flag set (locked): {0}",
        (Lang::Ko, Msg::DoctorUchgFix) => "업데이트가 막히면 chflags nouchg {0}",
        (Lang::En, Msg::DoctorUchgFix) => "if updates fail: chflags nouchg {0}",
        (Lang::Ko, Msg::DoctorUchgStatFailed) => "stat 실패: {0}",
        (Lang::En, Msg::DoctorUchgStatFailed) => "stat failed: {0}",
        (Lang::Ko, Msg::DoctorUchgNa) => "macOS가 아니라 건너뜀",
        (Lang::En, Msg::DoctorUchgNa) => "not macOS, skipped",
        (Lang::Ko, Msg::DoctorHooksOk) => "생성 훅 {0}개 모두 연결됨 ({1})",
        (Lang::En, Msg::DoctorHooksOk) => "all {0} generated hooks wired in {1}",
        (Lang::Ko, Msg::DoctorHooksMissing) => "생성 훅 {1}개 중 {0}개 없음 ({2})",
        (Lang::En, Msg::DoctorHooksMissing) => "{0} of {1} generated hooks missing in {2}",
        (Lang::Ko, Msg::DoctorHooksNoFile) => "설정 파일 없음: {0}",
        (Lang::En, Msg::DoctorHooksNoFile) => "no settings file: {0}",
        (Lang::Ko, Msg::DoctorHooksBad) => "설정 파일을 읽을 수 없음: {0}",
        (Lang::En, Msg::DoctorHooksBad) => "settings file unreadable: {0}",
        (Lang::Ko, Msg::DoctorHooksFix) => {
            "이 프로젝트에서 chaude를 한 번 실행하면 훅 설정이 복구됩니다"
        }
        (Lang::En, Msg::DoctorHooksFix) => {
            "run chaude once in this project to restore the hook settings"
        }
        (Lang::Ko, Msg::DoctorLoginOk) => "로그인됨 ({0})",
        (Lang::En, Msg::DoctorLoginOk) => "logged in ({0})",
        (Lang::Ko, Msg::DoctorLoginNo) => "claude auth status: 로그인 안 됨",
        (Lang::En, Msg::DoctorLoginNo) => "claude auth status: not logged in",
        (Lang::Ko, Msg::DoctorLoginUnknown) => "로그인 확인 불가 (macOS는 키체인에 보관할 수 있음)",
        (Lang::En, Msg::DoctorLoginUnknown) => {
            "login not confirmed (macOS may keep it in Keychain)"
        }
        (Lang::Ko, Msg::DoctorLoginFix) => "터미널에서 claude login 을 실행하세요",
        (Lang::En, Msg::DoctorLoginFix) => "run claude login in a terminal",
        (Lang::Ko, Msg::DoctorTargetNone) => "target/ 없음 ({0})",
        (Lang::En, Msg::DoctorTargetNone) => "no target/ under {0}",
        (Lang::Ko, Msg::DoctorTargetOk) => "{0} GB",
        (Lang::En, Msg::DoctorTargetOk) => "{0} GB",
        (Lang::Ko, Msg::DoctorTargetBig) => "{0} GB (10 GB 초과)",
        (Lang::En, Msg::DoctorTargetBig) => "{0} GB (over 10 GB)",
        (Lang::Ko, Msg::DoctorTargetFix) => "쓰지 않는 빌드 산출물을 지우세요: rm -rf target/debug",
        (Lang::En, Msg::DoctorTargetFix) => "delete unused build output: rm -rf target/debug",
    }
}

// Pure: a user argument (ko, en, auto, ko-KR, ...) to the settings value auto | ko | en. Unknown -> None.
pub fn pref_code(raw: &str) -> Option<&'static str> {
    match parse_pref(raw)? {
        Pref::Auto => Some("auto"),
        Pref::Fixed(Lang::Ko) => Some("ko"),
        Pref::Fixed(Lang::En) => Some("en"),
    }
}

// The catalog string in the current language.
pub fn t(msg: Msg) -> &'static str {
    text(current(), msg)
}

// Fills {0}, {1}... in one pass; argument text is never rescanned for placeholders.
// An index with no matching arg is kept as written.
pub fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            rest = &rest[open..];
            break;
        };
        match after[..close]
            .parse::<usize>()
            .ok()
            .and_then(|i| args.get(i))
        {
            Some(v) => out.push_str(&v.to_string()),
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

// The catalog template in the current language, filled with args.
pub fn tf(msg: Msg, args: &[&dyn Display]) -> String {
    fill(t(msg), args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_beats_config_beats_locale() {
        assert_eq!(
            resolve(Some("en"), Some("ko"), Some("ko_KR.UTF-8")),
            Lang::En
        );
        assert_eq!(
            resolve(Some("ko"), Some("en"), Some("en_US.UTF-8")),
            Lang::Ko
        );
        assert_eq!(resolve(None, Some("en"), Some("ko_KR.UTF-8")), Lang::En);
        assert_eq!(resolve(None, Some("ko"), Some("en_US.UTF-8")), Lang::Ko);
    }

    #[test]
    fn auto_defers_to_locale() {
        assert_eq!(
            resolve(Some("auto"), Some("auto"), Some("ko_KR.UTF-8")),
            Lang::Ko
        );
        assert_eq!(resolve(None, Some("auto"), Some("en_US.UTF-8")), Lang::En);
        assert_eq!(resolve(None, None, Some("ja_JP.UTF-8")), Lang::En);
        assert_eq!(resolve(None, None, None), Lang::En);
    }

    #[test]
    fn unknown_values_are_skipped() {
        assert_eq!(resolve(Some("fr"), Some("ko"), Some("en_US")), Lang::Ko);
        assert_eq!(resolve(Some(""), None, Some("ko-KR")), Lang::Ko);
        assert_eq!(parse_pref("EN"), Some(Pref::Fixed(Lang::En)));
        assert_eq!(parse_pref("ko-KR"), Some(Pref::Fixed(Lang::Ko)));
        assert_eq!(parse_pref("fr"), None);
    }

    #[test]
    fn pref_code_maps_user_input_to_settings_value() {
        assert_eq!(pref_code("EN"), Some("en"));
        assert_eq!(pref_code("ko-KR"), Some("ko"));
        assert_eq!(pref_code("auto"), Some("auto"));
        assert_eq!(pref_code("fr"), None);
        assert_eq!(pref_code(""), None);
    }

    #[test]
    fn locale_maps_korean_only() {
        assert_eq!(lang_from_locale("ko_KR.UTF-8"), Lang::Ko);
        assert_eq!(lang_from_locale("C"), Lang::En);
        assert_eq!(lang_from_locale("POSIX"), Lang::En);
    }

    #[test]
    fn catalog_renders_per_language() {
        assert_eq!(text(Lang::Ko, Msg::StateDone), "완료");
        assert_eq!(text(Lang::En, Msg::StateDone), "done");
        assert_eq!(text(Lang::En, Msg::NewMessage), "↓ new message");
        assert_eq!(text(Lang::En, Msg::NoAgents), "no agents yet");
        assert_eq!(text(Lang::Ko, Msg::DoctorSummary).matches("{").count(), 3);
        assert_eq!(
            text(Lang::En, Msg::DoctorSummary),
            "OK {0} · WARN {1} · FAIL {2}"
        );
    }

    #[test]
    fn fill_substitutes_once_without_rescanning_args() {
        assert_eq!(fill("{0}초 시간초과", &[&30]), "30초 시간초과");
        assert_eq!(fill("· {0} 실패: {1}", &[&"{1}", &"x"]), "· {1} 실패: x");
        assert_eq!(fill("a {9} b {", &[&1]), "a {9} b {");
    }
}
