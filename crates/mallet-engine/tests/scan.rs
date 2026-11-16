//! End-to-end tests of a scan against a made-up game directory.
//!
//! Every file here is synthetic (ADR 0017): no real game or mod file is
//! needed to run the test suite.

// Helper functions of an integration test are not inside a `#[test]`
// function, so the test exemptions in clippy.toml do not reach them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use mallet_core::{ArgValue, Finding, ScanReport, Severity, kinds};
use mallet_engine::{EngineError, ScanOptions, last_report, scan, scan_and_store};
use mallet_formats_bethesda::fixture::{self, RecordSpec};

const SKSE_LOG: &str = "\
SKSE64 runtime: initialize (version = 2.3.1 01070680 01DD748122C36157, os = 6.2 (9200))
checking plugin Fixes.dll
checking plugin helper.dll
checking plugin Old.dll
plugin Fixes.dll (00000001 Fixes 00000007) loaded correctly (handle 1)
plugin helper.dll (00000000  00000000) no version data 0 (handle 0)
plugin Old.dll (00000001 old 00000001) disabled, incompatible with current version of the game 0 (handle 0)
";

const NPC: [u8; 4] = *b"NPC_";
const MASTER_FLAG: u32 = 1;

/// The form ID of the character used in the tests, and her face mesh.
const UTHGERD: u32 = 0x0001_3BA1;
const UTHGERD_MESH: &str = r"meshes\actors\character\facegendata\facegeom\skyrim.esm\00013ba1.nif";

/// A made-up installation: a game directory with the game's master file,
/// the directory that holds `plugins.txt`, and a script extender log.
struct Setup {
    root: tempfile::TempDir,
    /// Which mod each deployed file belongs to, for the Vortex manifest.
    deployed: Vec<(String, String)>,
}

impl Setup {
    fn new() -> Self {
        let setup = Self {
            root: tempfile::tempdir().unwrap(),
            deployed: Vec::new(),
        };
        fs::create_dir_all(setup.data_dir()).unwrap();
        fs::create_dir_all(setup.local_dir()).unwrap();
        fs::write(setup.log_path(), SKSE_LOG).unwrap();
        // The plugins the log names are still in the game.
        for plugin in ["Fixes.dll", "helper.dll", "Old.dll"] {
            setup.write_data(&format!("SKSE/Plugins/{plugin}"), b"not a real library");
        }
        setup.write_data(
            "Skyrim.esm",
            &fixture::plugin(&[], MASTER_FLAG, &[(NPC, vec![uthgerd(0x11)])]),
        );
        setup.activate(&[]);
        setup
    }

    fn game_dir(&self) -> PathBuf {
        self.root.path().join("Game")
    }

    fn data_dir(&self) -> PathBuf {
        self.game_dir().join("Data")
    }

    fn local_dir(&self) -> PathBuf {
        self.root.path().join("Local")
    }

    fn log_path(&self) -> PathBuf {
        self.root.path().join("skse64.log")
    }

    fn cache_path(&self) -> PathBuf {
        self.root.path().join("cache").join("cache.sqlite")
    }

    fn manifest_path(&self) -> PathBuf {
        self.data_dir().join("vortex.deployment.json")
    }

    /// Writes `plugins.txt` with the given plugins active, in this order.
    fn activate(&self, plugins: &[&str]) {
        let lines = plugins
            .iter()
            .fold(String::new(), |lines, name| lines + "*" + name + "\n");
        fs::write(self.local_dir().join("plugins.txt"), lines).unwrap();
    }

