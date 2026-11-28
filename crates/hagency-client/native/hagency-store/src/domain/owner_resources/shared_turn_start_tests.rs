//! ADR 0016, option A: the server takes back work whose turn it never
//! recorded and shows its retry. That rests on one premise about the
//! computer: it begins a turn only when the answer to its start request says
//! that the server's record of the start was made now.
//!
//! The daemon meets it by the shape of its code. A run of a process shows
//! only the paths it takes, and a turn sent before the answer would be a
//! path that no test run takes. So this test reads the production sources
//! of the daemon and its store, and fails when
//!
//! - a start request is built anywhere but in the start step, which can ask
//!   only with the attempt that the store gives out once per work, after it
//!   has saved that the start was asked for;
//! - the issued call is built anywhere but where an answer whose record was
//!   not made now is refused, and the answer is saved before the call is
//!   handed back; or
//! - a turn's text is written to a runner by anything but the issued call.
//!
//! An answer for a record made earlier, or no answer at all, then leaves
//! nothing that can send the turn to the CLI.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

macro_rules! store {
    ($file:literal) => {
        concat!(
            "hagency-store/src/domain/owner_resources/capacity/managed_agents/custody/shared_execution/",
            $file
        )
    };
}
macro_rules! host {
    ($file:literal) => {
        concat!("hagency/src/console/native/shared_execution/", $file)
    };
}
/// The files of the shared path, where a generic name such as `.prompt(` can
/// send a pooled turn.
const SHARED_PATH: [&str; 2] = [
    "hagency-store/src/domain/owner_resources/capacity/managed_agents/custody/shared_execution",
    "hagency/src/console/native/shared_execution",
];

/// What a call that writes a turn's text to a runner is spelled like. The
/// first two exist only on a shared Codex process and are looked for in every
/// production file; the others only on the shared path.
const TURN_SENDS: [&str; 6] = [
    "start_shared_text_turn(",
    "start_shared_context_turn(",
    ".prompt(",
    ".start_turn(",
    ".start_claude_turn(",
    ".start_octos_turn(",
];
const EVERYWHERE: usize = 2;
/// Every production call spelled like a turn's send: its text without white
/// space, how often it occurs in its file, and why it is right.
const SENDS: &[(&str, &str, usize, &str)] = &[
    (store!("work.rs"), "process.start_shared_text_turn(self.model_input().into())", 1,
     "the issued call sends its own text (Codex)"),
    (store!("work.rs"), "process.start_shared_context_turn(self.model_input().into(),&digest)", 1,
     "the issued call sends its own text (Codex, with the thread's context)"),
    (store!("capture/claude.rs"), "process.prompt(self.model_input())", 1,
     "the issued call sends its own text (Claude)"),
    (store!("capture/octos.rs"), "process.start_turn(turn_id,self.model_input())", 1,
     "the issued call sends its own text (Octos)"),
    (host!("run.rs"), "issued.start_turn(process)", 1, "the call issued for this start sends the turn"),
    (host!("attempt_run.rs"), "issued.start_turn(process)", 1, "the call issued for this start sends the turn"),
    (host!("budget_run.rs"), "issued.start_turn(process)", 1, "the call issued for this start sends the turn"),
    (host!("claude_run/bridge.rs"), "custody.start_claude_turn(process)", 1,
     "the call issued for this start sends the turn"),
    (host!("octos_capture.rs"), "custody.start_octos_turn(process,turn)", 1,
     "the call issued for this start sends the turn"),
    (host!("run.rs"), "history.prompt(text)", 1, "adds the room's history to the turn's text and sends nothing"),
    (host!("attempt_run.rs"), "history.prompt(text)", 1, "adds the room's history to the turn's text and sends nothing"),
    (host!("budget_run.rs"), "history.prompt(text)", 1, "adds the room's history to the turn's text and sends nothing"),
    (host!("claude_run.rs"), "reader.prompt(text)", 1, "adds the room's history to the turn's text and sends nothing"),
    (host!("octos_run.rs"), "reader.prompt(text)", 1, "adds the room's history to the turn's text and sends nothing"),
];

/// What a request for the start of shared work is spelled like.
const START_REQUESTS: [&str; 3] = ["Action::WorkStart(", "WorkAction::Start(", "ContextAction::Start{"];
/// Every production use of a start request: its text without white space, how
/// often it occurs in its file, whether it builds the request (and must then
/// sit in a function that takes the store's attempt), and why it is right.
const REQUESTS: &[(&str, &str, usize, bool, &str)] = &[
    (host!("context_io.rs"), "b.upload(Action::WorkStart(SharedWorkInput{", 1, true,
     "the start step asks for the start of encrypted work"),
    (host!("context_io.rs"), "b.upload(ContextAction::Start{", 1, true,
     "the start step asks for the start of work with the thread's context"),
    (host!("budget_context_io.rs"), "b.upload(WorkAction::Start(SharedWorkInput{", 1, true,
     "the start step of an Agent with a budget asks for the start of encrypted work"),
    (host!("budget_context_io.rs"), "b.upload(WorkAction::Context(Box::new(ContextAction::Start{", 1, true,
     "the start step of an Agent with a budget asks for the start of work with the thread's context"),
    (host!("io.rs"), "Action::WorkStart(v)=>", 1, false, "checks the request's authority; builds none"),
    (host!("io.rs"), "Action::WorkStart(input)=>", 1, false, "names the request's route; builds none"),
    (host!("budget_work_io.rs"), "ContextAction::Start{..}", 1, false,
     "tells whether a context request needs a fresh proof; builds none"),
];

