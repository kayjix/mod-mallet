//! Reads the reports crash loggers write when the game crashes.
//!
//! Three loggers are common, and they write similar reports in different
//! shapes (ADR 0044):
//!
//! ```text
//! Crash Logger SSE AE VR      <My Games>/<game>/SKSE/crash-YYYY-MM-DD-HH-MM-SS.log
//! Trainwreck                  <My Games>/<game>/SKSE/Crashlogs/crash-YYYY-MM-DD-HH-MM-SS.log
//! .NET Script Framework       <game>/Data/NetScriptFramework/Crash/Crash_YYYY_M_D_H-M-S.txt
//! ```
//!
//! Crash Logger and Trainwreck start alike:
//!
//! ```text
//! Unhandled exception "EXCEPTION_ACCESS_VIOLATION" at 0x7FF700001234 SkyrimSE.exe+0001234    mov eax, [rcx]
//! ```
//!
//! and list the call stack under `PROBABLE CALL STACK:`, or, in newer
//! versions of Crash Logger, under `CALL STACK (…):` with probable frames
//! marked `[P]`. Newer versions of Crash Logger also list
//! `POSSIBLE RELEVANT OBJECTS:`, one per line:
//!
//! ```text
//!     RSP+10: (Character*) "Some Name" [0x00012345] ("Some Mod.esp")
//! ```
//!
//! The .NET Script Framework writes the address in brackets and its
//! sections in braces:
//!
//! ```text
//! Unhandled native exception occurred at 0x7FF700001234 (SkyrimSE.exe+1234) on thread 42!
//! Possible relevant objects (1)
//! {
//!   [  10]   Character(Name: `Some Name`, FormId: 00012345, File: `Some Mod.esp`)
//! }
//! Probable callstack
//! {
//!   [0]   0x7FF700001234   (SkyrimSE.exe+1234)
//! }
//! ```
//!
//! Where a report has no list of relevant objects, the objects are taken
//! from the registers and the stack. Only the reports of the last
//! [`WINDOW_DAYS`] days are read, at most [`MAX_LOGS`] of them, newest
//! first: older crashes say little about the setup as it is now.
//!
//! A report is untrusted input (ADR 0013): [`parse`] accepts any text,
//! skips what it does not recognise and never panics.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use mallet_core::facts::crashes::{CrashFacts, CrashLog, CrashLogger, CrashObject};

/// How many days back crash reports are read.
pub const WINDOW_DAYS: u32 = 30;

/// How many crash reports are read at most.
pub const MAX_LOGS: usize = 20;

/// The largest report that is read. Real ones are well under a megabyte.
const MAX_LOG_SIZE: u64 = 16 * 1024 * 1024;

/// How many probable frames of the call stack are kept.
const MAX_FRAMES: usize = 12;

/// The folders crash loggers write to, with the logger that writes there.
/// `extender_dir` is the folder the script extender writes its log to, the
/// one Crash Logger and Trainwreck write to as well; `game_dir` is the
/// folder the game is installed in. The folders need not exist.
pub fn folders(extender_dir: Option<&Path>, game_dir: &Path) -> Vec<(PathBuf, CrashLogger)> {
    let mut folders = Vec::new();
    if let Some(extender) = extender_dir {
        let extender = extender.to_path_buf();
        folders.push((extender.join("Crashlogs"), CrashLogger::Trainwreck));
        folders.push((extender, CrashLogger::CrashLogger));
    }
    folders.push((
        game_dir
            .join("Data")
            .join("NetScriptFramework")
            .join("Crash"),
        CrashLogger::NetScriptFramework,
    ));
    folders
}