    /// Writes a file into the data directory that no mod manager knows.
    fn write_data(&self, relative: &str, content: &[u8]) {
        let path = self.data_dir().join(relative.replace('\\', "/"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Writes a file into the data directory as deployed by Vortex for
    /// `mod_name`.
    fn deploy(&mut self, mod_name: &str, relative: &str, content: &[u8]) {
        self.write_data(relative, content);
        self.deployed
            .push((relative.to_owned(), mod_name.to_owned()));
    }

    /// Writes the Vortex deployment manifest for everything deployed so far.
    fn with_vortex_deployment(mut self) -> Self {
        self.deployed.push((
            r"SKSE\Plugins\Old.dll".to_owned(),
            "Old Mod 123 1.0".to_owned(),
        ));
        let escape = |text: &str| text.replace('\\', "\\\\");
        let files: Vec<String> = self
            .deployed
            .iter()
            .map(|(path, mod_name)| {
                format!(
                    r#"{{ "relPath": "{}", "source": "{mod_name}" }}"#,
                    escape(path)
                )
            })
            .collect();
        let manifest = format!(
            r#"{{ "targetPath": "{}", "files": [ {} ] }}"#,
            escape(&self.data_dir().to_string_lossy()),
            files.join(", ")
        );
        fs::write(self.manifest_path(), manifest).unwrap();
        self
    }

    fn options(&self) -> ScanOptions {
        ScanOptions {
            game_dir: Some(self.game_dir()),
            local_dir: Some(self.local_dir()),
            extender_log: Some(self.log_path()),
            deployment_manifest: None,
            // Tests never look at the real Vortex of the machine they run on.
            manager_state: Some(self.manager_state_path()),
            manager_live_state: None,
            downloads_dir: Some(self.downloads_dir()),
            unpacker: None,
        }
    }

    fn manager_state_path(&self) -> PathBuf {
        self.root.path().join("vortex-state.json")
    }

    fn live_state_path(&self) -> PathBuf {
        self.root.path().join("link-state.json")
    }

    fn downloads_dir(&self) -> PathBuf {
        self.root.path().join("downloads")
    }
}

/// Vortex's state with one mod, shown under `name`.
fn state_naming_broken_mod(name: &str) -> String {
    format!(
        r#"{{ "persistent": {{ "mods": {{ "skyrimse": {{ "Broken Mod": {{
            "attributes": {{ "logicalFileName": "{name}" }} }} }} }} }} }}"#
    )
}

/// Writes a file and gives it a modification time `seconds` after a fixed
/// moment, so that tests do not depend on how fast files are written.
fn write_at(path: &Path, content: &str, seconds: u64) {
    fs::write(path, content).unwrap();
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000 + seconds);
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// A setup with one finding that names its mod, and the options to scan
/// it with both state files.
fn setup_with_two_states() -> (Setup, ScanOptions) {
    let mut setup = Setup::new();
    setup.deploy("Broken Mod", "Broken.esp", &empty_plugin());
    setup.deploy("Broken Mod", "Broken.bsa", b"not an archive");
    setup.activate(&["Broken.esp"]);
    let setup = setup.with_vortex_deployment();
    let options = ScanOptions {
        manager_live_state: Some(setup.live_state_path()),
        ..setup.options()
    };
    (setup, options)
}

fn name_of_broken_mod(report: &mallet_core::ScanReport) -> String {
    only(report, "setup.file-unreadable")
        .provider
        .as_ref()
        .unwrap()
        .name
        .clone()
}

fn manager_state_location(report: &mallet_core::ScanReport) -> &str {
    let mut locations = report
        .locations
        .iter()
        .filter(|location| location.role == mallet_core::LocationRole::ManagerState);
    let location = locations.next().unwrap();
    assert!(locations.next().is_none(), "one state file is read");
    &location.path
}

#[test]
fn the_links_state_is_read_when_it_is_newer_than_vortex_backup() {
    let (setup, options) = setup_with_two_states();
    write_at(
        &setup.manager_state_path(),
        &state_naming_broken_mod("Name an hour ago"),
        0,
    );
    write_at(
        &setup.live_state_path(),
        &state_naming_broken_mod("Name now"),
        60,
    );

    let report = scan(&options).unwrap();

    assert_eq!(name_of_broken_mod(&report), "Name now");
    assert_eq!(
        manager_state_location(&report),
        setup.live_state_path().display().to_string()
    );
}

#[test]
fn vortex_backup_is_read_when_the_link_has_not_written_since() {
    let (setup, options) = setup_with_two_states();
    write_at(
        &setup.live_state_path(),
        &state_naming_broken_mod("Name when the link last ran"),
        0,
    );
    write_at(
        &setup.manager_state_path(),
        &state_naming_broken_mod("Name in the backup"),
        60,
    );

    let report = scan(&options).unwrap();

    assert_eq!(name_of_broken_mod(&report), "Name in the backup");
    assert_eq!(
        manager_state_location(&report),
        setup.manager_state_path().display().to_string()
    );
}

