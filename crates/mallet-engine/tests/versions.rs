//! The versions of a thing of the catalogue (ADR 0054), from a made-up
//! game folder and Vortex staging folder.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::path::Path;

use mallet_core::catalogue::AssetSource;
use mallet_core::{Action, ModRef};
use mallet_engine::{ScanOptions, asset_versions};
use mallet_formats_bethesda::fixture;

fn write(path: &Path, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn every_mod_with_a_copy_is_a_version_and_the_deployed_one_is_shown() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("Game");
    let data = game.join("Data");
    let local = root.path().join("Local");
    let staging = root.path().join("staging");
    write(&data.join("Skyrim.esm"), &fixture::plugin(&[], 1, &[]));
    write(&local.join("plugins.txt"), b"");

    // Mod A's sword is deployed; Mod B brings its own sword and texture,
    // which Vortex kept back.
    let sword_a = fixture::nif(&[&[
        r"textures\weapons\sword.dds",
        r"textures\weapons\sword_n.dds",
    ]]);
    let sword_b = fixture::nif(&[&[r"textures\weapons\sword_b.dds"]]);
    write(&data.join("meshes/weapons/sword.nif"), &sword_a);
    write(&staging.join("Mod A/meshes/weapons/sword.nif"), &sword_a);
    write(&staging.join("Mod B/meshes/weapons/sword.nif"), &sword_b);
    write(
        &staging.join("Mod B/textures/weapons/sword_b.dds"),
        b"not a texture",
    );
    let escape = |text: &str| text.replace('\\', "\\\\");
    write(
        &data.join("vortex.deployment.json"),
        format!(
            r#"{{ "targetPath": "{}", "files": [
                {{ "relPath": "meshes\\weapons\\sword.nif", "source": "Mod A" }},
                {{ "relPath": "textures\\other.dds", "source": "Mod B" }}
            ] }}"#,
            escape(&data.to_string_lossy())
        )
        .as_bytes(),
    );

    let options = ScanOptions {
        game: None,
        game_dir: Some(game),
        local_dir: Some(local),
        extender_log: None,
        deployment_manifest: None,
        manager_state: None,
        manager_live_state: None,
        downloads_dir: None,
        staging_dir: Some(staging),
        unpacker: None,
        masterlist: None,
        manager: mallet_engine::ModManager::Vortex,
    };
    let versions = asset_versions(&options, &["meshes/weapons/sword.nif".to_owned()]).unwrap();

    assert_eq!(
        versions.files,
        [
            "meshes/weapons/sword.nif",
            "textures/weapons/sword.dds",
            "textures/weapons/sword_n.dds",
            "textures/weapons/sword_b.dds",
        ]
    );
    assert_eq!(versions.versions.len(), 2);
    let shown = &versions.versions[0];
    assert_eq!(shown.id, "mod:Mod A");
    assert_eq!(shown.shown, ["meshes/weapons/sword.nif"]);
    let AssetSource::Mod { mod_ref } = &shown.source else {
        unreachable!("a mod's version")
    };
    assert_eq!(mod_ref.id, "Mod A");

    let kept_back = &versions.versions[1];
    assert_eq!(kept_back.id, "mod:Mod B");
    assert_eq!(
        kept_back.files,
        ["meshes/weapons/sword.nif", "textures/weapons/sword_b.dds"]
    );
    assert_eq!(kept_back.shown, Vec::<String>::new());

    // Taking Mod B's version leaves its files out of Mod A and gives them
    // back to Mod B; Mod A's version is in the game already.
    let names = |mods: &[ModRef]| -> Vec<String> { mods.iter().map(|m| m.id.clone()).collect() };
    let Some(Action::FileOverrides {
        files,
        leave_out,
        provide,
    }) = &kept_back.choice
    else {
        unreachable!("a choice for the version kept back")
    };
    assert_eq!(files, &kept_back.files);
    assert_eq!(names(leave_out), ["Mod A"]);
    assert_eq!(names(provide), ["Mod B"]);
    assert_eq!(shown.choice, None);

    let Some(Action::FileOverrides {
        files,
        leave_out,
        provide,
    }) = &versions.reset
    else {
        unreachable!("a way back")
    };
    assert_eq!(
        files,
        &["meshes/weapons/sword.nif", "textures/weapons/sword_b.dds"]
    );
    assert_eq!(leave_out, &Vec::<ModRef>::new());
    assert_eq!(names(provide), ["Mod A", "Mod B"]);
}