/// Reads the recent crash reports in `folders`, as [`folders`] gives them,
/// as of `now`.
pub fn read(folders: &[(PathBuf, CrashLogger)], now: SystemTime) -> CrashFacts {
    let oldest = now
        .checked_sub(Duration::from_secs(u64::from(WINDOW_DAYS) * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut found: Vec<(SystemTime, PathBuf, CrashLogger)> = Vec::new();
    for (folder, logger) in folders {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !is_report_name(*logger, &name) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            if metadata.is_file() && metadata.len() <= MAX_LOG_SIZE && modified >= oldest {
                found.push((modified, entry.path(), *logger));
            }
        }
    }
    found.sort_by_key(|(modified, _, _)| std::cmp::Reverse(*modified));
    found.truncate(MAX_LOGS);

    let logs = found
        .into_iter()
        .filter_map(|(_, path, logger)| {
            let bytes = std::fs::read(&path).ok()?;
            let name = path.file_name()?.to_string_lossy().into_owned();
            Some(parse(logger, &name, &String::from_utf8_lossy(&bytes)))
        })
        .collect();
    CrashFacts {
        logs,
        window_days: WINDOW_DAYS,
    }
}

/// Whether a file of this name, in lower case, is a report of `logger`.
fn is_report_name(logger: CrashLogger, name: &str) -> bool {
    match logger {
        CrashLogger::CrashLogger | CrashLogger::Trainwreck => {
            name.starts_with("crash-") && has_extension(name, "log")
        }
        CrashLogger::NetScriptFramework => name.starts_with("crash_") && has_extension(name, "txt"),
    }
}

/// Extracts what a crash report says. `logger` is the logger whose folder
/// it was found in; the text itself decides where it can.
pub fn parse(logger: CrashLogger, file: &str, text: &str) -> CrashLog {
    let logger = if text.contains("Trainwreck") {
        CrashLogger::Trainwreck
    } else if text.contains("Unhandled native exception occurred at") {
        CrashLogger::NetScriptFramework
    } else {
        logger
    };
    let lines: Vec<&str> = text.lines().collect();
    let (exception, address) = lines
        .iter()
        .find_map(|line| crash_line(line))
        .unwrap_or((None, None));

    let mut objects: Vec<CrashObject> = section(
        &lines,
        &["POSSIBLE RELEVANT OBJECTS", "Possible relevant objects"],
    )
    .into_iter()
    .filter_map(object)
    .collect();
    if objects.is_empty() {
        // Without a list of their own, the objects are in the registers
        // and on the stack.
        objects = section(&lines, &["REGISTERS", "Registers"])
            .into_iter()
            .chain(section(&lines, &["STACK", "Stack"]))
            .filter(|line| !line.trim_start().starts_with("Modified by"))
            .filter_map(object)
            .filter(|object| object.plugin.is_some() || object.mesh.is_some())
            .collect();
    }

    let mut stack_modules: Vec<String> = Vec::new();
    for line in section(
        &lines,
        &["PROBABLE CALL STACK", "CALL STACK", "Probable callstack"],
    ) {
        let trimmed = line.trim_start();
        // In the newer list, frames found by scanning the stack are marked
        // `[S]` and are no more than guesses.
        if trimmed.contains("][S]") {
            continue;
        }
        if let Some(module) = frame_address(line).and_then(|address| module_of(&address)) {
            if !stack_modules
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&module))
            {
                stack_modules.push(module);
            }
            if stack_modules.len() >= MAX_FRAMES {
                break;
            }
        }
    }

    CrashLog {
        file: file.to_owned(),
        logger,
        time: time_from_name(file),
        exception,
        address,
        stack_modules,
        objects,
    }
}

/// The exception and the address from the line that reports the crash.
fn crash_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    if let Some(rest) = line.strip_prefix("Unhandled exception ") {
        let exception = between(rest, "\"", "\"").map(str::to_owned);
        let after = rest.split_once(" at ").map(|(_, after)| after)?;
        return Some((exception, frame_address(after)));
    }
    if line.starts_with("Unhandled native exception occurred at") {
        return Some((None, frame_address(line)));
    }
    None
}