#[test]
fn the_links_state_is_enough_without_a_backup() {
    let (setup, options) = setup_with_two_states();
    write_at(
        &setup.live_state_path(),
        &state_naming_broken_mod("Name now"),
        0,
    );

    let report = scan(&options).unwrap();

    assert_eq!(name_of_broken_mod(&report), "Name now");
}

#[test]
fn an_unreadable_state_from_the_link_falls_back_to_the_backup() {
    let (setup, options) = setup_with_two_states();
    write_at(
        &setup.manager_state_path(),
        &state_naming_broken_mod("Name in the backup"),
        0,
    );
    write_at(&setup.live_state_path(), "{ half a file", 60);

    let report = scan(&options).unwrap();

    assert_eq!(name_of_broken_mod(&report), "Name in the backup");
    assert_eq!(
        manager_state_location(&report),
        setup.manager_state_path().display().to_string()
    );
}

/// The character with a face described by one head part.
fn uthgerd(head_part: u32) -> RecordSpec {
    RecordSpec::new(NPC, UTHGERD)
        .with(*b"EDID", fixture::zstring("Uthgerd"))
        .with(*b"PNAM", head_part.to_le_bytes().to_vec())
}

fn plugin_with(npc: RecordSpec) -> Vec<u8> {
    fixture::plugin(&["Skyrim.esm"], 0, &[(NPC, vec![npc])])
}

fn empty_plugin() -> Vec<u8> {
    fixture::plugin(&["Skyrim.esm"], 0, &[])
}

fn kinds_of(report: &ScanReport) -> Vec<&str> {
    report.findings.iter().map(|f| f.kind.as_str()).collect()
}

fn only<'a>(report: &'a ScanReport, kind: &str) -> &'a Finding {
    let mut matching = report.findings.iter().filter(|f| f.kind == kind);
    let finding = matching
        .next()
        .unwrap_or_else(|| panic!("no {kind} among {:?}", kinds_of(report)));
    assert!(matching.next().is_none(), "more than one {kind}");
    finding
}

#[test]
fn a_plugin_removed_since_the_last_game_start_is_not_reported() {
    let setup = Setup::new().with_vortex_deployment();
    fs::remove_file(setup.data_dir().join("SKSE/Plugins/Old.dll")).unwrap();
    let report = scan(&setup.options()).unwrap();

    assert!(!kinds_of(&report).contains(&"extender.plugin-incompatible"));
    assert!(kinds_of(&report).contains(&"extender.plugin-no-version-data"));
}

#[test]
fn reports_the_plugins_that_did_not_load() {
    let setup = Setup::new().with_vortex_deployment();
    let report = scan(&setup.options()).unwrap();

    assert_eq!(report.game.id, "skyrimse");
    assert_eq!(report.game.version.as_deref(), Some("1.7.104"));
    assert_eq!(report.game.extender_version.as_deref(), Some("2.3.1"));
    assert_eq!(report.extender_plugins_checked, 3);
    assert_eq!(report.plugins_checked, 1);
    assert_eq!(
        kinds_of(&report),
        [
            "extender.plugin-incompatible",
            "extender.plugin-no-version-data"
        ]
    );

    let incompatible = &report.findings[0];
    assert_eq!(incompatible.severity, Severity::Error);
    assert_eq!(
        incompatible.id.as_str(),
        "extender.plugin-incompatible:file:data/skse/plugins/old.dll"
    );
    assert_eq!(
        incompatible.provider.as_ref().map(|m| m.name.as_str()),
        Some("Old Mod 123 1.0")
    );
    // The helper library was not deployed by Vortex in this setup.
    assert_eq!(report.findings[1].provider, None);
}

