//! The report as text for a terminal.

use std::io::{self, Write};

use mallet_core::{Action, ArgValue, ScanReport, Severity, Subject};
use mallet_messages::Messages;

/// Writes `report` to `out` as sentences in the language of `messages`.
///
/// Notes are things worth knowing that need no action, and a large setup
/// has many of them. Unless `show_notes` is set they are only counted, so
/// that the errors and warnings stay readable.
pub fn write_report(
    report: &ScanReport,
    messages: &Messages,
    show_notes: bool,
    out: &mut dyn Write,
) -> io::Result<()> {
    write_header(report, messages, out)?;
    if let Some(changes) = &report.version_changes {
        writeln!(out, "{}", version_changes(changes, messages))?;
    }
    writeln!(out)?;

    if report.findings.is_empty() {
        writeln!(out, "{}", messages.text("report-no-findings", &[]))?;
        return Ok(());
    }

    let file_label = messages.text("label-file", &[]);
    let mod_label = messages.text("label-mod", &[]);
    let remedy_label = messages.text("label-remedy", &[]);
    let action_label = messages.text("label-action", &[]);

    let shown = report
        .findings
        .iter()
        .filter(|finding| show_notes || finding.severity != Severity::Info);
    for finding in shown {
        let rendered = messages.render(finding);
        writeln!(
            out,
            "{} ({}): {}",
            rendered.severity_label, rendered.confidence_label, rendered.title
        )?;
        writeln!(out, "  {}", rendered.summary)?;
        if let Subject::File { path } = &finding.subject {
            writeln!(out, "  {file_label}: {path}")?;
        }
        if let Some(provider) = &finding.provider {
            writeln!(out, "  {mod_label}: {}", provider.name)?;
        }
        writeln!(out, "  {remedy_label}: {}", rendered.remedy)?;
        for (action, shown) in finding.actions.iter().zip(&rendered.actions) {
            let Action::ReinstallMod { mod_ref, .. } = action else {
                continue;
            };
            writeln!(
                out,
                "  {action_label}: mallet vortex reinstall \"{}\"",
                mod_ref.id
            )?;
            for step in &shown.guide {
                let tick = &step.tick;
                writeln!(
                    out,
                    "    [ ] {}: {} › {} › {}",
                    step.page_label, tick.step, tick.group, tick.option
                )?;
            }
        }
        writeln!(out, "  [{}]", finding.kind)?;
        writeln!(out)?;
    }

    let totals = totals(report);
    writeln!(out, "{}", messages.text("report-totals", &totals))?;
    if !show_notes
        && let Some((_, ArgValue::Number(notes))) = totals.last()
        && *notes > 0
    {
        writeln!(out, "{}", messages.text("report-notes-hidden", &[]))?;
    }
    writeln!(out, "{}", messages.text("report-explain-hint", &[]))?;
    Ok(())
}

/// What changed in the versions since the last scan, in one sentence
/// (ADR 0079).
fn version_changes(changes: &mallet_core::VersionChanges, messages: &Messages) -> String {
    let count = |len: usize| ArgValue::Number(i64::try_from(len).unwrap_or(i64::MAX));
    let step = |id: &str, step: &mallet_core::VersionStep| {
        messages.text(
            id,
            &[
                ("from", ArgValue::from(step.from.as_str())),
                ("to", ArgValue::from(step.to.as_str())),
            ],
        )
    };
    let mut parts = Vec::new();
    if let Some(game) = &changes.game {
        parts.push(step("report-version-game", game));
    }
    if let Some(extender) = &changes.extender {
        parts.push(step("report-version-extender", extender));
    }
    if !changes.plugins.is_empty() {
        parts.push(messages.text(
            "report-version-plugins",
            &[("count", count(changes.plugins.len()))],
        ));
    }
    if !changes.mods.is_empty() {
        parts.push(messages.text(
            "report-version-mods",
            &[("count", count(changes.mods.len()))],
        ));
    }
    if changes.foundation_unfit {
        parts.push(messages.text("report-version-foundation", &[]));
    }
    if !changes.unfit.is_empty() {
        parts.push(messages.text(
            "report-version-unfit",
            &[
                ("count", count(changes.unfit.len())),
                ("files", ArgValue::from(changes.unfit.join(", "))),
            ],
        ));
    }
    messages.text(
        "report-version-since",
        &[
            (
                "since",
                ArgValue::from(if changes.since == 0 { "start" } else { "scan" }),
            ),
            ("changes", ArgValue::from(parts.join("; "))),
        ],
    )
}

/// Writes the places the scan looked at, one per line.
pub(crate) fn write_locations(
    report: &ScanReport,
    messages: &Messages,
    out: &mut dyn Write,
) -> io::Result<()> {
    for location in &report.locations {
        let rendered = messages.render_location(location);
        let state = if location.found {
            rendered.origin_label
        } else {
            messages.text("location-not-found", &[])
        };
        writeln!(out, "{}: {} ({state})", rendered.role_label, location.path)?;
    }
    writeln!(out)
}

fn write_header(report: &ScanReport, messages: &Messages, out: &mut dyn Write) -> io::Result<()> {
    let name = ArgValue::from(report.game.name.as_str());
    let game = match &report.game.version {
        Some(version) => messages.text(
            "report-game",
            &[
                ("name", name),
                ("version", ArgValue::from(version.as_str())),
            ],
        ),
        None => messages.text("report-game-unknown-version", &[("name", name)]),
    };

    let Some(extender_version) = &report.game.extender_version else {
        writeln!(out, "{game}")?;
        writeln!(out, "{}", messages.text("report-no-extender-log", &[]))?;
        return write_files_read(report, messages, out);
    };
    let extender = messages.text(
        "report-extender",
        &[("version", ArgValue::from(extender_version.as_str()))],
    );
    let checked = ArgValue::Number(i64::from(report.extender_plugins_checked));
    writeln!(out, "{game}, {extender}")?;
    writeln!(
        out,
        "{}",
        messages.text("report-plugins-checked", &[("count", checked)])
    )?;
    write_files_read(report, messages, out)
}

fn write_files_read(
    report: &ScanReport,
    messages: &Messages,
    out: &mut dyn Write,
) -> io::Result<()> {
    let counts = [
        (
            "plugins",
            ArgValue::Number(i64::from(report.plugins_checked)),
        ),
        (
            "archives",
            ArgValue::Number(i64::from(report.archives_checked)),
        ),
    ];
    writeln!(out, "{}", messages.text("report-files-read", &counts))?;
    if report.installers_checked > 0 {
        let count = ArgValue::Number(i64::from(report.installers_checked));
        writeln!(
            out,
            "{}",
            messages.text("report-installers-read", &[("count", count)])
        )?;
    }
    if report.meshes_checked > 0 {
        let count = ArgValue::Number(i64::from(report.meshes_checked));
        writeln!(
            out,
            "{}",
            messages.text("report-meshes-compared", &[("count", count)])
        )?;
    }
    Ok(())
}

fn totals(report: &ScanReport) -> [(&'static str, ArgValue); 3] {
    let count = |severity: Severity| {
        let found = report
            .findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count();
        ArgValue::Number(i64::try_from(found).unwrap_or(i64::MAX))
    };
    [
        ("errors", count(Severity::Error)),
        ("warnings", count(Severity::Warning)),
        ("notes", count(Severity::Info)),
    ]
}
