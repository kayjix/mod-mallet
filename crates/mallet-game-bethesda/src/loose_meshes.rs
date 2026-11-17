//! What the loose meshes say about themselves: which variant of the format
//! they are in, and which textures make up their surfaces.
//!
//! Every game reads the mesh format in its own variant, numbered by
//! Bethesda: Skyrim (2011) writes 83, Skyrim Special Edition 100. A mesh
//! taken over from a mod for the old Skyrim without converting it can make
//! the new one crash or show nothing ([`nif::variant`]).
//!
//! The surface of a mesh is drawn from a colour map and a normal map, the
//! first two slots of each texture set ([`nif::read_surface_textures`]).
//!
//! Only the start and the texture blocks of each mesh are read. Opening
//! thousands of meshes still takes seconds, so what was read is kept in
//! the cache under the size and date of each file, and a mesh is opened
//! again only when it changed.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use mallet_core::facts::{ForeignMesh, MeshTextures};
use mallet_core::{GamePath, TextCache};
use mallet_formats_bethesda::nif::{self, NifError};

use crate::game::Game;
use crate::meshes::{file_state, game_paths};

/// Under which name what a loose mesh says is kept in the cache.
const CACHE_NAMESPACE: &str = "loose-meshes";

/// Where the game looks for meshes, as the game sees the path.
const MESHES_FOLDER: &str = "data/meshes/";

/// How much of a mesh is read at first for its variant; its header nearly
/// always fits.
const FIRST_READ: u64 = 4 * 1024;

/// How much is read when the header did not fit.
const LONGER_READ: u64 = 64 * 1024;

/// What the loose meshes say about themselves.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LooseMeshes {
    /// The meshes in a variant of the format the game does not read.
    pub foreign: Vec<ForeignMesh>,
    /// The surface textures of every mesh that could be read.
    pub textures: Vec<MeshTextures>,
}

/// The variant of the mesh format `game` reads, or `None` for a game whose
/// meshes do not say (Oblivion).
pub fn native_version(game: Game) -> Option<u32> {
    match game {
        Game::SkyrimSe | Game::SkyrimVr => Some(100),
        Game::Skyrim => Some(83),
        Game::Fallout4 | Game::Fallout4Vr => Some(130),
        Game::FalloutNv | Game::Fallout3 => Some(34),
        Game::Oblivion => None,
    }
}

/// Reads the meshes among `loose` files (the path the game sees, and the
/// file) of `game`. Only the meshes folder counts; the game loads no mesh
/// from anywhere else. Meshes that cannot be read are left out.
pub fn read(game: Game, loose: &[(GamePath, PathBuf)], cache: &dyn TextCache) -> LooseMeshes {
    let native = native_version(game);
    let mut meshes = LooseMeshes::default();
    let candidates = loose.iter().filter(|(path, file)| {
        path.as_str().starts_with(MESHES_FOLDER)
            && file
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("nif"))
    });
    for (path, file) in candidates {
        let state = file_state(file);
        let cached = state
            .as_deref()
            .and_then(|state| cache.get(CACHE_NAMESPACE, path.as_str(), state))
            .and_then(|text| Entry::from_text(&text));
        let entry = if let Some(entry) = cached {
            entry
        } else {
            let Some(entry) = Entry::read(file) else {
                continue;
            };
            if let Some(state) = &state {
                cache.put(CACHE_NAMESPACE, path.as_str(), state, &entry.to_text());
            }
            entry
        };
        if let (Some(native), Some(variant)) = (native, entry.variant)
            && variant.bethesda_version != native
        {
            meshes.foreign.push(ForeignMesh {
                path: path.clone(),
                bethesda_version: variant.bethesda_version,
                skinned: variant.skinned,
            });
        }
        meshes.textures.push(MeshTextures {
            path: path.clone(),
            textures: entry.textures,
        });
    }
    meshes
}

/// What one mesh says about itself.
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    /// Its variant, if its header could be read.
    variant: Option<nif::Variant>,
    /// Its surface textures.
    textures: Vec<GamePath>,
}

impl Entry {
    /// Reads the mesh in `file`; `None` if nothing in it can be read.
    fn read(file: &Path) -> Option<Self> {
        let mut reader = BufReader::new(File::open(file).ok()?);
        let variant = read_variant(&mut reader);
        reader.seek(SeekFrom::Start(0)).ok()?;
        let textures = nif::read_surface_textures(&mut reader).ok().map(|raw| {
            // A slot that names a folder and only blanks names nothing.
            game_paths(&raw)
                .into_iter()
                .filter(|path| !path.file_name().trim().is_empty())
                .collect()
        });
        if variant.is_none() && textures.is_none() {
            return None;
        }
        Some(Self {
            variant,
            textures: textures.unwrap_or_default(),
        })
    }