/// The first reference problem of milestone 1: a flora mod's loose files
/// silently replace part of a grass mod that is packed in an archive.
#[test]
fn reports_loose_files_that_replace_part_of_another_mods_archive() {
    let mut setup = Setup::new();
    setup.deploy("Grass Mod", "Grass.esp", &empty_plugin());
    setup.deploy(
        "Grass Mod",
        "Grass.bsa",
        &fixture::bsa(&[
            r"meshes\landscape\grass\fern.nif",
            r"textures\landscape\grass\fern.dds",
            r"textures\landscape\grass\moss.dds",
        ]),
    );
    setup.deploy("Flora Mod", r"meshes\landscape\grass\fern.nif", b"mesh");
    setup.deploy(
        "Flora Mod",
        r"textures\landscape\grass\fern.dds",
        b"texture",
    );
    setup.activate(&["Grass.esp"]);
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.archives_checked, 1);

    let finding = only(&report, "assets.loose-over-archive");
    assert_eq!(finding.severity, Severity::Info);
    assert_eq!(finding.args["count"], ArgValue::Number(2));
    assert_eq!(finding.args["winnerMod"], ArgValue::from("Flora Mod"));
    assert_eq!(finding.args["archive"], ArgValue::from("grass.bsa"));
    assert_eq!(finding.args["archiveMod"], ArgValue::from("Grass Mod"));
}

/// The second reference problem of milestone 1: a face overhaul provides a
/// character's face files, and a plugin loaded later changes her record
/// back to another face.
#[test]
fn reports_characters_whose_face_files_do_not_match_the_winning_record() {
    let mut setup = Setup::new();
    setup.write_data("Skyrim - Meshes0.bsa", &fixture::bsa(&[UTHGERD_MESH]));
    setup.deploy("Face Overhaul", "Faces.esp", &plugin_with(uthgerd(0x22)));
    setup.deploy("Face Overhaul", UTHGERD_MESH, b"face mesh");
    setup.deploy("College Mod", "College.esp", &plugin_with(uthgerd(0x11)));
    setup.activate(&["Faces.esp", "College.esp"]);
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.plugins_checked, 3);

    let finding = only(&report, "npcs.face-mismatch");
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.args["examples"], ArgValue::from("Uthgerd"));
    assert_eq!(finding.args["winnerPlugin"], ArgValue::from("College.esp"));
    assert_eq!(finding.args["facePlugin"], ArgValue::from("Faces.esp"));
    assert_eq!(finding.args["faceMod"], ArgValue::from("Face Overhaul"));

    // With the overhaul loaded last, its record wins and matches its files.
    setup.activate(&["College.esp", "Faces.esp"]);
    let report = scan(&setup.options()).unwrap();
    assert!(!kinds_of(&report).contains(&"npcs.face-mismatch"));
}

/// The reference problem of milestone 3: a mesh mod replaces the game's
/// wall with a version that points at its own texture, so the wall no
/// longer shows what a texture mod provides for the original texture.
#[test]
fn reports_meshes_that_no_longer_use_a_retextured_texture() {
    let wall = r"meshes\architecture\wall.nif";
    let original = fixture::nif(&[&[
        r"textures\architecture\wall.dds",
        r"textures\architecture\wall_n.dds",
    ]]);
    let replacement = fixture::nif(&[&[
        r"textures\meshmod\wall.dds",
        r"textures\architecture\wall_n.dds",
    ]]);

    let mut setup = Setup::new();
    setup.write_data(
        "Skyrim - Meshes0.bsa",
        &fixture::bsa_with(
            &[(wall, original.as_slice())],
            fixture::BsaOptions {
                version: 105,
                compressed: true,
                embedded_names: false,
            },
        ),
    );
    setup.deploy("Mesh Mod", wall, &replacement);
    setup.deploy("Mesh Mod", r"textures\meshmod\wall.dds", b"texture");
    setup.deploy("Texture Mod", r"textures\architecture\wall.dds", b"texture");
    setup.deploy(
        "Texture Mod",
        r"textures\architecture\wall_n.dds",
        b"texture",
    );
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.meshes_checked, 1);

    let finding = only(&report, "meshes.retexture-bypassed");
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.args["count"], ArgValue::Number(1));
    assert_eq!(finding.args["examples"], ArgValue::from("wall.nif"));
    assert_eq!(finding.args["meshMod"], ArgValue::from("Mesh Mod"));
    assert_eq!(finding.args["textureMod"], ArgValue::from("Texture Mod"));
    let kind = kinds::find(&finding.kind).unwrap();
    let mut declared = kind.args.to_vec();
    declared.sort_unstable();
    assert_eq!(finding.args.keys().collect::<Vec<_>>(), declared);

    // A replacement that keeps the original textures is fine.
    fs::write(
        setup.data_dir().join("meshes/architecture/wall.nif"),
        &original,
    )
    .unwrap();
    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.meshes_checked, 1);
    assert!(!kinds_of(&report).contains(&"meshes.retexture-bypassed"));

    // A loose file that is not a mesh at all is skipped, not fatal.
    fs::write(
        setup.data_dir().join("meshes/architecture/wall.nif"),
        b"not a mesh",
    )
    .unwrap();
    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.meshes_checked, 0);
}

