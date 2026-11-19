//! Pictures of the files a finding is about, for the window: every version
//! of a texture that the setup has, small enough to show side by side.

use std::path::Path;

use mallet_core::{FilePreview, GamePath, ModSource, NoModSource, PreviewVersion};
use mallet_game_bethesda::files::{self, Origin};
use mallet_source_vortex::VortexDeployment;

use crate::ScanOptions;

/// The largest width or height of a picture.
const MAX_SIDE: u32 = 256;

/// Every version of the file at `path` (below the data directory) that the
/// setup has, as pictures where the file is a texture that can be shown;
/// the version the game uses comes last. Only reads.
pub fn file_preview(options: &ScanOptions, path: &str) -> Option<FilePreview> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let found = files::versions(game, &game_dir, options.local_dir.as_deref(), path);
    let mods = deployment(options, &game_dir);
    let inside = |name: &str| GamePath::new(&format!("data/{name}"));
    let versions = found
        .into_iter()
        .map(|version| {
            let (archive, provided_by) = match version.origin {
                Origin::Loose => (None, inside(path.trim_start_matches("data/"))),
                Origin::Archive(name) => {
                    let provided_by = inside(&name);
                    (Some(name), provided_by)
                }
            };
            PreviewVersion {
                archive,
                provider: mods.provider_of(&provided_by),
                image: picture(&version.bytes),
            }
        })
        .collect();
    Some(FilePreview {
        path: path.to_owned(),
        versions,
    })
}

/// Which mod deployed which file, if Vortex says so; otherwise nothing.
pub(crate) fn deployment(options: &ScanOptions, game_dir: &Path) -> Box<dyn ModSource> {
    let manifest_path = options
        .deployment_manifest
        .clone()
        .unwrap_or_else(|| mallet_source_vortex::data_manifest_path(game_dir));
    std::fs::read_to_string(manifest_path)
        .ok()
        .and_then(|text| VortexDeployment::parse(&text, &game_dir.to_string_lossy()).ok())
        .map_or_else(
            || Box::new(NoModSource) as Box<dyn ModSource>,
            |deployment| Box::new(deployment),
        )
}

/// A texture as a small PNG, from the largest mipmap level that fits.
fn picture(bytes: &[u8]) -> Option<String> {
    let image = files::thumbnail(bytes, MAX_SIDE)?;
    let png = crate::png::encode_rgb(image.width, image.height, &image.pixels).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        mallet_core::encoding::base64(&png)
    ))
}
