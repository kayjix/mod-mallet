//! Tests of the command line from the outside: arguments in, text out.

// Helper functions of an integration test are not inside a `#[test]`
// function, so the test exemptions in clippy.toml do not reach them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::path::Path;

const SKSE_LOG: &str = "\
SKSE64 runtime: initialize (version = 2.3.1 01070680 01DD748122C36157, os = 6.2 (9200))
plugin Fixes.dll (00000001 Fixes 00000007) loaded correctly (handle 1)
plugin helper.dll (00000000  00000000) no version data 0 (handle 0)
plugin Old.dll (00000001 old 00000001) disabled, incompatible with current version of the game 0 (handle 0)
";

/// Creates a made-up game directory below `root` and returns the arguments
/// that point a scan at it.
fn setup(root: &Path) -> Vec<String> {
    let game_dir = root.join("Game");
    let data_dir = game_dir.join("Data");
    fs::create_dir_all(&data_dir).unwrap();

    let log = root.join("skse64.log");
    fs::write(&log, SKSE_LOG).unwrap();
    // The plugins the log names are still in the game; a plugin that is
    // gone since is not reported.
    let plugins = data_dir.join("SKSE").join("Plugins");
    fs::create_dir_all(&plugins).unwrap();
    for plugin in ["Fixes.dll", "helper.dll", "Old.dll"] {
        fs::write(plugins.join(plugin), b"not a real library").unwrap();
    }

    // An empty list of active plugins, so that the test never looks at the
    // real one of the machine it runs on.
    let local_dir = root.join("Local");
    fs::create_dir_all(&local_dir).unwrap();
    fs::write(local_dir.join("plugins.txt"), "").unwrap();

    let target = data_dir.to_string_lossy().replace('\\', "\\\\");
    fs::write(
        data_dir.join("vortex.deployment.json"),
        format!(
            r#"{{ "targetPath": "{target}", "files": [
                {{ "relPath": "SKSE\\Plugins\\Old.dll", "source": "Old Mod 123 1.0" }}
            ] }}"#
        ),
    )
    .unwrap();

    vec![
        "--game-dir".to_owned(),
        game_dir.to_string_lossy().into_owned(),
        "--local-dir".to_owned(),
        local_dir.to_string_lossy().into_owned(),
        "--manager-state".to_owned(),
        root.join("no-vortex-state.json")
            .to_string_lossy()
            .into_owned(),
        "--extender-log".to_owned(),
        log.to_string_lossy().into_owned(),
        "--cache".to_owned(),
        root.join("cache.sqlite").to_string_lossy().into_owned(),
    ]
}

fn run(args: &[&str]) -> anyhow::Result<String> {
    let mut out = Vec::new();
    let all = std::iter::once("mallet").chain(args.iter().copied());
    mallet_cli::run(all, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

fn run_scan(leading: &[&str], setup_args: &[String], trailing: &[&str]) -> String {
    let mut args: Vec<&str> = leading.to_vec();
    args.push("scan");
    args.extend(setup_args.iter().map(String::as_str));
    args.extend_from_slice(trailing);
    run(&args).unwrap()
}

#[test]
fn scan_prints_a_report_in_english() {
    let root = tempfile::tempdir().unwrap();
    let output = run_scan(&[], &setup(root.path()), &[]);
    insta::assert_snapshot!(output);
}

#[test]
fn scan_prints_a_report_in_german() {
    let root = tempfile::tempdir().unwrap();
    let output = run_scan(&["--lang", "de"], &setup(root.path()), &[]);
    insta::assert_snapshot!(output);
}

#[test]
fn notes_are_listed_on_request() {
    let root = tempfile::tempdir().unwrap();
    let setup_args = setup(root.path());

    let without = run_scan(&[], &setup_args, &[]);
    assert!(!without.contains("helper.dll"));
    assert!(without.contains("Add --notes"));

    let with = run_scan(&[], &setup_args, &["--notes"]);
    assert!(with.contains("helper.dll"));
    assert!(!with.contains("Add --notes"));
}

#[test]
fn report_repeats_the_last_scan() {
    let root = tempfile::tempdir().unwrap();
    let setup_args = setup(root.path());
    let scanned = run_scan(&[], &setup_args, &[]);

    let cache = setup_args.last().unwrap();
    let reported = run(&["report", "--cache", cache]).unwrap();
    assert_eq!(reported, scanned);
}

#[test]
fn report_without_a_scan_says_so() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("none.sqlite");
    let output = run(&["report", "--cache", &cache.to_string_lossy()]).unwrap();
    assert_eq!(
        output,
        "No scan has been stored yet. Run \"mallet scan\" first.\n"
    );
}

#[test]
fn json_output_is_the_scan_report() {
    let root = tempfile::tempdir().unwrap();
    let output = run_scan(&[], &setup(root.path()), &["--format", "json"]);

    let report: mallet_core::ScanReport = serde_json::from_str(&output).unwrap();
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].kind, "extender.plugin-incompatible");
}