/// An installer with one patch that is ticked when `City.esp` is active.
const INSTALLER: &str = r#"<config>
  <installSteps>
    <installStep name="Patches">
      <optionalFileGroups>
        <group name="Other mods" type="SelectAny">
          <plugins>
            <plugin name="Something else"/>
            <plugin name="City Mod patch">
              <typeDescriptor>
                <dependencyType>
                  <defaultType name="Optional"/>
                  <patterns>
                    <pattern>
                      <dependencies>
                        <fileDependency file="City.esp" state="Active"/>
                      </dependencies>
                      <type name="Recommended"/>
                    </pattern>
                  </patterns>
                </dependencyType>
              </typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
</config>"#;

/// Vortex's state for a patch hub that was installed from an archive, with
/// one option chosen that is not the city patch.
const MANAGER_STATE: &str = r#"{
  "settings": { "profiles": { "lastActiveProfile": { "skyrimse": "p1" } } },
  "persistent": {
    "profiles": { "p1": { "modState": { "Patch Hub 1.0": { "enabled": true } } } },
    "mods": { "skyrimse": { "Patch Hub 1.0": {
      "archiveId": "a1",
      "installationPath": "Patch Hub 1.0",
      "attributes": {
        "logicalFileName": "Patch Hub",
        "installerChoices": { "options": [ { "name": "Patches", "groups": [
          { "name": "Other mods", "choices": [ { "name": "Something else", "idx": 0 } ] }
        ] } ] }
      }
    } } },
    "downloads": { "files": { "a1": { "localPath": "Patch Hub 1.0.7z" } } }
  }
}"#;

/// The reference problem of milestone 2: a patch hub was installed before
/// the mod it has a patch for, so the patch was never offered.
///
/// Needs a 7-Zip to build and read the archive, which the Windows machines
/// of CI have. Elsewhere there is nothing to test against.
#[test]
fn reports_a_patch_the_installer_would_tick_now() {
    let Some(unpacker) = mallet_unpack::Unpacker::locate() else {
        eprintln!("no 7-Zip found, skipping");
        return;
    };
    let mut setup = Setup::new();
    setup.deploy("Patch Hub 1.0", "PatchHub.esp", &empty_plugin());
    setup.deploy("City Mod", "City.esp", &empty_plugin());
    setup.activate(&["PatchHub.esp", "City.esp"]);

    let content = setup.root.path().join("archive-content");
    fs::create_dir_all(content.join("fomod")).unwrap();
    fs::write(content.join("fomod").join("ModuleConfig.xml"), INSTALLER).unwrap();
    fs::create_dir_all(setup.downloads_dir()).unwrap();
    let packed = Command::new(unpacker.program())
        .arg("a")
        .arg(setup.downloads_dir().join("Patch Hub 1.0.7z"))
        .arg(content.join("fomod"))
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(packed.success());
    fs::write(setup.manager_state_path(), MANAGER_STATE).unwrap();
    let setup = setup.with_vortex_deployment();

    let mut options = setup.options();
    options.unpacker = Some(unpacker.program().to_path_buf());
    let report = scan_and_store(&options, &setup.cache_path()).unwrap();
    assert_eq!(report.installers_checked, 1);

    let finding = only(&report, "installer.options-now-recommended");
    assert_eq!(finding.severity, Severity::Warning);
    // The mod goes by the name Vortex shows, not by its folder.
    assert_eq!(finding.args["mod"], ArgValue::from("Patch Hub"));
    assert_eq!(finding.args["count"], ArgValue::Number(1));
    assert_eq!(
        finding.args["examples"],
        ArgValue::from("Patches › Other mods › City Mod patch")
    );
    let kind = kinds::find(&finding.kind).unwrap();
    let mut declared = kind.args.to_vec();
    declared.sort_unstable();
    assert_eq!(finding.args.keys().collect::<Vec<_>>(), declared);

    // The second scan does not need the unpacker: the installer is cached.
    options.unpacker = Some(PathBuf::from("no-such-unpacker"));
    let again = scan_and_store(&options, &setup.cache_path()).unwrap();
    assert_eq!(again.findings, report.findings);

    // Without cache and without unpacker the installer cannot be read, and
    // the scan says so instead of failing.
    let blind = scan(&options).unwrap();
    assert_eq!(blind.installers_checked, 0);
    assert!(!kinds_of(&blind).contains(&"installer.options-now-recommended"));

    // With the city mod gone, the installer has nothing more to offer.
    setup.activate(&["PatchHub.esp"]);
    let without_city = scan_and_store(&options, &setup.cache_path()).unwrap();
    assert!(!kinds_of(&without_city).contains(&"installer.options-now-recommended"));
}

