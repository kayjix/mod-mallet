//! A report meant for sharing, as text for a forum post or a bug report
//! (ADR 0091).

use std::fmt::Write as _;

use mallet_core::{ArgValue, Placeholder, ScanReport, Severity, SharedReport, Subject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Messages;

/// A report meant for sharing, put into words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedShare {
    /// The report as Markdown, which also reads well as plain text.
    pub text: String,
    /// One sentence saying what was removed from it.
    pub removed: String,
}

fn count(len: usize) -> ArgValue {
    ArgValue::Number(i64::try_from(len).unwrap_or(i64::MAX))
}

impl Messages {
    /// Puts a report meant for sharing into words: the setup, the places
    /// read, the findings and the load order, with the placeholders
    /// explained.
    pub fn render_share(&self, shared: &SharedReport) -> RenderedShare {
        let removed = self.text("share-removed", &[]);
        let report = &shared.report;
        let mut text = String::new();
        // Writing into a `String` cannot fail.
        let _ = self.write_share(report, &shared.placeholders, &removed, &mut text);
        RenderedShare { text, removed }
    }

    fn write_share(
        &self,
        report: &ScanReport,
        placeholders: &[Placeholder],
        removed: &str,
        out: &mut String,
    ) -> std::fmt::Result {
        writeln!(out, "# {}", self.text("share-title", &[]))?;
        writeln!(out)?;
        self.write_share_header(report, out)?;
        writeln!(out)?;
        writeln!(out, "> {removed}")?;

        if !placeholders.is_empty() {
            writeln!(out)?;
            writeln!(out, "## {}", self.text("share-placeholders", &[]))?;
            writeln!(out)?;
            for placeholder in placeholders {
                writeln!(
                    out,
                    "- `{}`: {}",
                    placeholder.token(),
                    self.text(placeholder_id(*placeholder), &[])
                )?;
            }
        }

        if !report.locations.is_empty() {
            writeln!(out)?;
            writeln!(out, "## {}", self.text("share-places", &[]))?;
            writeln!(out)?;
            for location in &report.locations {
                let rendered = self.render_location(location);
                let state = if location.found {
                    rendered.origin_label
                } else {
                    self.text("location-not-found", &[])
                };
                writeln!(
                    out,
                    "- {}: `{}` ({state})",
                    rendered.role_label, location.path
                )?;
            }
        }

        self.write_share_findings(report, out)?;
        self.write_share_load_order(report, out)
    }

    /// The errors and warnings in full, the notes one line each, and the
    /// totals.
    fn write_share_findings(&self, report: &ScanReport, out: &mut String) -> std::fmt::Result {
        writeln!(out)?;
        writeln!(out, "## {}", self.text("share-problems", &[]))?;
        writeln!(out)?;
        let problems: Vec<_> = report
            .findings
            .iter()
            .filter(|finding| finding.severity != Severity::Info)
            .collect();
        if problems.is_empty() {
            writeln!(out, "{}", self.text("report-no-findings", &[]))?;
        }
        let file_label = self.text("label-file", &[]);
        let mod_label = self.text("label-mod", &[]);
        let remedy_label = self.text("label-remedy", &[]);
        let kind_label = self.text("label-kind", &[]);
        for finding in problems {
            let rendered = self.render_for(finding, report.manager.as_deref());
            writeln!(
                out,
                "### {} ({}): {}",
                rendered.severity_label, rendered.confidence_label, rendered.title
            )?;
            writeln!(out)?;
            writeln!(out, "{}", rendered.summary)?;
            writeln!(out)?;
            if let Subject::File { path } = &finding.subject {
                writeln!(out, "- {file_label}: `{path}`")?;
            }
            if let Some(provider) = &finding.provider {
                writeln!(out, "- {mod_label}: {}", provider.name)?;
            }
            writeln!(out, "- {remedy_label}: {}", rendered.remedy)?;
            writeln!(out, "- {kind_label}: `{}`", finding.kind)?;
            writeln!(out)?;
        }

        let notes: Vec<_> = report
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Info)
            .collect();
        if !notes.is_empty() {
            writeln!(out, "## {}", self.text("share-notes", &[]))?;
            writeln!(out)?;
            for finding in notes {
                let rendered = self.render_for(finding, report.manager.as_deref());
                writeln!(
                    out,
                    "- {}: {} (`{}`)",
                    rendered.title, rendered.summary, finding.kind
                )?;
            }
            writeln!(out)?;
        }

