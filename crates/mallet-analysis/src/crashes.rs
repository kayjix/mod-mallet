//! What the recent crash reports point at.
//!
//! A crash report says where the crash happened and lists what was nearby.
//! Reading it is a matter of likelihood (ADR 0044):
//!
//! - A crash inside a program file that a mod brings, a DLL, is the
//!   strongest lead: that file's code was running when the game failed.
//! - A mod whose DLL is on the probable call stack, or whose records or
//!   meshes the report lists as relevant objects, may be involved. It may
//!   also just have been close by, which is why these are hints.
//! - A crash inside the game itself with nothing of a mod nearby says
//!   little about the mods; it is counted, not blamed on anyone.
//!
//! Files of the operating system, of graphics drivers and of the script
//! extender itself are never suspects.

use std::collections::BTreeMap;

use mallet_core::facts::crashes::CrashLog;
use mallet_core::{Finding, GamePath, ModRef, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many examples a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// How many probable frames from the crash outwards count for a DLL to be
/// on the call stack.
const STACK_DEPTH: usize = 6;

/// Program files that are part of Windows, the graphics drivers, the game
/// or the script extender, in lower case. A crash in one of them is not a
/// mod's doing as far as the report can tell.
const NOT_SUSPECTS: &[&str] = &[
    "kernelbase.dll",
    "kernel32.dll",
    "ntdll.dll",
    "ucrtbase.dll",
    "msvcrt.dll",
    "msvcp140.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "user32.dll",
    "win32u.dll",
    "gdi32.dll",
    "combase.dll",
    "dxgi.dll",
    "d3d11.dll",
    "d3dcompiler_47.dll",
    "xaudio2_7.dll",
    "x3daudio1_7.dll",
    "nvwgf2umx.dll",
    "nvwgf2um.dll",
    "nvoglv64.dll",
    "amdxx64.dll",
    "amdxc64.dll",
    "atidxx64.dll",
    "igd10iumd64.dll",
    "igd12umd64.dll",
    "steam_api64.dll",
    "gameoverlayrenderer64.dll",
];

/// Reports what the recent crash reports point at.
pub struct RecentCrashes;

/// What the reports say about one suspect.
#[derive(Default)]
struct Evidence<'a> {
    /// The reports that point at it, by file name.
    reports: Vec<&'a str>,
    /// The newest of their times.
    last: Option<&'a str>,
    /// What points at it: objects, meshes or program files.
    clues: Vec<String>,
}

impl<'a> Evidence<'a> {
    fn add(&mut self, log: &'a CrashLog, clue: Option<String>) {
        if !self.reports.contains(&log.file.as_str()) {
            self.reports.push(&log.file);
        }
        if let Some(time) = log.time.as_deref()
            && self.last.is_none_or(|last| time > last)
        {
            self.last = Some(time);
        }
        self.clues.extend(clue);
    }

    fn count(&self) -> i64 {
        i64::try_from(self.reports.len()).unwrap_or(i64::MAX)
    }
}

impl Analysis for RecentCrashes {
    fn id(&self) -> &'static str {
        "recent-crashes"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let crashes = &context.facts.crashes;
        let Some(newest) = crashes.logs.first() else {
            return Vec::new();
        };

        let mut in_dll: BTreeMap<String, Evidence<'_>> = BTreeMap::new();
        let mut involved: BTreeMap<ModRef, Evidence<'_>> = BTreeMap::new();
        for log in &crashes.logs {
            let crashed_in = log.module().filter(|module| is_suspect(module));
            if let Some(module) = crashed_in {
                in_dll
                    .entry(module.to_owned())
                    .or_default()
                    .add(log, log.address.clone());
            }
            for (mod_ref, clue) in clues(context, log, crashed_in) {
                involved.entry(mod_ref).or_default().add(log, Some(clue));
            }
        }