/// A reference problem met while trying out actions: a patch was ticked in
/// an installer for a mod that is not installed, and the game would have
/// closed while starting.
#[test]
fn reports_a_patch_that_builds_on_a_plugin_which_is_not_installed() {
    let mut setup = Setup::new();
    setup.deploy("City Mod", "City.esp", &empty_plugin());
    setup.deploy(
        "Patch Hub",
        "City - Dungeons Patch.esp",
        &fixture::plugin(&["Skyrim.esm", "City.esp", "Dungeons.esp"], 0, &[]),
    );
    setup.activate(&["City.esp", "City - Dungeons Patch.esp"]);
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();

    let finding = only(&report, "plugins.master-missing");
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(
        finding.args["plugin"],
        ArgValue::from("City - Dungeons Patch.esp")
    );
    assert_eq!(finding.args["masters"], ArgValue::from("Dungeons.esp"));
    assert_eq!(
        finding.provider.as_ref().map(|m| m.name.as_str()),
        Some("Patch Hub")
    );
}

/// A house of the game: 800 by 600 units, 600 high.
const HOUSE: u32 = 0x0001_0F00;

fn house() -> RecordSpec {
    let obnd: Vec<u8> = [-400i16, -300, 0, 400, 300, 600]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    RecordSpec::new(*b"STAT", HOUSE)
        .with(*b"EDID", fixture::zstring("TownHouse"))
        .with(*b"OBND", obnd)
        .with(*b"MODL", fixture::zstring(r"Architecture\TownHouse.nif"))
}

/// A plugin that puts the game's house into the game's worldspace at
/// `position`.
fn town_mod(cell: u32, reference: u32, position: [f32; 3]) -> Vec<u8> {
    let world = RecordSpec::new(*b"WRLD", 0x0000_003C).with(*b"EDID", fixture::zstring("Tamriel"));
    let cell = RecordSpec::new(*b"CELL", cell);
    fixture::plugin_with_raw_groups(
        &["Skyrim.esm"],
        0,
        &[fixture::worldspace(
            &world,
            &[(
                cell,
                vec![fixture::reference(
                    reference, HOUSE, position, [0.0; 3], None,
                )],
            )],
        )],
    )
}