/// The address `module+offset` in a line that names one: in brackets, or
/// after a hexadecimal number.
fn frame_address(line: &str) -> Option<String> {
    // `(SkyrimSE.exe+1234)` in reports of the .NET Script Framework.
    if let Some(inner) = between(line, "(", ")")
        && is_address(inner)
    {
        return Some(inner.trim().to_owned());
    }
    // `0x7FF700001234   SkyrimSE.exe+1234 -> 90654+0x12	test …` in the
    // others. A program file's name may contain blanks.
    let start = line.find("0x")?;
    let rest = line.get(start..)?;
    let after_number = rest
        .find(char::is_whitespace)
        .and_then(|end| rest.get(end..))?;
    let after_number = after_number.trim_start();
    let end = [
        after_number.find('\t'),
        after_number.find(" -> "),
        after_number.find(" |"),
    ]
    .into_iter()
    .flatten()
    .min()
    .unwrap_or(after_number.len());
    let candidate = after_number.get(..end)?.trim();
    is_address(candidate).then(|| candidate.to_owned())
}

/// Whether `text` looks like `module.ext+offset`.
fn is_address(text: &str) -> bool {
    let Some((module, offset)) = text.trim().rsplit_once('+') else {
        return false;
    };
    let module = module.to_ascii_lowercase();
    (has_extension(&module, "exe") || has_extension(&module, "dll"))
        && !offset.is_empty()
        && offset.chars().all(|c| c.is_ascii_hexdigit())
}

/// Whether the file `name` has the extension `extension`, in any case.
fn has_extension(name: &str, extension: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

/// The program file of an address.
fn module_of(address: &str) -> Option<String> {
    address
        .rsplit_once('+')
        .map(|(module, _)| module.trim().to_owned())
}

/// The lines of the first section whose title starts with one of
/// `titles`: up to the next empty line, or, in braces, up to the closing
/// brace.
fn section<'a>(lines: &[&'a str], titles: &[&str]) -> Vec<&'a str> {
    let Some(start) = lines.iter().position(|line| {
        let line = line.trim_end();
        titles.iter().any(|title| line.starts_with(title))
    }) else {
        return Vec::new();
    };
    let mut body = lines.iter().skip(start + 1).peekable();
    let braced = body.peek().is_some_and(|line| line.trim() == "{");
    if braced {
        body.next();
        return body
            .take_while(|line| line.trim() != "}")
            .copied()
            .collect();
    }
    body.take_while(|line| !line.trim().is_empty())
        .copied()
        .collect()
}

/// A game object named in a line, if the line names one.
fn object(line: &str) -> Option<CrashObject> {
    let quoted = quoted_texts(line);
    let plugin = quoted
        .iter()
        .rev()
        // A chain `A.esm <- B.esp` ends with the plugin whose version is used.
        .flat_map(|text| text.rsplit(" <- "))
        .find(|text| is_plugin(text))
        .map(|text| text.trim().to_owned());
    let mesh = quoted
        .iter()
        .find(|text| text.to_ascii_lowercase().ends_with(".nif"))
        .map(|text| (*text).to_owned());
    let name = quoted
        .iter()
        .find(|text| !is_plugin(text) && !text.to_ascii_lowercase().ends_with(".nif"))
        .map(|text| (*text).to_owned());
    let kind = between(line, "(", "*)")
        .filter(|kind| !kind.contains(['(', '"']))
        .map(str::to_owned)
        .or_else(|| {
            // `Character(Name: …)` in reports of the .NET Script Framework.
            let after_index = line
                .trim_start()
                .split_once(']')
                .map_or(line, |(_, rest)| rest);
            let head = after_index.trim_start().split('(').next()?;
            (!head.is_empty()
                && head
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .then(|| head.to_owned())
        });
    let form_id = between(line, "[0x", "]")
        .map(|id| format!("0x{id}"))
        .or_else(|| {
            between(line, "FormId: ", ",")
                .or_else(|| between(line, "FormId: ", ")"))
                .map(str::to_owned)
        });
    if plugin.is_none() && mesh.is_none() && name.is_none() && kind.is_none() {
        return None;
    }
    Some(CrashObject {
        kind,
        name,
        form_id,
        plugin,
        mesh,
    })
}

/// The texts in double quotes or backticks in a line, in order.
fn quoted_texts(line: &str) -> Vec<&str> {
    let mut texts = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find(['"', '`']) {
        let quote = rest.get(start..=start).unwrap_or("\"");
        let Some(after) = rest.get(start + 1..) else {
            break;
        };
        let Some(end) = after.find(quote) else {
            break;
        };
        if let Some(text) = after.get(..end) {
            texts.push(text);
        }
        rest = after.get(end + 1..).unwrap_or("");
    }
    texts
}

/// Whether `text` is the file name of a plugin.
fn is_plugin(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    !lower.contains(['\\', '/'])
        && [".esp", ".esm", ".esl"]
            .iter()
            .any(|extension| lower.ends_with(extension))
}

/// The text between the first `open` and the next `close` after it.
fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let rest = text.get(start..)?;
    let end = rest.find(close)?;
    rest.get(..end)
}

