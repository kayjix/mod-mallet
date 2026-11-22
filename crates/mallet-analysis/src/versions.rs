//! What a game update or a mod update did to a setup (ADR 0079): which
//! script extender plugins no longer fit the game, whether the script
//! extender and the Address Library still do, which patches build on mods
//! that were updated under them, and what changed since the last scan.
//!
//! The script extender decides about a plugin by its version data, and
//! this follows the same rule, so that the answer is known before the game
//! is started: a plugin that finds the game's code without fixed addresses
//! (through the Address Library or by patterns) *and* does not depend on
//! the layout of the game's structures runs in any game version; any other
//! plugin runs only in the versions it lists.

use std::collections::{BTreeMap, BTreeSet};

use mallet_core::facts::ExtenderPluginStatus;
use mallet_core::facts::versions::{
    NativePlugin, PluginDeclaration, PluginVersionData, RuntimeVersion,
};
use mallet_core::{
    Finding, ModInfo, ModRef, ModStep, PluginStep, PluginVersion, ScanReport, Subject,
    VersionChanges, VersionSnapshot, VersionStep, kinds,
};

use crate::{Analysis, Context};

/// The first version of the game since which the script extender reads
/// plugins' version data, Skyrim's Anniversary update.
const ANNIVERSARY: RuntimeVersion = RuntimeVersion::new(1, 6, 317, 0);

/// How many plugins a finding about many names.
const EXAMPLES: usize = 5;

/// Reports what no longer fits the version of the game or of the script
/// extender, and patches whose masters' mods were updated under them.
pub struct VersionChecks;

impl Analysis for VersionChecks {
    fn id(&self) -> &'static str {
        "versions"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        if let Some(game) = game_version(context) {
            findings.extend(extender_finding(context, game));
            findings.extend(address_library_finding(context, game));
            findings.extend(
                context
                    .facts
                    .versions
                    .native_plugins
                    .iter()
                    .filter_map(|plugin| plugin_finding(context, plugin, game)),
            );
        }
        findings.extend(master_findings(context));
        findings.extend(loot_finding(context));
        findings
    }
}

/// What the script extender will make of a plugin in a game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It loads.
    Fits,
    /// It was built for other game versions only.
    WrongGame,
    /// It was built for the game before version 1.6.
    BeforeAnniversary,
    /// It needs a newer script extender.
    NeedsNewerExtender(RuntimeVersion),
}

/// What the script extender in `extender` will make of `plugin` in the
/// game version `game`.
pub fn verdict(
    plugin: &PluginDeclaration,
    game: RuntimeVersion,
    extender: Option<RuntimeVersion>,
) -> Verdict {
    match plugin {
        PluginDeclaration::VersionData(data) => {
            if !fits_game(data, game) {
                Verdict::WrongGame
            } else if let (Some(required), Some(installed)) = (data.extender_required, extender)
                && required > installed
            {
                Verdict::NeedsNewerExtender(required)
            } else {
                Verdict::Fits
            }
        }
        PluginDeclaration::QueryOnly if game >= ANNIVERSARY => Verdict::BeforeAnniversary,
        _ => Verdict::Fits,
    }
}

/// Whether the script extender accepts a plugin with `data` in the game
/// version `game`.
fn fits_game(data: &PluginVersionData, game: RuntimeVersion) -> bool {
    // A way of independence the extender does not know is its own reason
    // to refuse; Mod Mallet does not guess about it.
    if data.unknown_independence {
        return true;
    }
    let finds_code = data.address_library || data.signatures;
    let structures = data.structs_post_629 || data.no_struct_use;
    (finds_code && structures) || data.compatible.contains(&game)
}

/// Whether a plugin finds the game's code through the Address Library,
/// whose file for the game version it reads when the game starts.
fn needs_address_library(data: &PluginVersionData) -> bool {
    data.address_library && !data.signatures
}

/// The game's version: from its program, or else from the script
/// extender's log of its last start.
fn game_version(context: &Context<'_>) -> Option<RuntimeVersion> {
    context.facts.versions.game.or_else(|| {
        context
            .facts
            .extender
            .as_ref()?
            .game_version
            .as_deref()
            .and_then(RuntimeVersion::parse)
    })
}