/// The reference problem of milestone 4: two mods that change the same
/// town each put the same house in the same spot.
#[test]
fn reports_a_house_that_two_mods_place_in_the_same_spot() {
    let mut setup = Setup::new();
    setup.write_data(
        "Skyrim.esm",
        &fixture::plugin(
            &[],
            MASTER_FLAG,
            &[(NPC, vec![uthgerd(0x11)]), (*b"STAT", vec![house()])],
        ),
    );
    setup.deploy(
        "Village Mod",
        "Village.esp",
        &town_mod(0x0100_0801, 0x0100_0810, [5000.0, 9000.0, 0.0]),
    );
    setup.deploy(
        "Hamlet Mod",
        "Hamlet.esp",
        &town_mod(0x0100_0901, 0x0100_0910, [5010.0, 9004.0, 2.0]),
    );
    setup.activate(&["Village.esp", "Hamlet.esp"]);
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();

    let finding = only(&report, "objects.duplicated");
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.args["first"], ArgValue::from("Hamlet Mod"));
    assert_eq!(finding.args["second"], ArgValue::from("Village Mod"));
    assert_eq!(finding.args["count"], ArgValue::Number(1));
    assert_eq!(
        finding.args["examples"],
        ArgValue::from("TownHouse (Tamriel 1, 2)")
    );
    // The same identity whichever plugin loads first.
    assert_eq!(
        finding.id.as_str(),
        "objects.duplicated:pair:file:data/hamlet.esp+file:data/village.esp"
    );

    // A patch for the two leaves the decision to its author.
    let mut setup = setup;
    setup.deploy(
        "Patch",
        "Village - Hamlet Patch.esp",
        &fixture::plugin(&["Village.esp", "Hamlet.esp"], 0, &[]),
    );
    setup.activate(&["Village.esp", "Hamlet.esp", "Village - Hamlet Patch.esp"]);
    let setup = setup.with_vortex_deployment();
    let patched = scan(&setup.options()).unwrap();
    assert!(!kinds_of(&patched).contains(&"objects.duplicated"));
}

#[test]
fn a_damaged_archive_becomes_a_finding_and_does_not_stop_the_scan() {
    let mut setup = Setup::new();
    setup.deploy("Broken Mod", "Broken.esp", &empty_plugin());
    setup.deploy(
        "Broken Mod",
        "Broken.bsa",
        b"definitely not an archive, just text",
    );
    setup.activate(&["Broken.esp"]);
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();
    let finding = only(&report, "setup.file-unreadable");
    assert_eq!(finding.args["file"], ArgValue::from("broken.bsa"));
    assert_eq!(
        finding.provider.as_ref().map(|m| m.name.as_str()),
        Some("Broken Mod")
    );
    // The script extender findings are still there.
    assert!(kinds_of(&report).contains(&"extender.plugin-incompatible"));
}

#[test]
fn every_finding_carries_exactly_the_arguments_its_kind_declares() {
    let mut setup = Setup::new();
    setup.write_data("Skyrim - Meshes0.bsa", &fixture::bsa(&[UTHGERD_MESH]));
    setup.deploy("Face Overhaul", "Faces.esp", &plugin_with(uthgerd(0x22)));
    setup.deploy("Face Overhaul", UTHGERD_MESH, b"face mesh");
    setup.deploy("College Mod", "College.esp", &plugin_with(uthgerd(0x11)));
    setup.deploy(
        "College Mod",
        "College.bsa",
        &fixture::bsa(&[r"textures\shared.dds"]),
    );
    setup.deploy("Grass Mod", "Grass.esp", &empty_plugin());
    setup.deploy(
        "Grass Mod",
        "Grass.bsa",
        &fixture::bsa(&[r"textures\shared.dds", r"textures\grass.dds"]),
    );
    setup.deploy("Flora Mod", r"textures\grass.dds", b"texture");
    setup.deploy("Broken Mod", "Broken.esp", &empty_plugin());
    setup.deploy(
        "Broken Mod",
        "Broken.bsa",
        b"definitely not an archive, just text",
    );
    setup.activate(&["Faces.esp", "College.esp", "Grass.esp", "Broken.esp"]);
    let setup = setup.with_vortex_deployment();

    let report = scan(&setup.options()).unwrap();
    let mut seen: Vec<&str> = kinds_of(&report);
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen,
        [
            "assets.archive-over-archive",
            "assets.loose-over-archive",
            "extender.plugin-incompatible",
            "extender.plugin-no-version-data",
            "npcs.face-mismatch",
            "setup.file-unreadable",
        ]
    );

    for finding in &report.findings {
        let kind = kinds::find(&finding.kind).unwrap();
        let mut declared = kind.args.to_vec();
        declared.sort_unstable();
        let carried: Vec<_> = finding.args.keys().map(String::as_str).collect();
        assert_eq!(carried, declared, "arguments of {}", kind.id);
        assert_eq!(finding.severity, kind.severity);
        assert_eq!(finding.confidence, kind.confidence);
    }
}