/// The time in a report's file name, as `YYYY-MM-DD HH:MM`: the first five
/// numbers in it are year, month, day, hour and minute.
fn time_from_name(file: &str) -> Option<String> {
    let numbers: Vec<u32> = file
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();
    let [year, month, day, hour, minute, ..] = numbers.as_slice() else {
        return None;
    };
    let plausible = (2000..=2999).contains(year)
        && (1..=12).contains(month)
        && (1..=31).contains(day)
        && *hour < 24
        && *minute < 60;
    plausible.then(|| format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}"))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A report in the shape of a newer Crash Logger: made-up names and
    /// numbers, real line shapes.
    const CRASH_LOGGER: &str = "\
CRASH TIME: 2026-01-02 03:04:05
Skyrim SSE v1.6.1170
CrashLoggerSSE v1-20-0-0 Jan  1 2026 00:00:00

Unhandled exception \"EXCEPTION_ACCESS_VIOLATION\" at 0x7FF700112233 SkyrimSE.exe+0112233\tmov eax, [rcx+0x10]
Exception Flags: 0x00000000

POSSIBLE RELEVANT OBJECTS:
\tRSP+10: (Character*) \"Farmer Tom\" [0x00012345] (\"Busy Farms.esp\")
\tRSP+48: (TESPackage*) [0x0A001234] (\"Busy Farms.esp\")
\tRSP+90: (BSFadeNode*) \"meshes\\farm\\haystack.nif\"
\tRDI: (PathingRequest*)

CALL STACK ([P]robable / [S]tack scan):
\t[ 0][P] 0x7FF700112233    SkyrimSE.exe+0112233 -> 12345+0x10\tmov eax, [rcx+0x10]
\t[ 1][P] 0x7FF700445566    SkyrimSE.exe+0445566 -> 23456+0x20\ttest al, al
\t[ 2][P] 0x7FFD00001111 Busy Farm Helper.dll+0001111\tcall rax
\t[ 3][S] 0x7FFD00002222    Unrelated.dll+0002222\tnop

REGISTERS:
\tRAX 0x0                (size_t) [0]

STACK:
\t[RSP+10 ] 0x1E200000000      (Character*) \"Farmer Tom\" [0x00012345] (\"Busy Farms.esp\")
\t\tModified by: Skyrim.esm -> Busy Farms.esp -> Other.esp

MODULES:
\tSkyrimSE.exe                                       0x7FF700000000

PLUGINS:
\t[ 0]     Skyrim.esm
";

    /// A report in the shape of the .NET Script Framework.
    const NET_SCRIPT_FRAMEWORK: &str = "\
Unhandled native exception occurred at 0x7FF700AABBCC (SkyrimSE.exe+AABBCC) on thread 42!

FrameworkName: NetScriptFramework

Possible relevant objects (2)
{
  [  10]   Character(Name: `Guard`, FormId: 0001A2B3, File: `Skyrim.esm <- City Guards.esp`)
  [  38]   NiNode(Name: `meshes\\city\\gate.nif`)
}

Probable callstack
{
  [0]   0x7FFD00334455   (Guard Brain.dll+334455)
  [1]   0x7FF700AABBCC   (SkyrimSE.exe+AABBCC)
}
";

    /// A report in the shape of Trainwreck, without a list of objects.
    const TRAINWRECK: &str = "\
Skyrim SSE v1.6.640
Trainwreck v1.0.0

Unhandled exception \"EXCEPTION_STACK_OVERFLOW\" at 0x7FFD00ABCDEF Physics Extra.dll+00ABCDEF

PROBABLE CALL STACK:
\t[0] 0x7FFD00ABCDEF Physics Extra.dll+00ABCDEF
\t[1] 0x7FF700001000 SkyrimSE.exe+0001000

REGISTERS:
\tRAX 0x1E200000000      (TESObjectREFR*) \"Rock\" [0x0B000ABC] (\"Big Rocks.esp\")
\tRBX 0x0                (size_t) [0]

STACK:
\t[RSP+0  ] 0x0                (size_t) [0]
";

    #[test]
    fn a_crash_logger_report_is_read() {
        let log = parse(
            CrashLogger::CrashLogger,
            "crash-2026-01-02-03-04-05.log",
            CRASH_LOGGER,
        );
        assert_eq!(log.logger, CrashLogger::CrashLogger);
        assert_eq!(log.time.as_deref(), Some("2026-01-02 03:04"));
        assert_eq!(log.exception.as_deref(), Some("EXCEPTION_ACCESS_VIOLATION"));
        assert_eq!(log.address.as_deref(), Some("SkyrimSE.exe+0112233"));
        assert_eq!(log.module(), Some("SkyrimSE.exe"));
        // The frame found by scanning the stack is left out.
        assert_eq!(log.stack_modules, ["SkyrimSE.exe", "Busy Farm Helper.dll"]);

        let plugins: Vec<Option<&str>> = log
            .objects
            .iter()
            .map(|object| object.plugin.as_deref())
            .collect();
        assert_eq!(
            plugins,
            [Some("Busy Farms.esp"), Some("Busy Farms.esp"), None, None]
        );
        let farmer = &log.objects[0];
        assert_eq!(farmer.kind.as_deref(), Some("Character"));
        assert_eq!(farmer.name.as_deref(), Some("Farmer Tom"));
        assert_eq!(farmer.form_id.as_deref(), Some("0x00012345"));
        assert_eq!(
            log.objects[2].mesh.as_deref(),
            Some(r"meshes\farm\haystack.nif")
        );
    }

    /// A report in the shape of Buffout 4, the crash logger of Fallout 4,
    /// which Crash Logger grew out of: made-up names and numbers.
    const BUFFOUT_4: &str = "\nFallout 4 v1.10.163
Buffout 4 v1.26.2

Unhandled exception \"EXCEPTION_ACCESS_VIOLATION\" at 0x7FF600AB1234 Fallout4.exe+0AB1234	mov rax, [rcx]

[Compatibility]
	F4EE: false

PROBABLE CALL STACK:
	[0] 0x7FF600AB1234 Fallout4.exe+0AB1234
	[1] 0x7FFD00C0FFEE Settlement Helper.dll+000FFEE

REGISTERS:
	RAX 0x1E200000000      (TESObjectREFR*) \"Wall\" [0x01001234] (\"Bigger Settlements.esp\")

STACK:
	[RSP+0  ] 0x0                (size_t) [0]
";

    #[test]
    fn a_buffout_4_report_of_fallout_4_is_read_like_crash_loggers() {
        let log = parse(
            CrashLogger::CrashLogger,
            "crash-2026-01-02-03-04-05.log",
            BUFFOUT_4,
        );
        assert_eq!(log.exception.as_deref(), Some("EXCEPTION_ACCESS_VIOLATION"));
        assert_eq!(log.module(), Some("Fallout4.exe"));
        assert_eq!(log.stack_modules, ["Fallout4.exe", "Settlement Helper.dll"]);
        assert!(is_report_name(
            CrashLogger::CrashLogger,
            &"Crash-2026-01-02-03-04-05.log".to_ascii_lowercase()
        ));
    }

    #[test]
    fn a_net_script_framework_report_is_read() {
        let log = parse(
            CrashLogger::NetScriptFramework,
            "Crash_2026_1_2_3-04-05.txt",
            NET_SCRIPT_FRAMEWORK,
        );
        assert_eq!(log.logger, CrashLogger::NetScriptFramework);
        assert_eq!(log.time.as_deref(), Some("2026-01-02 03:04"));
        assert_eq!(log.address.as_deref(), Some("SkyrimSE.exe+AABBCC"));
        assert_eq!(log.stack_modules, ["Guard Brain.dll", "SkyrimSE.exe"]);
        let guard = &log.objects[0];
        assert_eq!(guard.kind.as_deref(), Some("Character"));
        assert_eq!(guard.name.as_deref(), Some("Guard"));
        assert_eq!(guard.form_id.as_deref(), Some("0001A2B3"));
        assert_eq!(guard.plugin.as_deref(), Some("City Guards.esp"));
        assert_eq!(
            log.objects[1].mesh.as_deref(),
            Some(r"meshes\city\gate.nif")
        );
    }

    #[test]
    fn a_trainwreck_report_takes_its_objects_from_the_registers() {
        let log = parse(
            CrashLogger::Trainwreck,
            "crash-2026-01-02-03-04-05.log",
            TRAINWRECK,
        );
        assert_eq!(log.logger, CrashLogger::Trainwreck);
        assert_eq!(log.exception.as_deref(), Some("EXCEPTION_STACK_OVERFLOW"));
        assert_eq!(log.address.as_deref(), Some("Physics Extra.dll+00ABCDEF"));
        assert_eq!(log.module(), Some("Physics Extra.dll"));
        assert_eq!(log.stack_modules, ["Physics Extra.dll", "SkyrimSE.exe"]);
        let [rock] = log.objects.as_slice() else {
            panic!("expected one object, got {:?}", log.objects);
        };
        assert_eq!(rock.plugin.as_deref(), Some("Big Rocks.esp"));
        assert_eq!(rock.name.as_deref(), Some("Rock"));
    }

    #[test]
    fn report_names_are_told_apart_from_other_logs() {
        assert!(is_report_name(
            CrashLogger::CrashLogger,
            "crash-2026-01-02-03-04-05.log"
        ));
        assert!(!is_report_name(CrashLogger::CrashLogger, "crashlogger.log"));
        assert!(is_report_name(
            CrashLogger::NetScriptFramework,
            "crash_2026_1_2_3-04-05.txt"
        ));
        assert!(!is_report_name(
            CrashLogger::NetScriptFramework,
            "crash-1.log"
        ));
    }

    #[test]
    fn only_recent_reports_are_read_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, CRASH_LOGGER).unwrap();
            path
        };
        write("crash-2026-01-01-00-00-00.log");
        write("crash-2026-01-02-00-00-00.log");
        write("CrashLogger.log");
        let folders = [(dir.path().to_path_buf(), CrashLogger::CrashLogger)];

        let now = SystemTime::now();
        let facts = read(&folders, now);
        assert_eq!(facts.window_days, WINDOW_DAYS);
        assert_eq!(facts.logs.len(), 2);

        let much_later = now + Duration::from_secs(u64::from(WINDOW_DAYS + 1) * 24 * 60 * 60);
        assert_eq!(read(&folders, much_later).logs, Vec::new());
    }

    #[test]
    fn times_in_names_must_be_plausible() {
        assert_eq!(time_from_name("crash-1-2.log"), None);
        assert_eq!(time_from_name("crash-2026-13-01-00-00-00.log"), None);
    }

    proptest! {
        #[test]
        fn never_panics_on_any_text(text in "(.|\n){0,400}") {
            let _ = parse(CrashLogger::CrashLogger, "crash.log", &text);
        }

        #[test]
        fn never_panics_on_a_damaged_report(cut in 0usize..1200, insert in ".{0,8}") {
            let mut text = CRASH_LOGGER.to_owned();
            let at = (0..=cut.min(text.len())).rev().find(|at| text.is_char_boundary(*at)).unwrap_or(0);
            text.insert_str(at, &insert);
            let _ = parse(CrashLogger::CrashLogger, "crash.log", &text);
            let _ = parse(CrashLogger::NetScriptFramework, "Crash_x.txt", NET_SCRIPT_FRAMEWORK.get(..cut.min(NET_SCRIPT_FRAMEWORK.len())).unwrap_or(""));
        }
    }
}
