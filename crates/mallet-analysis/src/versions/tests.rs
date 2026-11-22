//! Made-up setups after made-up updates.

use mallet_core::facts::versions::{ExtenderInstall, VersionFacts};
use mallet_core::facts::{ExtenderPlugin, ExtenderReport, PluginFacts};
use mallet_core::{
    Action, ArgValue, Facts, GameInfo, GamePath, ModInfo, ModRef, ModSource, NoModSource, Severity,
};

use super::*;
use crate::run_all;

fn version(text: &str) -> RuntimeVersion {
    RuntimeVersion::parse(text).unwrap()
}

/// A plugin built for exactly these game versions.
fn bound(file: &str, versions: &[&str]) -> NativePlugin {
    declared(
        file,
        PluginVersionData {
            name: file.to_owned(),
            compatible: versions.iter().map(|text| version(text)).collect(),
            ..PluginVersionData::default()
        },
    )
}

/// A plugin that finds the game through the Address Library and uses the
/// structures since 1.6.629: it runs in any version.
fn independent(file: &str) -> NativePlugin {
    declared(
        file,
        PluginVersionData {
            name: file.to_owned(),
            address_library: true,
            structs_post_629: true,
            ..PluginVersionData::default()
        },
    )
}

fn declared(file: &str, data: PluginVersionData) -> NativePlugin {
    plugin(file, PluginDeclaration::VersionData(data))
}

fn plugin(file: &str, declaration: PluginDeclaration) -> NativePlugin {
    NativePlugin {
        path: GamePath::new("data/skse/plugins").join(file),
        file_name: file.to_owned(),
        file_version: Some("1.0.0.0".to_owned()),
        product_name: None,
        declaration,
    }
}

fn skse(runtimes: &[&str]) -> ExtenderInstall {
    ExtenderInstall {
        name: "SKSE".to_owned(),
        version: Some(version("2.3.1")),
        runtimes: runtimes.iter().map(|text| version(text)).collect(),
        versioned: true,
    }
}

/// Skyrim updated to 1.7.104, with SKSE and the Address Library for it.
fn updated(plugins: Vec<NativePlugin>) -> Facts {
    Facts {
        versions: VersionFacts {
            game: Some(version("1.7.104")),
            extender: Some(skse(&["1.6.1170", "1.7.104"])),
            address_library: Some(vec![version("1.6.1170"), version("1.7.104")]),
            native_plugins: plugins,
            ..VersionFacts::default()
        },
        ..Facts::default()
    }
}

fn kinds_of(findings: &[Finding]) -> Vec<(&str, String)> {
    findings
        .iter()
        .map(|finding| (finding.kind.as_str(), finding.subject.identity()))
        .collect()
}

/// A mod manager that deployed each file of a list for a mod of the same
/// name as the file's stem.
struct ByStem;

impl ModSource for ByStem {
    fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
        let name = path.as_str().rsplit('/').next()?;
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        Some(ModRef {
            source: "test".to_owned(),
            id: format!("{stem}-id"),
            name: stem.to_owned(),
        })
    }
}

#[test]
fn the_reference_case_racemenu_after_the_update_to_1_7_104() {
    let mut facts = updated(vec![
        bound("skee64.dll", &["1.6.1170"]),
        independent("EngineFixes.dll"),
        bound("JContainers64.dll", &["1.7.104"]),
    ]);
    // The log of the last start, already in 1.7.104, agrees.
    facts.extender = Some(ExtenderReport {
        extender_version: Some("2.3.1".to_owned()),
        game_version: Some("1.7.104".to_owned()),
        plugin_dir: GamePath::new("data/skse/plugins"),
        plugins: vec![ExtenderPlugin {
            file_name: "skee64.dll".to_owned(),
            status: ExtenderPluginStatus::IncompatibleGameVersion,
        }],
    });
    facts
        .versions
        .newer_versions
        .insert("skee64-id".to_owned(), "0.4.21".to_owned());
    let findings = run_all(&Context::new(&facts, &ByStem));

    assert_eq!(
        kinds_of(&findings),
        [(
            "versions.plugin-wrong-game",
            "file:data/skse/plugins/skee64.dll".to_owned()
        )],
        "the log's own finding about it is not repeated"
    );
    let finding = &findings[0];
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(finding.args["builtFor"], ArgValue::from("1.6.1170"));
    assert_eq!(finding.args["gameVersion"], ArgValue::from("1.7.104"));
    assert_eq!(finding.args["confirmed"], ArgValue::from("yes"));
    assert_eq!(finding.args["update"], ArgValue::from("0.4.21"));
    assert!(matches!(
        finding.actions.as_slice(),
        [Action::DisableMod { mod_ref }] if mod_ref.name == "skee64"
    ));
}