/// The script extender's version: from its loader, or else from its log.
fn extender_version(context: &Context<'_>) -> Option<RuntimeVersion> {
    let versions = &context.facts.versions;
    versions
        .extender
        .as_ref()
        .and_then(|extender| extender.version)
        .or_else(|| {
            context
                .facts
                .extender
                .as_ref()?
                .extender_version
                .as_deref()
                .and_then(RuntimeVersion::parse)
        })
}

/// The script extender has no library for the game version.
fn extender_finding(context: &Context<'_>, game: RuntimeVersion) -> Option<Finding> {
    let installed = context.facts.versions.extender.as_ref()?;
    if installed.supports(game) {
        return None;
    }
    let plugins = context
        .facts
        .versions
        .native_plugins
        .iter()
        .filter(|plugin| matches!(plugin.declaration, PluginDeclaration::VersionData(_)))
        .count();
    Some(
        Finding::new(&kinds::VERSIONS_EXTENDER_LACKS_GAME, Subject::Setup)
            .with_arg("extender", installed.name.as_str())
            .with_arg("gameVersion", game.to_string())
            .with_arg(
                "extenderVersion",
                extender_version(context).map_or_else(String::new, |v| v.to_string()),
            )
            .with_arg(
                "newest",
                installed
                    .runtimes
                    .iter()
                    .max()
                    .map_or_else(String::new, ToString::to_string),
            )
            .with_arg("plugins", i64::try_from(plugins).unwrap_or(i64::MAX)),
    )
}

/// The Address Library has no file for the game version, while plugins
/// rely on it.
fn address_library_finding(context: &Context<'_>, game: RuntimeVersion) -> Option<Finding> {
    let library = context.facts.versions.address_library.as_ref()?;
    if library.contains(&game) {
        return None;
    }
    let reliant: Vec<&str> = context
        .facts
        .versions
        .native_plugins
        .iter()
        .filter(|plugin| match &plugin.declaration {
            PluginDeclaration::VersionData(data) => {
                fits_game(data, game) && needs_address_library(data)
            }
            _ => false,
        })
        .map(|plugin| plugin.file_name.as_str())
        .collect();
    if reliant.is_empty() {
        return None;
    }
    let count = i64::try_from(reliant.len()).unwrap_or(i64::MAX);
    let examples = crate::examples(reliant, EXAMPLES);
    Some(if library.is_empty() {
        Finding::new(&kinds::VERSIONS_ADDRESS_LIBRARY_MISSING, Subject::Setup)
            .with_arg("count", count)
            .with_arg("examples", examples)
    } else {
        Finding::new(&kinds::VERSIONS_ADDRESS_LIBRARY_LACKS_GAME, Subject::Setup)
            .with_arg("gameVersion", game.to_string())
            .with_arg("count", count)
            .with_arg("examples", examples)
    })
}

/// The finding about one plugin that does not fit, if it does not.
fn plugin_finding(
    context: &Context<'_>,
    plugin: &NativePlugin,
    game: RuntimeVersion,
) -> Option<Finding> {
    let extender = extender_version(context);
    let subject = Subject::File {
        path: plugin.path.clone(),
    };
    let provider = context.mods.provider_of(&plugin.path);
    let update = provider
        .as_ref()
        .and_then(|found| context.facts.versions.newer_versions.get(&found.id))
        .cloned()
        .unwrap_or_default();
    let file = plugin.file_name.as_str();
    let finding = match verdict(&plugin.declaration, game, extender) {
        Verdict::Fits => return None,
        Verdict::WrongGame => {
            let PluginDeclaration::VersionData(data) = &plugin.declaration else {
                return None;
            };
            let built_for: BTreeSet<RuntimeVersion> = data.compatible.iter().copied().collect();
            let listed = built_for
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let confirmed = if confirmed_by_log(context, file, game) {
                "yes"
            } else {
                "no"
            };
            // Built only for newer versions: the mod was updated for a game
            // update this setup does not have yet.
            if !built_for.is_empty() && built_for.iter().all(|version| *version > game) {
                Finding::new(&kinds::VERSIONS_PLUGIN_NEEDS_NEWER_GAME, subject)
                    .with_arg("file", file)
                    .with_arg("gameVersion", game.to_string())
                    .with_arg("builtFor", listed)
                    .with_arg("confirmed", confirmed)
            } else {
                Finding::new(&kinds::VERSIONS_PLUGIN_WRONG_GAME, subject)
                    .with_arg("file", file)
                    .with_arg("gameVersion", game.to_string())
                    .with_arg("builtFor", listed)
                    .with_arg("confirmed", confirmed)
                    .with_arg("updateKnown", if update.is_empty() { "no" } else { "yes" })
                    .with_arg("update", update)
            }
        }
        Verdict::BeforeAnniversary => {
            Finding::new(&kinds::VERSIONS_PLUGIN_BEFORE_ANNIVERSARY, subject)
                .with_arg("file", file)
                .with_arg("gameVersion", game.to_string())
                .with_arg("updateKnown", if update.is_empty() { "no" } else { "yes" })
                .with_arg("update", update)
        }
        Verdict::NeedsNewerExtender(required) => {
            Finding::new(&kinds::VERSIONS_PLUGIN_NEEDS_NEWER_EXTENDER, subject)
                .with_arg("file", file)
                .with_arg("required", required.to_string())
                .with_arg(
                    "extenderVersion",
                    extender.map_or_else(String::new, |v| v.to_string()),
                )
        }
    };
    Some(crate::offer_disabling(finding.with_provider(provider)))
}

