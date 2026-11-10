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
    StageStart,
    StageCall,
    StageTranslate,
    StageDone,
    StageFailed,
    CopyDone,
    CopyFailed,
    StallNote,
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
    }

    #[test]
    fn fill_substitutes_once_without_rescanning_args() {
        assert_eq!(fill("{0}초 시간초과", &[&30]), "30초 시간초과");
        assert_eq!(fill("· {0} 실패: {1}", &[&"{1}", &"x"]), "· {1} 실패: x");
        assert_eq!(fill("a {9} b {", &[&1]), "a {9} b {");
    }
}
