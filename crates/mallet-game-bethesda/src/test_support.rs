//! What the tests of several readers share.

use std::path::PathBuf;

/// Writes each of `plugins` (file name, content) into a new temporary
/// folder, and lists them by name and file in the same order, as the
/// readers take the active plugins of a setup.
pub(crate) fn plugin_files(
    plugins: &[(&str, Vec<u8>)],
) -> (tempfile::TempDir, Vec<(String, PathBuf)>) {
    let folder = tempfile::tempdir().unwrap();
    let list = plugins
        .iter()
        .map(|(name, bytes)| {
            let path = folder.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            ((*name).to_owned(), path)
        })
        .collect();
    (folder, list)
}