#[test]
fn a_plugin_is_judged_before_the_game_is_started_again() {
    // The log still tells of 1.6.1170; Steam has installed 1.7.104 since.
    let mut facts = updated(vec![bound("skee64.dll", &["1.6.1170"])]);
    facts.extender = Some(ExtenderReport {
        extender_version: Some("2.3.1".to_owned()),
        game_version: Some("1.6.1170".to_owned()),
        plugin_dir: GamePath::new("data/skse/plugins"),
        plugins: Vec::new(),
    });
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].args["confirmed"], ArgValue::from("no"));
    assert_eq!(findings[0].args["update"], ArgValue::from(""));
}

#[test]
fn the_script_extenders_rule_decides_which_plugins_fit() {
    let game = version("1.7.104");
    let extender = Some(version("2.3.1"));
    let data = |al: bool, sig: bool, s629: bool, none: bool, list: &[&str]| {
        PluginDeclaration::VersionData(PluginVersionData {
            address_library: al,
            signatures: sig,
            structs_post_629: s629,
            no_struct_use: none,
            compatible: list.iter().map(|text| version(text)).collect(),
            ..PluginVersionData::default()
        })
    };
    // Shapes taken from the version data of real plugins, with made-up
    // names: independent with stray entries in the list, bound to one
    // version, independent by patterns, half independent.
    let cases = [
        (data(true, false, true, false, &["1.7.99"]), Verdict::Fits),
        (data(true, false, false, true, &["1.0.0"]), Verdict::Fits),
        (data(false, true, false, true, &[]), Verdict::Fits),
        (
            data(false, false, false, false, &["1.7.104"]),
            Verdict::Fits,
        ),
        (data(false, false, true, false, &["1.7.104"]), Verdict::Fits),
        (
            data(false, false, true, false, &["1.6.1170"]),
            Verdict::WrongGame,
        ),
        (
            data(true, false, false, false, &["1.6.1170"]),
            Verdict::WrongGame,
        ),
        (data(false, false, false, false, &[]), Verdict::WrongGame),
        (PluginDeclaration::QueryOnly, Verdict::BeforeAnniversary),
        (PluginDeclaration::Library, Verdict::Fits),
        (PluginDeclaration::Unreadable, Verdict::Fits),
    ];
    for (declaration, expected) in cases {
        assert_eq!(
            verdict(&declaration, game, extender),
            expected,
            "{declaration:?}"
        );
    }
    // Before the Anniversary update the old question is the right one.
    assert_eq!(
        verdict(&PluginDeclaration::QueryOnly, version("1.5.97"), extender),
        Verdict::Fits
    );
}

#[test]
fn a_plugin_may_need_a_newer_script_extender() {
    let mut data = PluginVersionData {
        address_library: true,
        structs_post_629: true,
        extender_required: Some(version("2.4.0")),
        ..PluginVersionData::default()
    };
    let facts = updated(vec![declared("New.dll", data.clone())]);
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(
        kinds_of(&findings)[0].0,
        "versions.plugin-needs-newer-extender"
    );
    assert_eq!(findings[0].args["required"], ArgValue::from("2.4.0"));
    assert_eq!(findings[0].args["extenderVersion"], ArgValue::from("2.3.1"));

    data.extender_required = Some(version("2.3.0"));
    let facts = updated(vec![declared("New.dll", data)]);
    assert_eq!(
        run_all(&Context::new(&facts, &NoModSource)),
        Vec::<Finding>::new()
    );
}

#[test]
fn a_mod_update_may_need_a_newer_game() {
    let mut facts = updated(vec![bound("Next.dll", &["1.7.110"])]);
    facts.versions.game = Some(version("1.7.104"));
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(
        kinds_of(&findings),
        [(
            "versions.plugin-needs-newer-game",
            "file:data/skse/plugins/next.dll".to_owned()
        )]
    );
    assert_eq!(findings[0].args["builtFor"], ArgValue::from("1.7.110"));
}

#[test]
fn a_plugin_for_the_game_before_1_6_is_reported_and_a_library_is_not() {
    let facts = updated(vec![
        plugin("Old.dll", PluginDeclaration::QueryOnly),
        plugin("tbb.dll", PluginDeclaration::Library),
    ]);
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(
        kinds_of(&findings),
        [(
            "versions.plugin-before-anniversary",
            "file:data/skse/plugins/old.dll".to_owned()
        )]
    );
}