        let mut findings = vec![
            Finding::new(&kinds::CRASHES_RECENT, Subject::Setup)
                .with_arg(
                    "count",
                    i64::try_from(crashes.logs.len()).unwrap_or(i64::MAX),
                )
                .with_arg("days", i64::from(crashes.window_days))
                .with_arg(
                    "last",
                    newest.time.as_deref().unwrap_or(newest.file.as_str()),
                )
                .with_arg(
                    "examples",
                    examples(
                        crashes.logs.iter().filter_map(|log| log.address.as_deref()),
                        EXAMPLE_LIMIT,
                    ),
                ),
        ];
        for (module, evidence) in in_dll {
            let path = dll_path(&module);
            findings.push(
                Finding::new(&kinds::CRASHES_IN_DLL, Subject::File { path: path.clone() })
                    .with_arg("file", module.as_str())
                    .with_arg("count", evidence.count())
                    .with_arg("last", evidence.last.unwrap_or("-"))
                    .with_provider(context.mods.provider_of(&path)),
            );
        }
        for (mod_ref, evidence) in involved {
            findings.push(
                Finding::new(
                    &kinds::CRASHES_MOD_INVOLVED,
                    Subject::Mod {
                        source: mod_ref.source.clone(),
                        id: mod_ref.id.clone(),
                    },
                )
                .with_arg("count", evidence.count())
                .with_arg("last", evidence.last.unwrap_or("-"))
                .with_arg(
                    "examples",
                    examples(evidence.clues.iter().map(String::as_str), EXAMPLE_LIMIT),
                )
                .with_provider(Some(mod_ref)),
            );
        }
        findings
    }
}

/// The mods a report points at, other than through the program file the
/// crash happened in, each with what points at it.
fn clues(context: &Context<'_>, log: &CrashLog, crashed_in: Option<&str>) -> Vec<(ModRef, String)> {
    let mut clues = Vec::new();
    let data = GamePath::new("data");
    for module in log
        .stack_modules
        .iter()
        .take(STACK_DEPTH)
        .filter(|module| is_suspect(module) && Some(module.as_str()) != crashed_in)
    {
        if let Some(mod_ref) = context.mods.provider_of(&dll_path(module)) {
            clues.push((mod_ref, module.clone()));
        }
    }
    for object in &log.objects {
        if let Some(plugin) = &object.plugin
            && let Some(mod_ref) = context.mods.provider_of(&data.join(plugin))
        {
            let clue = match (&object.name, &object.kind) {
                (Some(name), _) => format!("{name} ({plugin})"),
                (None, Some(kind)) => format!("{kind} ({plugin})"),
                (None, None) => plugin.clone(),
            };
            clues.push((mod_ref, clue));
        }
        if let Some(mesh) = &object.mesh
            && let Some(mod_ref) = context.mods.provider_of(&mesh_path(mesh))
        {
            clues.push((mod_ref, mesh.clone()));
        }
    }
    clues
}

/// Whether a crash in the program file `module` may be a mod's doing.
fn is_suspect(module: &str) -> bool {
    let lower = module.to_ascii_lowercase();
    std::path::Path::new(&lower)
        .extension()
        .is_some_and(|extension| extension == "dll")
        && !NOT_SUSPECTS.contains(&lower.as_str())
        // The script extender's own library, such as `skse64_1_6_1170.dll`.
        && !lower.starts_with("skse64_")
        && !lower.starts_with("sksevr_")
        && !lower.starts_with("f4se_")
}

/// Where a script extender plugin of this name lies, as the game sees the
/// path.
fn dll_path(module: &str) -> GamePath {
    GamePath::new("data/skse/plugins").join(module)
}

