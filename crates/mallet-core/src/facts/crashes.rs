//! What the crash loggers wrote when the game crashed.
//!
//! A crash logger is a script extender plugin that writes a report when
//! the game crashes: where in which program file it happened, the call
//! stack that led there, and the game objects it found nearby. Which of
//! them caused the crash is a guess, even for people who read such logs
//! every day (ADR 0044).

/// The recent crashes of the game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrashFacts {
    /// The crash reports that were read, newest first.
    pub logs: Vec<CrashLog>,
    /// How many days back reports were read.
    pub window_days: u32,
}

/// The crash logger that wrote a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashLogger {
    /// Crash Logger SSE AE VR (`crash-*.log` in the script extender's
    /// folder).
    CrashLogger,
    /// Trainwreck (`crash-*.log` in `Crashlogs` below the script extender's
    /// folder).
    Trainwreck,
    /// .NET Script Framework (`Crash_*.txt` in `NetScriptFramework/Crash`
    /// below the data directory).
    NetScriptFramework,
}

/// One crash report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashLog {
    /// The file name of the report.
    pub file: String,
    /// Which crash logger wrote it.
    pub logger: CrashLogger,
    /// When the game crashed, as `YYYY-MM-DD HH:MM`, if the report says.
    pub time: Option<String>,
    /// The kind of failure, such as `EXCEPTION_ACCESS_VIOLATION`.
    pub exception: Option<String>,
    /// Where the crash happened: a program file and an offset in it, such
    /// as `SkyrimSE.exe+11CC0B2`.
    pub address: Option<String>,
    /// The program files of the probable call stack, from the crash
    /// outwards, each once.
    pub stack_modules: Vec<String>,
    /// The game objects the logger found near the crash.
    pub objects: Vec<CrashObject>,
}

impl CrashLog {
    /// The program file the crash happened in, such as `SkyrimSE.exe`.
    pub fn module(&self) -> Option<&str> {
        let address = self.address.as_deref()?;
        address.rsplit_once('+').map(|(module, _)| module)
    }
}

/// A game object a crash logger found near a crash. Each part is as the
/// logger wrote it, and any of them may be missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrashObject {
    /// The type of the object, such as `Character` or `BSFadeNode`.
    pub kind: Option<String>,
    /// Its name, such as the name of a character or a node.
    pub name: Option<String>,
    /// Its form ID as the logger wrote it. It depends on the load order and
    /// is only shown, never compared.
    pub form_id: Option<String>,
    /// The plugin the logger attributes it to.
    pub plugin: Option<String>,
    /// The mesh it comes from, as the logger wrote the path.
    pub mesh: Option<String>,
}