/// Whether the script extender's log of the last start, in the same game
/// version, says it switched the plugin off for the game version.
fn confirmed_by_log(context: &Context<'_>, file: &str, game: RuntimeVersion) -> bool {
    context.facts.extender.as_ref().is_some_and(|report| {
        report
            .game_version
            .as_deref()
            .and_then(RuntimeVersion::parse)
            == Some(game)
            && report.plugins.iter().any(|logged| {
                logged.file_name.eq_ignore_ascii_case(file)
                    && logged.status == ExtenderPluginStatus::IncompatibleGameVersion
            })
    })
}

/// Whether the version checks speak about the script extender plugin
/// `file` themselves, so that the log of the last start need not: the
/// file was read, and the game's version is known.
pub fn covers(context: &Context<'_>, file: &str) -> bool {
    game_version(context).is_some()
        && context.facts.versions.native_plugins.iter().any(|plugin| {
            plugin.file_name.eq_ignore_ascii_case(file)
                && matches!(
                    plugin.declaration,
                    PluginDeclaration::VersionData(_) | PluginDeclaration::QueryOnly
                )
        })
}

/// Plugins that build on plugins whose mods were updated since an earlier
/// scan, while their own mod was not.
fn master_findings(context: &Context<'_>) -> Vec<Finding> {
    let Some(changes) = &context.facts.versions.changes else {
        return Vec::new();
    };
    if changes.mods.is_empty() {
        return Vec::new();
    }
    let updated: BTreeMap<&str, &ModStep> = changes
        .mods
        .iter()
        .map(|step| (step.mod_ref.id.as_str(), step))
        .collect();
    let provider_of: BTreeMap<String, Option<ModRef>> = context
        .facts
        .plugins
        .iter()
        .map(|plugin| {
            (
                plugin.name.to_lowercase(),
                context.mods.provider_of(&plugin.path),
            )
        })
        .collect();
    let mut findings = Vec::new();
    for plugin in &context.facts.plugins {
        let Some(Some(own)) = provider_of.get(&plugin.name.to_lowercase()) else {
            continue;
        };
        if updated.contains_key(own.id.as_str()) {
            continue;
        }
        let moved: Vec<String> = plugin
            .masters
            .iter()
            .filter_map(|master| {
                let provider = provider_of.get(&master.to_lowercase())?.as_ref()?;
                if provider.id == own.id {
                    return None;
                }
                let step = updated.get(provider.id.as_str())?;
                Some(format!(
                    "{master} ({}: {} → {})",
                    step.mod_ref.name, step.from, step.to
                ))
            })
            .collect();
        if moved.is_empty() {
            continue;
        }
        findings.push(
            Finding::new(
                &kinds::VERSIONS_MASTER_UPDATED,
                Subject::File {
                    path: plugin.path.clone(),
                },
            )
            .with_arg("plugin", plugin.name.as_str())
            .with_arg("mod", own.name.as_str())
            .with_arg("count", i64::try_from(moved.len()).unwrap_or(i64::MAX))
            .with_arg("masters", moved.join(", "))
            .with_provider(Some(own.clone())),
        );
    }
    findings
}