/// The game path of a mesh as a crash report writes it: below `meshes`,
/// with or without that folder in front.
fn mesh_path(mesh: &str) -> GamePath {
    let path = GamePath::new(mesh);
    if path.as_str().starts_with("meshes/") {
        GamePath::new("data").join(path.as_str())
    } else {
        GamePath::new("data/meshes").join(path.as_str())
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::crashes::{CrashFacts, CrashLogger, CrashObject};
    use mallet_core::{ArgValue, Confidence, Facts, Severity};

    use super::*;
    use crate::assets::testing::Mods;
    use crate::run_all;

    fn log(
        file: &str,
        time: &str,
        address: &str,
        stack: &[&str],
        objects: Vec<CrashObject>,
    ) -> CrashLog {
        CrashLog {
            file: file.to_owned(),
            logger: CrashLogger::CrashLogger,
            time: Some(time.to_owned()),
            exception: Some("EXCEPTION_ACCESS_VIOLATION".to_owned()),
            address: Some(address.to_owned()),
            stack_modules: stack.iter().map(|module| (*module).to_owned()).collect(),
            objects,
        }
    }

    fn of_plugin(name: &str, plugin: &str) -> CrashObject {
        CrashObject {
            kind: Some("Character".to_owned()),
            name: Some(name.to_owned()),
            plugin: Some(plugin.to_owned()),
            ..CrashObject::default()
        }
    }

    fn facts(logs: Vec<CrashLog>) -> Facts {
        Facts {
            crashes: CrashFacts {
                logs,
                window_days: 30,
            },
            ..Facts::default()
        }
    }

    fn mods() -> Mods {
        Mods(vec![
            ("data/skse/plugins/physics.dll", "Physics"),
            ("data/skse/plugins/helper.dll", "Helper"),
            ("data/farms.esp", "Farms"),
            ("data/meshes/farm/barn.nif", "Barns"),
        ])
    }

    fn of_kind<'a>(findings: &'a [Finding], kind: &str) -> Vec<&'a Finding> {
        findings
            .iter()
            .filter(|finding| finding.kind == kind)
            .collect()
    }

    #[test]
    fn a_crash_inside_a_mods_dll_is_a_likely_lead() {
        let facts = facts(vec![
            log(
                "b.log",
                "2026-01-03 10:00",
                "Physics.dll+1234",
                &["Physics.dll"],
                Vec::new(),
            ),
            log(
                "a.log",
                "2026-01-01 09:00",
                "Physics.dll+1234",
                &["Physics.dll"],
                Vec::new(),
            ),
        ]);
        let findings = run_all(&Context::new(&facts, &mods()));

        let [finding] = of_kind(&findings, "crashes.in-dll")[..] else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "crashes.in-dll:file:data/skse/plugins/physics.dll"
        );
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(finding.args["last"], ArgValue::from("2026-01-03 10:00"));
        assert_eq!(
            finding
                .provider
                .as_ref()
                .map(|mod_ref| mod_ref.name.as_str()),
            Some("Physics")
        );
        // Its own DLL is not counted again as a mod involved.
        assert_eq!(
            of_kind(&findings, "crashes.mod-involved"),
            Vec::<&Finding>::new()
        );
    }

    #[test]
    fn objects_meshes_and_dlls_on_the_stack_are_hints_at_their_mods() {
        let barn = CrashObject {
            kind: Some("BSFadeNode".to_owned()),
            mesh: Some(r"meshes\farm\barn.nif".to_owned()),
            ..CrashObject::default()
        };
        let facts = facts(vec![log(
            "a.log",
            "2026-01-01 09:00",
            "SkyrimSE.exe+0123",
            &["SkyrimSE.exe", "Helper.dll", "KERNEL32.DLL"],
            vec![
                of_plugin("Farmer Tom", "Farms.esp"),
                of_plugin("Guard", "Skyrim.esm"),
                barn,
            ],
        )]);
        let findings = run_all(&Context::new(&facts, &mods()));

        let involved = of_kind(&findings, "crashes.mod-involved");
        let names: Vec<&str> = involved
            .iter()
            .filter_map(|finding| {
                finding
                    .provider
                    .as_ref()
                    .map(|mod_ref| mod_ref.name.as_str())
            })
            .collect();
        assert_eq!(names, ["Barns", "Farms", "Helper"]);
        let farms = involved[1];
        assert_eq!(farms.severity, Severity::Warning);
        assert_eq!(farms.confidence, Confidence::Hint);
        assert_eq!(
            farms.args["examples"],
            ArgValue::from("Farmer Tom (Farms.esp)")
        );
        assert_eq!(of_kind(&findings, "crashes.in-dll"), Vec::<&Finding>::new());
    }

    #[test]
    fn recent_crashes_are_summed_up_once() {
        let facts = facts(vec![
            log(
                "b.log",
                "2026-01-03 10:00",
                "SkyrimSE.exe+0123",
                &[],
                Vec::new(),
            ),
            log(
                "a.log",
                "2026-01-01 09:00",
                "ntdll.dll+0456",
                &[],
                Vec::new(),
            ),
        ]);
        let findings = run_all(&Context::new(&facts, &mods()));

        let [summary] = of_kind(&findings, "crashes.recent")[..] else {
            panic!("expected one summary, got {findings:?}");
        };
        assert_eq!(summary.severity, Severity::Info);
        assert_eq!(summary.args["count"], ArgValue::Number(2));
        assert_eq!(summary.args["last"], ArgValue::from("2026-01-03 10:00"));
        // Neither the game nor Windows is blamed.
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn no_reports_no_findings() {
        let findings = run_all(&Context::new(&facts(Vec::new()), &mods()));
        assert_eq!(of_kind(&findings, "crashes.recent"), Vec::<&Finding>::new());
    }
}