#[test]
fn a_setup_without_vortex_is_scanned_without_attribution() {
    let setup = Setup::new();
    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.findings.len(), 2);
    assert!(report.findings.iter().all(|f| f.provider.is_none()));
}

#[test]
fn a_setup_without_a_script_extender_log_has_no_findings() {
    let setup = Setup::new();
    fs::remove_file(setup.log_path()).unwrap();
    let report = scan(&setup.options()).unwrap();

    assert_eq!(report.extender_plugins_checked, 0);
    assert_eq!(report.game.version, None);
    assert_eq!(report.findings, Vec::new());
}

#[test]
fn an_unknown_load_order_is_reported_and_the_rest_still_runs() {
    let setup = Setup::new();
    // Without a data directory there is no load order to read.
    fs::remove_dir_all(setup.data_dir()).unwrap();

    let report = scan(&setup.options()).unwrap();
    assert_eq!(report.plugins_checked, 0);
    assert!(kinds_of(&report).contains(&"setup.load-order-unknown"));
    assert!(kinds_of(&report).contains(&"extender.plugin-incompatible"));
}

#[test]
fn a_missing_game_directory_is_reported_as_such() {
    let setup = Setup::new();
    let options = ScanOptions {
        game_dir: Some(setup.root.path().join("Nowhere")),
        ..setup.options()
    };
    assert!(matches!(
        scan(&options),
        Err(EngineError::GameDirMissing(_))
    ));
}

#[test]
fn a_deployment_for_another_installation_is_an_error() {
    let setup = Setup::new();
    fs::write(
        setup.manifest_path(),
        r#"{ "targetPath": "Z:\\Elsewhere\\Data", "files": [] }"#,
    )
    .unwrap();
    assert!(matches!(
        scan(&setup.options()),
        Err(EngineError::Vortex(_))
    ));
}

#[test]
fn the_last_scan_can_be_shown_again_without_scanning() {
    let setup = Setup::new().with_vortex_deployment();
    assert_eq!(last_report(&setup.cache_path()).unwrap(), None);

    let report = scan_and_store(&setup.options(), &setup.cache_path()).unwrap();
    assert_eq!(last_report(&setup.cache_path()).unwrap(), Some(report));
}

#[test]
fn scanning_changes_nothing_in_the_game_directory() {
    let mut setup = Setup::new();
    setup.deploy("Grass Mod", "Grass.esp", &empty_plugin());
    setup.deploy(
        "Grass Mod",
        "Grass.bsa",
        &fixture::bsa(&[r"textures\grass.dds"]),
    );
    setup.activate(&["Grass.esp"]);
    let setup = setup.with_vortex_deployment();

    let before = listing(&setup.game_dir());
    scan_and_store(&setup.options(), &setup.cache_path()).unwrap();
    assert_eq!(listing(&setup.game_dir()), before);
}

/// Every file below `directory` with its size and modification time.
fn listing(directory: &Path) -> Vec<(PathBuf, u64, Option<std::time::SystemTime>)> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                found.push((entry.path(), metadata.len(), metadata.modified().ok()));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn the_patch_in_the_game_is_reported_with_its_mod() {
    let mut setup = Setup::new();
    setup.deploy(
        "Mod Mallet Patch",
        "ModMallet_SWAP.ini",
        b"[References]\n0x812~Village.esp|0x3B~Skyrim.esm\n",
    );
    let setup = setup.with_vortex_deployment();
    let report = scan(&setup.options()).unwrap();

    let patch = report.installed_patch.unwrap();
    assert_eq!(
        patch.objects,
        [mallet_core::ObjectRef {
            plugin: "Village.esp".to_owned(),
            local_id: 0x812,
        }]
    );
    assert_eq!(
        patch.provider.map(|provider| provider.name).as_deref(),
        Some("Mod Mallet Patch")
    );
}

#[test]
fn without_a_patch_none_is_reported() {
    let report = scan(&Setup::new().options()).unwrap();
    assert_eq!(report.installed_patch, None);
}
