//! Looks into mod archives.
//!
//! Mods are downloaded as zip, 7z and rar archives. Mod Mallet needs single
//! small files out of them, such as the installer description. Instead of
//! bringing three decompression libraries (and, for rar, one whose license
//! does not fit, see ADR 0002), it uses a 7-Zip that is already on the
//! machine: the one the Vortex mod manager ships, or an installed 7-Zip.
//! That follows the project's rule to reuse the tools people already have.
//!
//! [`Unpacker::locate`] finds the program by a fixed list of places and
//! never searches the disk (ADR 0018). The archive is only read; nothing is
//! unpacked to disk.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where Vortex keeps its bundled 7-Zip, below its installation directory.
const VORTEX_7ZIP: &str = r"resources\app.asar.unpacked\node_modules\7z-bin\win32\7z.exe";

/// Names 7-Zip goes by on systems where it is found through `PATH`.
const PATH_NAMES: [&str; 3] = ["7z", "7zz", "7za"];

/// Why an archive could not be looked into.
#[derive(Debug, thiserror::Error)]
pub enum UnpackError {
    /// The unpacker program could not be started.
    #[error("`{}` could not be started: {source}", .program.display())]
    Start {
        /// The program that was to be started.
        program: PathBuf,
        /// What the system reported.
        source: std::io::Error,
    },
    /// The unpacker reported a problem with the archive.
    #[error("the archive `{}` could not be read: {message}", .archive.display())]
    Archive {
        /// The archive.
        archive: PathBuf,
        /// What the unpacker said.
        message: String,
    },
}

/// A 7-Zip program that reads archives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpacker {
    program: PathBuf,
}

impl Unpacker {
    /// Uses the 7-Zip program at `program`.
    pub fn at(program: PathBuf) -> Self {
        Self { program }
    }

    /// Finds a 7-Zip on this machine: first the one Vortex ships, then an
    /// installed 7-Zip, then one on `PATH`.
    pub fn locate() -> Option<Self> {
        candidates()
            .into_iter()
            .find(|candidate| candidate.is_file())
            .map(Self::at)
    }

    /// The program this unpacker runs.
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// The paths of the files in `archive`.
    pub fn list(&self, archive: &Path) -> Result<Vec<String>, UnpackError> {
        // `-slt` prints one `Path = ...` line per entry, `-ba` leaves out
        // the banner, `-sccUTF-8` makes names outside ASCII come through.
        let output = self.run(archive, &["l", "-ba", "-slt", "-sccUTF-8"], None)?;
        Ok(parse_listing(&String::from_utf8_lossy(&output)))
    }

    /// The content of the file `inner_path` in `archive`.
    pub fn read(&self, archive: &Path, inner_path: &str) -> Result<Vec<u8>, UnpackError> {
        // `e -so` writes the file to standard output instead of to disk.
        self.run(archive, &["e", "-so", "-bd", "-y"], Some(inner_path))
    }

    /// The content of the file in `archive` whose path ends in `suffix`,
    /// compared without regard to case or the kind of slash. If several
    /// match, the one closest to the top of the archive is taken. `None`
    /// if there is no such file.
    pub fn read_by_suffix(
        &self,
        archive: &Path,
        suffix: &str,
    ) -> Result<Option<Vec<u8>>, UnpackError> {
        let listing = self.list(archive)?;
        match find_by_suffix(&listing, suffix) {
            Some(inner_path) => self.read(archive, inner_path).map(Some),
            None => Ok(None),
        }
    }

    fn run(
        &self,
        archive: &Path,
        arguments: &[&str],
        inner_path: Option<&str>,
    ) -> Result<Vec<u8>, UnpackError> {
        let mut command = Command::new(&self.program);
        command
            .args(arguments)
            .arg("--")
            .arg(archive)
            .args(inner_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_window(&mut command);

        let output = command.output().map_err(|source| UnpackError::Start {
            program: self.program.clone(),
            source,
        })?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            let message = String::from_utf8_lossy(&output.stderr);
            let message = message
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("the unpacker gave no reason")
                .to_owned();
            Err(UnpackError::Archive {
                archive: archive.to_path_buf(),
                message,
            })
        }
    }
}

/// The paths in the output of `7z l -slt`.
pub fn parse_listing(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.trim_end().strip_prefix("Path = "))
        .map(str::to_owned)
        .collect()
}

/// The path among `paths` that ends in `suffix` as a whole path or after a
/// folder, compared without regard to case or the kind of slash. The one
/// with the fewest folders above it wins.
pub fn find_by_suffix<'a>(paths: &'a [String], suffix: &str) -> Option<&'a str> {
    let normalize = |path: &str| path.replace('\\', "/").to_lowercase();
    let suffix = normalize(suffix);

    paths
        .iter()
        .filter(|path| {
            let path = normalize(path);
            path == suffix || path.ends_with(&format!("/{suffix}"))
        })
        .min_by_key(|path| path.matches(['/', '\\']).count())
        .map(String::as_str)
}