/// The two places that build the issued call.
const ISSUES: [&str; 2] = [store!("attempt_work.rs"), store!("work.rs")];

#[test]
fn a_turn_is_sent_only_by_a_call_issued_for_a_start_made_now() {
    let native = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the native workspace");
    let mut sources = BTreeMap::new();
    for root in ["hagency/src", "hagency-store/src"] {
        let mut files = Vec::new();
        rust_files(&native.join(root), &mut files);
        assert!(files.len() > 10, "{root} must be read, not skipped");
        for file in files {
            let relative = file
                .strip_prefix(native)
                .expect("inside the workspace")
                .to_string_lossy()
                .replace('\\', "/");
            let text = std::fs::read_to_string(&file).expect("UTF-8 Rust source");
            sources.insert(relative, Source::new(&production(&text)));
        }
    }
    let shared = |file: &str| {
        SHARED_PATH
            .iter()
            .any(|path| file == format!("{path}.rs") || file.starts_with(&format!("{path}/")))
    };

    // A turn's text reaches a runner only through the issued call.
    let sends = audit(&sources, &TURN_SENDS, |file, token| {
        shared(file) || TURN_SENDS[..EVERYWHERE].contains(&token)
    });
    let expected = listed_counts(SENDS.iter().map(|(file, text, count, why)| {
        assert!(!why.is_empty());
        (*file, *text, *count)
    }), &sources, &TURN_SENDS);
    assert_eq!(
        sends, expected,
        "a new place writes a turn's text to a runner: send it through the issued call \
         (SharedCallCustody), which exists only for a start whose record was made now"
    );

    // A start is asked for only with the store's attempt.
    let requests = audit(&sources, &START_REQUESTS, |_, _| true);
    let expected = listed_counts(REQUESTS.iter().map(|(file, text, count, _, why)| {
        assert!(!why.is_empty());
        (*file, *text, *count)
    }), &sources, &START_REQUESTS);
    assert_eq!(
        requests, expected,
        "a new place uses a start request: build it only in the start step, with the store's attempt"
    );
    for (file, text, _, builds, _) in REQUESTS {
        for at in sources[*file].find(text) {
            let function = sources[*file].function(at);
            assert_eq!(
                function.contains("attempt:SharedStartAttempt"),
                *builds,
                "{file}: `{text}` must be built only by a function that takes the store's attempt"
            );
        }
    }

    // The store gives out one attempt per work, after saving that the start
    // was asked for.
    let attempts = literals(&sources, "SharedStartAttempt{");
    assert_eq!(attempts, vec![(store!("work.rs").to_owned(), 1)], "where an attempt is built");
    let work = &sources[store!("work.rs")];
    for at in work.literal("SharedStartAttempt{") {
        let function = work.function(at);
        let asked = refused_before(&function, "kind='start_requested'")
            .expect("a second attempt for the same work is refused as unknown");
        let saved = function
            .find("\"start_requested\"")
            .expect("the attempt saves that the start was asked for");
        assert!(asked < saved, "the refusal of a second attempt comes first");
    }

    // The issued call exists only for an answer whose record was made now,
    // and only after that answer is saved.
    let issued = literals(&sources, "SharedCallCustody{");
    assert_eq!(
        issued,
        ISSUES.iter().map(|file| ((*file).to_owned(), 1)).collect::<Vec<_>>(),
        "where the issued call is built"
    );
    for file in ISSUES {
        let source = &sources[file];
        for at in source.literal("SharedCallCustody{") {
            let function = source.function(at);
            let refused = refused_before(&function, "if!start.newly_started")
                .unwrap_or_else(|| panic!("{file}: an answer for a record made earlier is refused as unknown"));
            let saved = function
                .find("\"start_confirmed\"")
                .unwrap_or_else(|| panic!("{file}: the answer is saved before the call is handed back"));
            assert!(refused < saved, "{file}: the refusal comes before the answer is saved");
        }
    }
    for name in ["SharedCallCustody", "SharedStartAttempt"] {
        let declared = format!("pubstruct{name}{{");
        let fields = work.compact.split_once(&declared).expect("declared in work.rs").1;
        let fields = fields.split_once('}').expect("a closed declaration").0;
        for field in fields.split(',').filter(|field| !field.is_empty()) {
            assert!(
                field.starts_with("pub(super)"),
                "{name}'s `{field}` must stay private to the store's shared execution"
            );
        }
    }
}