#[test]
fn explain_describes_a_kind() {
    let output = run(&["explain", "extender.plugin-incompatible"]).unwrap();
    insta::assert_snapshot!(output);
}

#[test]
fn explain_rejects_an_unknown_kind() {
    let error = run(&["explain", "no.such-kind"]).unwrap_err();
    assert!(error.to_string().contains("no.such-kind"));
}

#[test]
fn kinds_lists_every_kind() {
    let output = run(&["kinds"]).unwrap();
    for kind in mallet_core::kinds::ALL {
        assert!(
            output.contains(kind.id),
            "{} missing from:\n{output}",
            kind.id
        );
    }
}

/// The arguments that point the `vortex` commands at a made-up Vortex.
fn vortex_args(root: &Path) -> Vec<String> {
    fs::create_dir_all(root.join("Vortex").join("plugins")).unwrap();
    vec![
        "--vortex-data-dir".to_owned(),
        root.join("Vortex").to_string_lossy().into_owned(),
        "--link-dir".to_owned(),
        root.join("bridge").to_string_lossy().into_owned(),
    ]
}

fn run_vortex(command: &[&str], vortex_args: &[String]) -> anyhow::Result<String> {
    let mut args = vec!["vortex"];
    args.extend_from_slice(command);
    args.extend(vortex_args.iter().map(String::as_str));
    run(&args)
}

#[test]
fn the_vortex_extension_is_installed_on_request_only() {
    let root = tempfile::tempdir().unwrap();
    let args = vortex_args(root.path());
    let extension = root.path().join("Vortex/plugins/mod-mallet-link");

    let status = run_vortex(&["status"], &args).unwrap();
    assert!(status.starts_with("The Vortex extension is not installed."));
    assert!(!extension.exists());

    let installed = run_vortex(&["install-extension"], &args).unwrap();
    assert!(installed.contains("Restart Vortex"));
    assert!(extension.join("index.js").is_file());

    let status = run_vortex(&["status"], &args).unwrap();
    assert!(status.contains("Vortex is not answering"));
}

#[test]
fn a_reinstall_that_cannot_be_handed_over_is_an_error_that_says_why() {
    let root = tempfile::tempdir().unwrap();
    let args = vortex_args(root.path());

    let error = run_vortex(&["reinstall", "Patch Hub 1.0"], &args).unwrap_err();
    assert!(error.to_string().contains("not installed yet"));

    run_vortex(&["install-extension"], &args).unwrap();
    let error = run_vortex(&["reinstall", "Patch Hub 1.0"], &args).unwrap_err();
    assert!(error.to_string().contains("Vortex is not answering"));
}

#[test]
fn an_unsupported_language_is_rejected() {
    let error = run(&["--lang", "fr", "kinds"]).unwrap_err();
    assert!(error.to_string().contains("not a supported language"));
}