/// The fixed places where a 7-Zip is looked for, best first.
fn candidates() -> Vec<PathBuf> {
    let directory = |variable: &str| std::env::var_os(variable).map(PathBuf::from);
    let mut candidates = Vec::new();

    // Vortex, installed for everyone or for one user.
    for program_files in ["ProgramFiles", "ProgramW6432"] {
        if let Some(base) = directory(program_files) {
            candidates.push(base.join("Vortex").join(VORTEX_7ZIP));
        }
    }
    if let Some(base) = directory("LOCALAPPDATA") {
        candidates.push(base.join("Programs").join("Vortex").join(VORTEX_7ZIP));
    }

    // 7-Zip itself.
    for program_files in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Some(base) = directory(program_files) {
            candidates.push(base.join("7-Zip").join("7z.exe"));
        }
    }

    // Anything on PATH.
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            for name in PATH_NAMES {
                candidates.push(directory.join(name));
                candidates.push(directory.join(format!("{name}.exe")));
            }
        }
    }
    candidates
}

/// Keeps a console window from flashing up when the desktop app starts the
/// unpacker.
#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|&path| path.to_owned()).collect()
    }

    #[test]
    fn reads_the_paths_from_a_listing() {
        let output = "\
Path = fomod\r
Folder = +\r
Size = 0\r
\r
Path = fomod\\ModuleConfig.xml\r
Folder = -\r
Size = 1234\r
\r
Path = Data\\Größe.esp\r
";
        assert_eq!(
            parse_listing(output),
            ["fomod", r"fomod\ModuleConfig.xml", r"Data\Größe.esp"]
        );
        assert_eq!(parse_listing(""), Vec::<String>::new());
    }

    #[test]
    fn finds_a_file_regardless_of_case_and_slashes() {
        let listing = paths(&[r"FOMod\ModuleConfig.xml", r"Data\Mod.esp"]);
        assert_eq!(
            find_by_suffix(&listing, "fomod/moduleconfig.xml"),
            Some(r"FOMod\ModuleConfig.xml")
        );
    }

    #[test]
    fn finds_a_file_below_a_top_level_folder() {
        let listing = paths(&["My Mod 1.0/fomod/ModuleConfig.xml"]);
        assert_eq!(
            find_by_suffix(&listing, "fomod/moduleconfig.xml"),
            Some("My Mod 1.0/fomod/ModuleConfig.xml")
        );
    }

    #[test]
    fn prefers_the_file_closest_to_the_top() {
        let listing = paths(&[
            "extras/other mod/fomod/ModuleConfig.xml",
            "fomod/ModuleConfig.xml",
        ]);
        assert_eq!(
            find_by_suffix(&listing, "fomod/moduleconfig.xml"),
            Some("fomod/ModuleConfig.xml")
        );
    }

    #[test]
    fn a_name_that_merely_ends_alike_does_not_match() {
        let listing = paths(&["notfomod/ModuleConfig.xml", "fomod/ModuleConfig.xml.bak"]);
        assert_eq!(find_by_suffix(&listing, "fomod/moduleconfig.xml"), None);
    }

    /// Runs only where a 7-Zip is present, which includes the Windows
    /// machines of CI. Elsewhere there is nothing to test against.
    #[test]
    fn reads_a_file_out_of_a_real_archive() {
        let Some(unpacker) = Unpacker::locate() else {
            eprintln!("no 7-Zip found, skipping");
            return;
        };
        let directory = tempfile::tempdir().unwrap();
        let fomod = directory.path().join("My Mod").join("FOMod");
        std::fs::create_dir_all(&fomod).unwrap();
        std::fs::write(fomod.join("ModuleConfig.xml"), "<config/>").unwrap();
        std::fs::write(directory.path().join("My Mod").join("readme.txt"), "hello").unwrap();

        let archive = directory.path().join("mod.7z");
        let created = Command::new(unpacker.program())
            .arg("a")
            .arg(&archive)
            .arg(directory.path().join("My Mod"))
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(created.success());

        let listing = unpacker.list(&archive).unwrap();
        assert!(listing.iter().any(|path| path.ends_with("readme.txt")));
        assert_eq!(
            unpacker
                .read_by_suffix(&archive, "fomod/moduleconfig.xml")
                .unwrap(),
            Some(b"<config/>".to_vec())
        );
        assert_eq!(
            unpacker
                .read_by_suffix(&archive, "fomod/missing.xml")
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_file_that_is_not_an_archive_is_an_error() {
        let Some(unpacker) = Unpacker::locate() else {
            eprintln!("no 7-Zip found, skipping");
            return;
        };
        let directory = tempfile::tempdir().unwrap();
        let not_an_archive = directory.path().join("text.7z");
        std::fs::write(&not_an_archive, "just some text").unwrap();
        assert!(matches!(
            unpacker.list(&not_an_archive),
            Err(UnpackError::Archive { .. })
        ));
    }

    #[test]
    fn a_missing_program_is_an_error() {
        let unpacker = Unpacker::at(PathBuf::from("no-such-program-anywhere"));
        assert!(matches!(
            unpacker.list(Path::new("archive.7z")),
            Err(UnpackError::Start { .. })
        ));
    }
}
