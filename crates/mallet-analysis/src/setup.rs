//! Problems with reading the setup itself.

use mallet_core::{Finding, Subject, kinds};

use crate::{Analysis, Context};

/// Reports what could not be read, so that a scan never looks more complete
/// than it was.
pub struct SetupProblems;

impl Analysis for SetupProblems {
    fn id(&self) -> &'static str {
        "setup-problems"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mut findings: Vec<Finding> = context
            .facts
            .unreadable
            .iter()
            .map(|file| {
                Finding::new(
                    &kinds::SETUP_FILE_UNREADABLE,
                    Subject::File {
                        path: file.path.clone(),
                    },
                )
                .with_arg("file", file.path.file_name())
                .with_arg("reason", file.reason.as_str())
                .with_provider(context.mods.provider_of(&file.path))
            })
            .collect();

        if let Some(reason) = &context.facts.load_order_problem {
            findings.push(
                Finding::new(&kinds::SETUP_LOAD_ORDER_UNKNOWN, Subject::Setup)
                    .with_arg("reason", reason.as_str()),
            );
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::UnreadableFile;
    use mallet_core::{ArgValue, Facts, GamePath, NoModSource, Severity};

    use super::*;
    use crate::run_all;

    #[test]
    fn an_unreadable_file_is_reported_with_the_readers_reason() {
        let facts = Facts {
            unreadable: vec![UnreadableFile {
                path: GamePath::new("data/broken.bsa"),
                reason: "the file is not a BSA archive".to_owned(),
            }],
            ..Facts::default()
        };
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "setup.file-unreadable");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.args["file"], ArgValue::from("broken.bsa"));
        assert_eq!(
            finding.args["reason"],
            ArgValue::from("the file is not a BSA archive")
        );
    }

    #[test]
    fn an_unknown_load_order_is_reported_once() {
        let facts = Facts {
            load_order_problem: Some("plugins.txt is missing".to_owned()),
            ..Facts::default()
        };
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "setup.load-order-unknown");
        assert_eq!(finding.id.as_str(), "setup.load-order-unknown:setup");
    }
}