#[test]
fn a_script_extender_without_a_library_for_the_game_is_one_finding() {
    let mut facts = updated(vec![independent("A.dll"), independent("B.dll")]);
    facts.versions.extender = Some(skse(&["1.6.1170"]));
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(
        kinds_of(&findings),
        [("versions.extender-lacks-game", "setup".to_owned())]
    );
    assert_eq!(findings[0].args["newest"], ArgValue::from("1.6.1170"));
    assert_eq!(findings[0].args["plugins"], ArgValue::from(2));
    assert_eq!(findings[0].args["extender"], ArgValue::from("SKSE"));
}

#[test]
fn the_address_library_must_have_the_game_version_for_the_plugins_that_use_it() {
    let mut facts = updated(vec![
        independent("A.dll"),
        independent("B.dll"),
        bound("C.dll", &["1.7.104"]),
    ]);
    facts.versions.address_library = Some(vec![version("1.6.1170")]);
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(
        kinds_of(&findings),
        [("versions.address-library-lacks-game", "setup".to_owned())]
    );
    assert_eq!(findings[0].args["count"], ArgValue::from(2));
    assert_eq!(findings[0].args["examples"], ArgValue::from("A.dll, B.dll"));

    facts.versions.address_library = Some(Vec::new());
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(
        kinds_of(&findings),
        [("versions.address-library-missing", "setup".to_owned())]
    );

    // Nobody needs it: nothing to say.
    facts.versions.native_plugins = vec![bound("C.dll", &["1.7.104"])];
    assert_eq!(
        run_all(&Context::new(&facts, &NoModSource)),
        Vec::<Finding>::new()
    );
}

#[test]
fn without_a_game_version_nothing_is_judged() {
    let mut facts = updated(vec![bound("skee64.dll", &["1.6.1170"])]);
    facts.versions.game = None;
    assert_eq!(
        run_all(&Context::new(&facts, &NoModSource)),
        Vec::<Finding>::new()
    );
}

#[test]
fn the_log_still_speaks_for_files_that_were_not_read() {
    let facts = Facts {
        extender: Some(ExtenderReport {
            extender_version: Some("2.3.1".to_owned()),
            game_version: Some("1.7.104".to_owned()),
            plugin_dir: GamePath::new("data/skse/plugins"),
            plugins: vec![ExtenderPlugin {
                file_name: "skee64.dll".to_owned(),
                status: ExtenderPluginStatus::IncompatibleGameVersion,
            }],
        }),
        ..Facts::default()
    };
    let findings = run_all(&Context::new(&facts, &NoModSource));
    assert_eq!(kinds_of(&findings)[0].0, "extender.plugin-incompatible");
}

fn mod_info(id: &str, name: &str, version: &str) -> ModInfo {
    ModInfo {
        mod_ref: ModRef {
            source: "vortex".to_owned(),
            id: id.to_owned(),
            name: name.to_owned(),
        },
        enabled: true,
        version: Some(version.to_owned()),
        newer_version: None,
        author: None,
        page: Some(format!("https://example.invalid/{name}")),
    }
}

fn report(scanned_at: u64, versions: VersionSnapshot, mods: Vec<ModInfo>) -> ScanReport {
    ScanReport {
        schema_version: 1,
        scanned_at,
        game: GameInfo {
            id: "skyrimse".to_owned(),
            name: "Skyrim".to_owned(),
            version: versions.game.clone(),
            extender_version: versions.extender.clone(),
            unreal_visuals: false,
        },
        extender_plugins_checked: 0,
        plugins_checked: 0,
        archives_checked: 0,
        installers_checked: 0,
        meshes_checked: 0,
        locations: Vec::new(),
        findings: Vec::new(),
        mods,
        map: mallet_core::WorldMap::default(),
        object_swapper: false,
        installed_patch: None,
        duels: Vec::new(),
        unreal_paks: Vec::new(),
        switched_off_plugins: Vec::new(),
        manager: None,
        load_order: Vec::new(),
        versions,
        version_changes: None,
    }
}

fn seen(game: &str, plugins: &[(&str, &str)]) -> VersionSnapshot {
    VersionSnapshot {
        game: Some(game.to_owned()),
        last_started: Some(game.to_owned()),
        extender: Some("2.3.1".to_owned()),
        plugins: plugins
            .iter()
            .map(|(file, version)| PluginVersion {
                file: (*file).to_owned(),
                version: Some((*version).to_owned()),
            })
            .collect(),
    }
}