    /// The entry as kept in the cache: the variant on the first line
    /// (`-` if unknown), then one texture per line.
    fn to_text(&self) -> String {
        let variant = self.variant.map_or_else(
            || "-".to_owned(),
            |variant| format!("{} {}", variant.bethesda_version, u8::from(variant.skinned)),
        );
        std::iter::once(variant.as_str())
            .chain(self.textures.iter().map(GamePath::as_str))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The entry from the cache; `None` if the text is not one.
    fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let first = lines.next()?;
        let variant = if first == "-" {
            None
        } else {
            let (version, skinned) = first.split_once(' ')?;
            Some(nif::Variant {
                bethesda_version: version.parse().ok()?,
                skinned: skinned == "1",
            })
        };
        Some(Self {
            variant,
            textures: lines.map(GamePath::new).collect(),
        })
    }
}

/// The variant of the mesh `reader` is at the start of, reading no more of
/// it than needed.
fn read_variant<R: Read>(reader: &mut R) -> Option<nif::Variant> {
    let mut start = Vec::new();
    reader
        .by_ref()
        .take(FIRST_READ)
        .read_to_end(&mut start)
        .ok()?;
    match nif::variant(&start) {
        Err(NifError::Damaged) if u64::try_from(start.len()).ok() == Some(FIRST_READ) => {
            reader
                .take(LONGER_READ - FIRST_READ)
                .read_to_end(&mut start)
                .ok()?;
            nif::variant(&start).ok()
        }
        result => result.ok(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mallet_core::NoCache;
    use mallet_formats_bethesda::fixture;
    use proptest::prelude::*;

    use super::*;

    /// A mesh with one surface in the variant `version`: a colour map, a
    /// normal map and a glow map, and a set of blank slots.
    fn mesh(version: u32) -> Vec<u8> {
        let mut mesh = fixture::nif(&[
            &[
                r"Data\Textures\Wall.dds",
                r"textures\wall_n.dds",
                r"textures\wall_g.dds",
            ],
            &[r"textures\ ", ""],
        ]);
        let after_line = mesh.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        let at = after_line + 4 + 1 + 4 + 4;
        mesh[at..at + 4].copy_from_slice(&version.to_le_bytes());
        mesh
    }

    fn loose(dir: &Path, files: &[(&str, &[u8])]) -> Vec<(GamePath, PathBuf)> {
        files
            .iter()
            .map(|(path, data)| {
                let file = dir.join(path.replace('/', "_"));
                fs::write(&file, data).unwrap();
                (GamePath::new(path), file)
            })
            .collect()
    }

    #[test]
    fn meshes_of_the_old_skyrim_and_surface_textures_are_found() {
        let dir = tempfile::tempdir().unwrap();
        let loose = loose(
            dir.path(),
            &[
                ("data/meshes/old.nif", &mesh(83)),
                ("data/meshes/new.nif", &mesh(100)),
                ("data/meshes/broken.nif", b"not a mesh"),
                ("data/meshes/readme.txt", &mesh(83)),
                // A mesh a tool keeps for itself, outside the meshes folder.
                ("data/edit scripts/lod.nif", &mesh(83)),
            ],
        );

        let meshes = read(Game::SkyrimSe, &loose, &NoCache);
        assert_eq!(
            meshes.foreign,
            vec![ForeignMesh {
                path: GamePath::new("data/meshes/old.nif"),
                bethesda_version: 83,
                skinned: false,
            }]
        );
        let wall = vec![
            GamePath::new("data/textures/wall.dds"),
            GamePath::new("data/textures/wall_n.dds"),
        ];
        assert_eq!(
            meshes.textures,
            vec![
                MeshTextures {
                    path: GamePath::new("data/meshes/old.nif"),
                    textures: wall.clone(),
                },
                MeshTextures {
                    path: GamePath::new("data/meshes/new.nif"),
                    textures: wall,
                },
            ]
        );

        // In the old Skyrim it is the other way round; Oblivion's meshes
        // do not say.
        let old = read(Game::Skyrim, &loose, &NoCache).foreign;
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].path, GamePath::new("data/meshes/new.nif"));
        assert_eq!(read(Game::Oblivion, &loose, &NoCache).foreign, Vec::new());
    }

    #[test]
    fn an_entry_survives_the_cache() {
        let entry = Entry {
            variant: Some(nif::Variant {
                bethesda_version: 83,
                skinned: true,
            }),
            textures: vec![GamePath::new("data/textures/a.dds")],
        };
        assert_eq!(Entry::from_text(&entry.to_text()), Some(entry));
        let unknown = Entry {
            variant: None,
            textures: Vec::new(),
        };
        assert_eq!(Entry::from_text(&unknown.to_text()), Some(unknown));
    }

    proptest! {
        #[test]
        fn never_panics_on_any_cache_text(text in ".{0,64}") {
            let _ = Entry::from_text(&text);
        }
    }
}