        let severities = |severity: Severity| {
            count(
                report
                    .findings
                    .iter()
                    .filter(|finding| finding.severity == severity)
                    .count(),
            )
        };
        writeln!(
            out,
            "{}",
            self.text(
                "report-totals",
                &[
                    ("errors", severities(Severity::Error)),
                    ("warnings", severities(Severity::Warning)),
                    ("notes", severities(Severity::Info)),
                ],
            )
        )
    }

    /// The active plugins in load order, each with its mod.
    fn write_share_load_order(&self, report: &ScanReport, out: &mut String) -> std::fmt::Result {
        if !report.load_order.is_empty() {
            writeln!(out)?;
            writeln!(
                out,
                "## {}",
                self.text(
                    "share-load-order",
                    &[("count", count(report.load_order.len()))]
                )
            )?;
            writeln!(out)?;
            for (index, plugin) in report.load_order.iter().enumerate() {
                match &plugin.provider {
                    Some(provider) => {
                        writeln!(out, "{}. {} ({})", index + 1, plugin.name, provider.name)?;
                    }
                    None => writeln!(out, "{}. {}", index + 1, plugin.name)?,
                }
            }
        }
        Ok(())
    }

    /// The game, the script extender and how much was read, one per line.
    fn write_share_header(&self, report: &ScanReport, out: &mut String) -> std::fmt::Result {
        let name = ArgValue::from(report.game.name.as_str());
        let game = match &report.game.version {
            Some(version) => self.text(
                "report-game",
                &[
                    ("name", name),
                    ("version", ArgValue::from(version.as_str())),
                ],
            ),
            None => self.text("report-game-unknown-version", &[("name", name)]),
        };
        if let Some(version) = &report.game.extender_version {
            let extender = self.text(
                "report-extender",
                &[("version", ArgValue::from(version.as_str()))],
            );
            writeln!(out, "{game}, {extender}  ")?;
            let checked = ArgValue::Number(i64::from(report.extender_plugins_checked));
            writeln!(
                out,
                "{}  ",
                self.text("report-plugins-checked", &[("count", checked)])
            )?;
        } else {
            writeln!(out, "{game}  ")?;
            writeln!(out, "{}  ", self.text("report-no-extender-log", &[]))?;
        }
        if let Some(manager) = &report.manager {
            writeln!(
                out,
                "{}  ",
                self.text(
                    "share-manager",
                    &[("manager", ArgValue::from(manager.as_str()))]
                )
            )?;
        }
        let read = [
            (
                "plugins",
                ArgValue::Number(i64::from(report.plugins_checked)),
            ),
            (
                "archives",
                ArgValue::Number(i64::from(report.archives_checked)),
            ),
        ];
        writeln!(out, "{}", self.text("report-files-read", &read))?;
        Ok(())
    }
}

fn placeholder_id(placeholder: Placeholder) -> &'static str {
    match placeholder {
        Placeholder::Game => "share-placeholder-game",
        Placeholder::Staging => "share-placeholder-staging",
        Placeholder::Instance => "share-placeholder-instance",
        Placeholder::Downloads => "share-placeholder-downloads",
        Placeholder::Documents => "share-placeholder-documents",
        Placeholder::LocalAppData => "share-placeholder-local-app-data",
        Placeholder::AppData => "share-placeholder-app-data",
        Placeholder::UserProfile => "share-placeholder-user-profile",
        Placeholder::User => "share-placeholder-user",
        Placeholder::Machine => "share-placeholder-machine",
        Placeholder::Profile => "share-placeholder-profile",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Locale;

    #[test]
    fn every_placeholder_has_a_text_in_every_language() {
        for locale in Locale::ALL {
            let messages = Messages::new(locale);
            for placeholder in Placeholder::ALL {
                let id = placeholder_id(placeholder);
                assert_ne!(messages.text(id, &[]), id, "{}: {id}", locale.code());
            }
        }
    }
}