/// Production text without white space, with the offset in the production
/// text of each of its bytes.
struct Source {
    text: String,
    compact: String,
    offsets: Vec<usize>,
}
impl Source {
    fn new(text: &str) -> Self {
        let mut compact = String::new();
        let mut offsets = Vec::new();
        for (at, c) in text.char_indices() {
            if !c.is_whitespace() {
                compact.push(c);
                offsets.extend(std::iter::repeat_n(at, c.len_utf8()));
            }
        }
        Self { text: text.to_owned(), compact, offsets }
    }
    fn find(&self, token: &str) -> Vec<usize> {
        self.compact.match_indices(token).map(|(at, _)| at).collect()
    }
    /// Where `token` builds a value: not where it is declared or implemented.
    fn literal(&self, token: &str) -> Vec<usize> {
        self.find(token)
            .into_iter()
            .filter(|at| {
                let before = &self.compact[..*at];
                !before.ends_with("struct") && !before.ends_with("impl") && !before.ends_with("for")
            })
            .collect()
    }
    /// The text without white space from the start of the function that holds
    /// the compact offset `at` up to it.
    fn function(&self, at: usize) -> String {
        let end = self.offsets[at];
        let start = self.text[..end]
            .match_indices("fn ")
            .map(|(at, _)| at)
            .filter(|at| *at == 0 || self.text[..*at].ends_with(char::is_whitespace))
            .last()
            .expect("inside a function");
        self.text[start..end].chars().filter(|c| !c.is_whitespace()).collect()
    }
}

/// How often each token occurs per file, where `looked_for` says to look.
fn audit(
    sources: &BTreeMap<String, Source>,
    tokens: &[&'static str],
    looked_for: impl Fn(&str, &str) -> bool,
) -> BTreeMap<(String, &'static str), usize> {
    let mut found = BTreeMap::new();
    for (file, source) in sources {
        for token in tokens {
            let count = source.compact.matches(token).count();
            if count > 0 && looked_for(file.as_str(), token) {
                found.insert((file.clone(), *token), count);
            }
        }
    }
    found
}

/// The counts that the listed uses make, after checking that each one occurs
/// in its file exactly as often as listed.
fn listed_counts<'a>(
    listed: impl Iterator<Item = (&'a str, &'a str, usize)>,
    sources: &BTreeMap<String, Source>,
    tokens: &[&'static str],
) -> BTreeMap<(String, &'static str), usize> {
    let mut expected = BTreeMap::new();
    for (file, text, count) in listed {
        let source = sources.get(file).unwrap_or_else(|| panic!("{file} is read"));
        assert_eq!(source.find(text).len(), count, "{file}: `{text}`");
        let spelled: Vec<_> = tokens.iter().filter(|token| text.contains(**token)).collect();
        assert_eq!(spelled.len(), 1, "`{text}` is spelled like one of {tokens:?}");
        *expected.entry((file.to_owned(), *spelled[0])).or_default() += count;
    }
    expected
}

/// Each file that builds `token`, and how often.
fn literals(sources: &BTreeMap<String, Source>, token: &str) -> Vec<(String, usize)> {
    sources
        .iter()
        .map(|(file, source)| (file.clone(), source.literal(token).len()))
        .filter(|(_, count)| *count > 0)
        .collect()
}

/// Where `condition` is tested in `function`, when the branch it opens
/// returns the outcome as unknown.
fn refused_before(function: &str, condition: &str) -> Option<usize> {
    let at = function.find(condition)?;
    let branch = &function[at..];
    let open = branch.find('{')?;
    branch[open + 1..]
        .starts_with("returnErr(Error::OutcomeUnknown)")
        .then_some(at)
}

/// Production text: full-line comments and inline `#[cfg(test)] mod … { … }`
/// bodies removed. Braces are counted outside string and char literals.
fn production(source: &str) -> String {
    let mut out = String::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let attribute = trimmed.trim_end();
        if attribute == "#[cfg(test)]" || attribute.starts_with("#[cfg(all(test") {
            while lines
                .peek()
                .is_some_and(|next| next.trim_start().starts_with("#["))
            {
                lines.next();
            }
            let body = lines.peek().is_some_and(|next| {
                let next = next.trim();
                next.ends_with('{')
                    && (next.starts_with("mod ")
                        || next.starts_with("pub mod ")
                        || next.starts_with("pub(crate) mod ")
                        || next.starts_with("pub(super) mod "))
            });
            if body {
                let mut depth = 0i64;
                for inner in lines.by_ref() {
                    depth += braces(inner);
                    if depth <= 0 {
                        break;
                    }
                }
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn braces(line: &str) -> i64 {
    let mut depth = 0;
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' if quoted => {
                chars.next();
            }
            '"' => quoted = !quoted,
            '\'' if !quoted => {
                let mut ahead = chars.clone();
                let first = ahead.next();
                let close = if first == Some('\\') {
                    ahead.next();
                    ahead.next()
                } else {
                    ahead.next()
                };
                if close == Some('\'') {
                    chars = ahead;
                }
            }
            '/' if !quoted && chars.peek() == Some(&'/') => break,
            '{' if !quoted => depth += 1,
            '}' if !quoted => depth -= 1,
            _ => {}
        }
    }
    depth
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source directory") {
        let path = entry.expect("readable source entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if name != "tests" {
                rust_files(&path, files);
            }
        } else if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
            files.push(path);
        }
    }
}
