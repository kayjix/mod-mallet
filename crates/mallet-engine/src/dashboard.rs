//! Telling the Vortex extension where Mod Mallet runs from, so that it can
//! list Mod Mallet among the tools on Vortex's dashboard (ADR 0063).

use std::path::Path;

use crate::ManagerOptions;

/// The picture Vortex shows for the tool: Mod Mallet's own icon.
const ICON: &[u8] = include_bytes!("../../../apps/desktop/src-tauri/icons/128x128.png");

/// The name of the note in the folder shared with the extension.
const NOTE_FILE: &str = "app.json";
/// The name of the icon in the folder shared with the extension.
const ICON_FILE: &str = "app-icon.png";

/// Leaves a note in the folder shared with the Vortex extension saying
/// where this Mod Mallet is and which version it is, with its icon beside
/// it. Nothing is written if the folder is not there, which means the
/// extension never ran on this machine, and nothing if the note already
/// says the same. The folder is Mod Mallet's own; Vortex's data is not
/// touched.
///
/// # Errors
///
/// If the program's own path is not known or a file cannot be written.
pub fn announce_app(options: &ManagerOptions) -> std::io::Result<()> {
    let Some(dir) = options.link_dir() else {
        return Ok(());
    };
    let exe = std::env::current_exe()?;
    write_note(&dir, &exe, env!("CARGO_PKG_VERSION"))
}

fn write_note(dir: &Path, exe: &Path, version: &str) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let note = serde_json::json!({
        "name": "Mod Mallet",
        "version": version,
        "exe": exe.display().to_string(),
        "icon": ICON_FILE,
    })
    .to_string();
    let note_path = dir.join(NOTE_FILE);
    if std::fs::read_to_string(&note_path).is_ok_and(|found| found == note) {
        return Ok(());
    }
    std::fs::write(dir.join(ICON_FILE), ICON)?;
    // The note is written last and whole, so that the extension never sees
    // a note without its icon.
    let temporary = dir.join(format!("{NOTE_FILE}.tmp"));
    std::fs::write(&temporary, note)?;
    std::fs::rename(temporary, note_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_is_left_only_where_the_link_has_a_folder() {
        let parent = tempfile::tempdir().unwrap();
        let folder = parent.path().join("vortex-bridge");
        let exe = Path::new(r"C:\Made Up\mod-mallet.exe");

        write_note(&folder, exe, "1.2.3").unwrap();
        assert!(!folder.exists());

        std::fs::create_dir(&folder).unwrap();
        write_note(&folder, exe, "1.2.3").unwrap();
        let note: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(NOTE_FILE)).unwrap())
                .unwrap();
        assert_eq!(note["version"], "1.2.3");
        assert_eq!(note["exe"], r"C:\Made Up\mod-mallet.exe");
        assert!(folder.join(ICON_FILE).is_file());

        // A new version replaces the note.
        write_note(&folder, exe, "1.3.0").unwrap();
        let text = std::fs::read_to_string(folder.join(NOTE_FILE)).unwrap();
        assert!(text.contains("1.3.0"));
    }
}