/// LOOT's warnings about the versions of the game and its script
/// extender, quoted.
fn loot_finding(context: &Context<'_>) -> Option<Finding> {
    let messages = &context.facts.knowledge.version_messages;
    if messages.is_empty() {
        return None;
    }
    Some(
        Finding::new(&kinds::VERSIONS_LOOT_WARNING, Subject::Setup)
            .with_arg("count", i64::try_from(messages.len()).unwrap_or(i64::MAX))
            .with_arg(
                "messages",
                messages
                    .iter()
                    .map(|message| message.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
    )
}

/// The versions a scan with `facts` saw.
pub fn snapshot(facts: &mallet_core::Facts) -> VersionSnapshot {
    let versions = &facts.versions;
    let last_started = facts
        .extender
        .as_ref()
        .and_then(|report| report.game_version.clone());
    VersionSnapshot {
        game: versions
            .game
            .map(|version| version.to_string())
            .or_else(|| last_started.clone()),
        last_started,
        extender: versions
            .extender
            .as_ref()
            .and_then(|extender| extender.version)
            .map(|version| version.to_string())
            .or_else(|| {
                facts
                    .extender
                    .as_ref()
                    .and_then(|report| report.extender_version.clone())
            }),
        plugins: versions
            .native_plugins
            .iter()
            .map(|plugin| PluginVersion {
                file: plugin.file_name.clone(),
                version: plugin.file_version.clone(),
            })
            .collect(),
    }
}

/// What changed between the `previous` scan and this one, which saw
/// `current` and the mods `mods`, at the time `now`. Changes the previous
/// scan reported are kept while something they broke is still broken, and
/// for [`VersionChanges::KEPT_FOR`] after they were noticed (ADR 0079).
///
/// Without a previous scan, a game updated since its last start (by the
/// script extender's log) still counts as a change.
pub fn changes(
    previous: Option<&ScanReport>,
    current: &VersionSnapshot,
    mods: &[ModInfo],
    now: u64,
) -> Option<VersionChanges> {
    let fresh = match previous {
        Some(previous) => difference(previous, current, mods, now),
        None => started_before_update(current, now),
    };
    let carried = previous
        .and_then(|previous| previous.version_changes.clone())
        .filter(|kept| {
            !kept.unfit.is_empty()
                || kept.foundation_unfit
                || now.saturating_sub(kept.noticed) < VersionChanges::KEPT_FOR
        });
    let merged = match (carried, fresh) {
        (None, fresh) => fresh,
        (Some(carried), None) => Some(carried),
        (Some(carried), Some(fresh)) => Some(merge(carried, fresh)),
    };
    merged.filter(|changes| !changes.is_empty())
}

/// The game updated since it was last started, by the script extender's
/// log, as a change without an earlier scan.
fn started_before_update(current: &VersionSnapshot, now: u64) -> Option<VersionChanges> {
    let game = step(current.last_started.as_deref(), current.game.as_deref())?;
    Some(VersionChanges {
        since: 0,
        noticed: now,
        game: Some(game),
        ..VersionChanges::default()
    })
}

/// What differs between the previous scan and this one.
fn difference(
    previous: &ScanReport,
    current: &VersionSnapshot,
    mods: &[ModInfo],
    now: u64,
) -> Option<VersionChanges> {
    // Reports made before ADR 0079 know the game version from the log only.
    let previous_game = previous
        .versions
        .game
        .as_deref()
        .or(previous.game.version.as_deref());
    let previous_extender = previous
        .versions
        .extender
        .as_deref()
        .or(previous.game.extender_version.as_deref());
    let plugins = current
        .plugins
        .iter()
        .filter_map(|now| {
            let before = previous
                .versions
                .plugins
                .iter()
                .find(|before| before.file.eq_ignore_ascii_case(&now.file))?;
            let moved = step(before.version.as_deref(), now.version.as_deref())?;
            Some(PluginStep {
                file: now.file.clone(),
                from: moved.from,
                to: moved.to,
            })
        })
        .collect();
    let changes = VersionChanges {
        since: previous.scanned_at,
        noticed: now,
        game: step(previous_game, current.game.as_deref()),
        extender: step(previous_extender, current.extender.as_deref()),
        plugins,
        mods: mod_steps(&previous.mods, mods),
        ..VersionChanges::default()
    };
    (!changes.is_empty()).then_some(changes)
}

/// The step between two versions, if both are known and differ.
fn step(from: Option<&str>, to: Option<&str>) -> Option<VersionStep> {
    let (from, to) = (from?, to?);
    (from != to).then(|| VersionStep {
        from: from.to_owned(),
        to: to.to_owned(),
    })
}

/// The mods whose versions changed. A mod manager may give an updated mod
/// a new identifier, so a mod that is gone is matched with a new one from
/// the same page under the same name.
fn mod_steps(before: &[ModInfo], now: &[ModInfo]) -> Vec<ModStep> {
    let same = |a: &ModInfo, b: &ModInfo| {
        a.mod_ref.id == b.mod_ref.id
            || (a.page.is_some() && a.page == b.page && a.mod_ref.name == b.mod_ref.name)
    };
    let mut steps: Vec<ModStep> = now
        .iter()
        .filter_map(|current| {
            let earlier = before
                .iter()
                .find(|earlier| earlier.mod_ref.id == current.mod_ref.id)
                .or_else(|| before.iter().find(|earlier| same(earlier, current)))?;
            let moved = step(earlier.version.as_deref(), current.version.as_deref())?;
            Some(ModStep {
                mod_ref: current.mod_ref.clone(),
                from: moved.from,
                to: moved.to,
            })
        })
        .collect();
    steps.sort_by(|a, b| {
        a.mod_ref
            .name
            .to_lowercase()
            .cmp(&b.mod_ref.name.to_lowercase())
    });
    steps
}

/// Changes kept from earlier scans followed by new ones: each thing goes
/// from its earliest known version to its latest, and is left out once it
/// is back where it started.
fn merge(carried: VersionChanges, fresh: VersionChanges) -> VersionChanges {
    let chain = |earlier: Option<VersionStep>, later: Option<VersionStep>| match (earlier, later) {
        (Some(earlier), Some(later)) => step(Some(&earlier.from), Some(&later.to)),
        (earlier, None) => earlier,
        (None, later) => later,
    };
    let mut plugins = carried.plugins;
    for later in fresh.plugins {
        match plugins
            .iter()
            .position(|earlier| earlier.file == later.file)
        {
            Some(index) => {
                let from = plugins.remove(index).from;
                if from != later.to {
                    plugins.push(PluginStep { from, ..later });
                }
            }
            None => plugins.push(later),
        }
    }
    let mut mods = carried.mods;
    for later in fresh.mods {
        let matching = mods.iter().position(|earlier| {
            earlier.mod_ref.id == later.mod_ref.id
                || earlier.to == later.from && earlier.mod_ref.name == later.mod_ref.name
        });
        match matching {
            Some(index) => {
                let from = mods.remove(index).from;
                if from != later.to {
                    mods.push(ModStep { from, ..later });
                }
            }
            None => mods.push(later),
        }
    }
    VersionChanges {
        since: carried.since,
        noticed: fresh.noticed,
        game: chain(carried.game, fresh.game),
        extender: chain(carried.extender, fresh.extender),
        plugins,
        mods,
        unfit: Vec::new(),
        foundation_unfit: false,
    }
}

/// `changes` with what no longer fits, from this scan's `findings`.
pub fn with_unfit(mut changes: VersionChanges, findings: &[Finding]) -> VersionChanges {
    let plugin_kinds = [
        kinds::VERSIONS_PLUGIN_WRONG_GAME.id,
        kinds::VERSIONS_PLUGIN_NEEDS_NEWER_GAME.id,
        kinds::VERSIONS_PLUGIN_BEFORE_ANNIVERSARY.id,
        kinds::VERSIONS_PLUGIN_NEEDS_NEWER_EXTENDER.id,
        kinds::EXTENDER_PLUGIN_INCOMPATIBLE.id,
    ];
    let mut unfit: Vec<String> = findings
        .iter()
        .filter(|finding| plugin_kinds.contains(&finding.kind.as_str()))
        .filter_map(|finding| match finding.args.get("file") {
            Some(mallet_core::ArgValue::Text(file)) => Some(file.clone()),
            _ => None,
        })
        .collect();
    unfit.sort_by_key(|file| file.to_lowercase());
    unfit.dedup();
    changes.unfit = unfit;
    changes.foundation_unfit = findings.iter().any(|finding| {
        finding.kind == kinds::VERSIONS_EXTENDER_LACKS_GAME.id
            || finding.kind == kinds::VERSIONS_ADDRESS_LIBRARY_LACKS_GAME.id
            || finding.kind == kinds::VERSIONS_ADDRESS_LIBRARY_MISSING.id
    });
    changes
}

#[cfg(test)]
mod tests;