const DAY: u64 = 24 * 60 * 60;

#[test]
fn a_game_update_since_the_last_scan_is_a_change() {
    let before = report(
        1000,
        seen("1.6.1170", &[("skee64.dll", "0.4.19")]),
        Vec::new(),
    );
    let now = seen("1.7.104", &[("skee64.dll", "0.4.19")]);
    let changes = changes(Some(&before), &now, &[], 5000).unwrap();
    assert_eq!(changes.since, 1000);
    assert_eq!(changes.noticed, 5000);
    assert_eq!(
        changes.game,
        Some(VersionStep {
            from: "1.6.1170".to_owned(),
            to: "1.7.104".to_owned()
        })
    );
    assert_eq!(changes.extender, None);
    assert_eq!(changes.plugins, []);
}

#[test]
fn nothing_changed_is_no_change() {
    let before = report(1000, seen("1.7.104", &[]), vec![mod_info("a", "A", "1.0")]);
    assert_eq!(
        changes(
            Some(&before),
            &seen("1.7.104", &[]),
            &[mod_info("a", "A", "1.0")],
            5000
        ),
        None
    );
}

#[test]
fn updated_plugins_and_mods_are_changes_even_under_a_new_identifier() {
    let before = report(
        1000,
        seen("1.7.104", &[("skee64.dll", "0.4.19.16")]),
        vec![mod_info("RaceMenu-0-4-19", "RaceMenu", "0.4.19")],
    );
    let now = seen("1.7.104", &[("skee64.dll", "0.4.20.0")]);
    let changes = changes(
        Some(&before),
        &now,
        &[mod_info("RaceMenu-0-4-20", "RaceMenu", "0.4.20")],
        5000,
    )
    .unwrap();
    assert_eq!(changes.game, None);
    assert_eq!(changes.plugins[0].from, "0.4.19.16");
    assert_eq!(changes.plugins[0].to, "0.4.20.0");
    assert_eq!(changes.mods[0].mod_ref.id, "RaceMenu-0-4-20");
    assert_eq!(changes.mods[0].from, "0.4.19");
}

#[test]
fn a_report_from_before_kept_its_game_version_from_the_log_only() {
    let mut before = report(1000, VersionSnapshot::default(), Vec::new());
    before.game.version = Some("1.6.1170".to_owned());
    let changes = changes(Some(&before), &seen("1.7.104", &[]), &[], 5000).unwrap();
    assert_eq!(changes.game.unwrap().from, "1.6.1170");
}

#[test]
fn without_an_earlier_scan_the_log_of_the_last_start_tells_of_an_update() {
    let mut now = seen("1.7.104", &[]);
    now.last_started = Some("1.6.1170".to_owned());
    let found = changes(None, &now, &[], 5000).unwrap();
    assert_eq!(found.since, 0);
    assert_eq!(found.game.unwrap().from, "1.6.1170");
    assert_eq!(changes(None, &seen("1.7.104", &[]), &[], 5000), None);
}

#[test]
fn a_change_is_kept_while_it_breaks_something_and_for_a_week() {
    let mut before = report(5000, seen("1.7.104", &[]), Vec::new());
    before.version_changes = Some(VersionChanges {
        since: 1000,
        noticed: 5000,
        game: Some(VersionStep {
            from: "1.6.1170".to_owned(),
            to: "1.7.104".to_owned(),
        }),
        unfit: vec!["skee64.dll".to_owned()],
        ..VersionChanges::default()
    });
    let now = seen("1.7.104", &[]);
    let kept = changes(Some(&before), &now, &[], 5000 + 30 * DAY).unwrap();
    assert_eq!(kept.since, 1000);
    assert_eq!(kept.game.unwrap().from, "1.6.1170");

    // Nothing broken any more: kept for a week, then gone.
    before.version_changes.as_mut().unwrap().unfit.clear();
    assert!(changes(Some(&before), &now, &[], 5000 + 6 * DAY).is_some());
    assert_eq!(changes(Some(&before), &now, &[], 5000 + 8 * DAY), None);
}

#[test]
fn changes_in_a_row_go_from_the_first_version_to_the_last() {
    let mut before = report(5000, seen("1.7.99", &[]), Vec::new());
    before.version_changes = Some(VersionChanges {
        since: 1000,
        noticed: 5000,
        game: Some(VersionStep {
            from: "1.6.1170".to_owned(),
            to: "1.7.99".to_owned(),
        }),
        unfit: vec!["skee64.dll".to_owned()],
        ..VersionChanges::default()
    });
    let changes = changes(Some(&before), &seen("1.7.104", &[]), &[], 9000).unwrap();
    assert_eq!(changes.since, 1000);
    assert_eq!(changes.noticed, 9000);
    assert_eq!(
        changes.game,
        Some(VersionStep {
            from: "1.6.1170".to_owned(),
            to: "1.7.104".to_owned()
        })
    );

    // Rolled back to where it started: the game step is gone.
    let mut back = report(9000, seen("1.7.104", &[]), Vec::new());
    back.version_changes = Some(changes);
    assert_eq!(
        super::changes(Some(&back), &seen("1.6.1170", &[]), &[], 9500).and_then(|c| c.game),
        None
    );
}

#[test]
fn what_no_longer_fits_is_taken_from_the_findings() {
    let facts = updated(vec![
        bound("skee64.dll", &["1.6.1170"]),
        plugin("Old.dll", PluginDeclaration::QueryOnly),
        independent("Fine.dll"),
    ]);
    let findings = run_all(&Context::new(&facts, &NoModSource));
    let changes = with_unfit(VersionChanges::default(), &findings);
    assert_eq!(changes.unfit, ["Old.dll", "skee64.dll"]);
    assert!(!changes.foundation_unfit);
}

#[test]
fn a_patch_whose_masters_mod_was_updated_under_it_is_a_hint() {
    let master = |name: &str| PluginFacts {
        name: name.to_owned(),
        path: GamePath::new("data").join(name),
        masters: Vec::new(),
        npcs: Vec::new(),
    };
    let mut patch = master("Patch.esp");
    patch.masters = vec![
        "Skyrim.esm".to_owned(),
        "Big.esp".to_owned(),
        "Other.esp".to_owned(),
    ];
    let mut facts = Facts {
        plugins: vec![
            master("Skyrim.esm"),
            master("Big.esp"),
            master("Other.esp"),
            patch,
        ],
        ..Facts::default()
    };
    facts.versions.changes = Some(VersionChanges {
        mods: vec![ModStep {
            mod_ref: ModRef {
                source: "test".to_owned(),
                id: "big-id".to_owned(),
                name: "Big".to_owned(),
            },
            from: "1.2".to_owned(),
            to: "2.0".to_owned(),
        }],
        ..VersionChanges::default()
    });
    let findings = run_all(&Context::new(&facts, &ByStem));
    assert_eq!(
        kinds_of(&findings),
        [("versions.master-updated", "file:data/patch.esp".to_owned())]
    );
    assert_eq!(
        findings[0].args["masters"],
        ArgValue::from("Big.esp (Big: 1.2 → 2.0)")
    );
    assert_eq!(findings[0].args["mod"], ArgValue::from("patch"));
    assert_eq!(findings[0].args["count"], ArgValue::from(1));
}

#[test]
fn every_kind_declares_the_arguments_its_findings_carry() {
    let mut facts = updated(vec![
        bound("skee64.dll", &["1.6.1170"]),
        plugin("Old.dll", PluginDeclaration::QueryOnly),
        declared(
            "New.dll",
            PluginVersionData {
                address_library: true,
                structs_post_629: true,
                extender_required: Some(version("9.0.0")),
                ..PluginVersionData::default()
            },
        ),
        independent("A.dll"),
        bound("Next.dll", &["1.7.110"]),
    ]);
    facts.knowledge.version_messages = vec![mallet_core::facts::knowledge::KnownMessage {
        level: mallet_core::facts::knowledge::MessageLevel::Warning,
        text: "A newer version of SKSE is available.".to_owned(),
    }];
    let mut findings = run_all(&Context::new(&facts, &NoModSource));
    facts.versions.extender = Some(skse(&["1.6.1170"]));
    facts.versions.address_library = Some(Vec::new());
    findings.extend(run_all(&Context::new(&facts, &NoModSource)));
    let found: BTreeSet<&str> = findings
        .iter()
        .map(|finding| finding.kind.as_str())
        .collect();
    assert_eq!(found.len(), 7, "{found:?}");

    for finding in findings {
        let kind = kinds::find(&finding.kind).unwrap();
        let mut declared: Vec<_> = kind.args.to_vec();
        declared.sort_unstable();
        let carried: Vec<_> = finding.args.keys().map(String::as_str).collect();
        assert_eq!(carried, declared, "arguments of {}", kind.id);
    }
}
