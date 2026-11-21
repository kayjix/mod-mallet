//! The `mallet` command: Mod Mallet for the terminal.
//!
//! The command line is a thin shell. It parses arguments, calls
//! `mallet-engine` and prints what `mallet-messages` makes of the result.
//! Anything that decides something about a setup belongs in those crates,
//! where the desktop app can use it too.

mod text;

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use mallet_core::{Action, ActionStatus, ArgValue, ModRef, ScanReport, kinds};
use mallet_engine::{ManagerOptions, ScanOptions};
use mallet_messages::{Locale, Messages};

pub use text::write_report;

/// Diagnoses a modded game setup and explains what is wrong.
#[derive(Debug, Parser)]
#[command(name = "mallet", version, propagate_version = true)]
struct Cli {
    /// Language of the output: `en` or `de`.
    #[arg(long, global = true, value_name = "LANG", default_value = "en")]
    lang: String,

    /// Show which places were read and how they were found, list the notes,
    /// and print what Mod Mallet is doing to the error stream.
    #[arg(long, short, global = true)]
    verbose: bool,

    /// The game to look at: `skyrimse`, `skyrimvr`, `skyrim`, `fallout4`,
    /// `fallout4vr`, `falloutnv`, `fallout3`, `oblivion` or
    /// `oblivionremastered`.
    #[arg(long, global = true, value_name = "GAME", default_value = "skyrimse")]
    game: String,

    #[command(subcommand)]
    command: Command,
}

// The command is parsed once per run; the size of the value does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Command {
    /// Examine the setup and print what is wrong with it.
    ///
    /// A scan only reads. It changes nothing in the game, the mods or the
    /// mod manager.
    Scan(ScanArgs),
    /// Print the result of the last scan again without scanning.
    Report(ReportArgs),
    /// Explain a kind of finding: what it means and what to do about it.
    Explain {
        /// The kind, for example `extender.plugin-incompatible`.
        kind: String,
    },
    /// List every kind of finding Mod Mallet can report.
    Kinds,
    /// Have Vortex act on a finding, through a small Vortex extension.
    ///
    /// Mod Mallet does not change mods itself. It asks Vortex, which does
    /// what its own menu entry would do.
    Vortex(VortexArgs),
}

#[derive(Debug, Args)]
struct VortexArgs {
    /// The directory in which Vortex keeps its data. Detected if omitted.
    #[arg(long, global = true, value_name = "DIR")]
    vortex_data_dir: Option<PathBuf>,

    /// The folder shared with the Vortex extension. Detected if omitted.
    #[arg(long, global = true, value_name = "DIR")]
    link_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: VortexCommand,
}

#[derive(Debug, Subcommand)]
enum VortexCommand {
    /// Say whether the extension is installed and Vortex is answering.
    Status,
    /// Put the extension into Vortex's plugin folder. Vortex loads it the
    /// next time it starts.
    InstallExtension,
    /// Ask Vortex to run a mod's installer again.
    Reinstall {
        /// The mod, as a finding names it: its folder in Vortex's staging
        /// directory.
        mod_id: String,
    },
}

#[derive(Debug, Args)]
struct ScanArgs {
    /// The directory the game is installed in. Detected if omitted.
    #[arg(long, value_name = "DIR")]
    game_dir: Option<PathBuf>,

    /// The directory that holds `plugins.txt`, the game's list of active
    /// plugins. Detected if omitted.
    #[arg(long, value_name = "DIR")]
    local_dir: Option<PathBuf>,

    /// The script extender's log file. Detected if omitted.
    #[arg(long, value_name = "FILE")]
    extender_log: Option<PathBuf>,

    /// The Vortex deployment manifest. Detected if omitted.
    #[arg(long, value_name = "FILE")]
    deployment_manifest: Option<PathBuf>,

    /// The backup of Vortex's state. Detected if omitted.
    #[arg(long, value_name = "FILE")]
    manager_state: Option<PathBuf>,

    /// The directory Vortex keeps downloaded mod archives in. Detected if
    /// omitted.
    #[arg(long, value_name = "DIR")]
    downloads_dir: Option<PathBuf>,

    /// The folder Vortex keeps every mod's files in. Taken from the
    /// deployment manifest if omitted. Only the names of its files are read.
    #[arg(long, value_name = "DIR")]
    staging_dir: Option<PathBuf>,

    /// The 7-Zip program used to look into mod archives. Detected if
    /// omitted.
    #[arg(long, value_name = "FILE")]
    unpacker: Option<PathBuf>,

    /// The LOOT masterlist, whose rules about plugins are checked. Detected
    /// if omitted and `--manager-state` is omitted, too.
    #[arg(long, value_name = "FILE")]
    masterlist: Option<PathBuf>,

    /// Do not use the cache: read everything again, and do not remember the
    /// result for `mallet report`.
    #[arg(long)]
    no_cache: bool,

    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ReportArgs {
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// How to print the result.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,

    /// Also list the notes: things worth knowing that need no action. The
    /// JSON format always contains them.
    #[arg(long)]
    notes: bool,

    /// Where the result of the last scan is kept.
    #[arg(long, value_name = "FILE")]
    cache: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Sentences for people.
    Text,
    /// The scan report as JSON, described by `schemas/scan-report.schema.json`.
    Json,
}

/// The definition of the command line, for generating its reference
/// documentation.
pub fn command() -> clap::Command {
    Cli::command()
}

/// Runs the command line with `args` (the first being the program name) and
/// writes the result to `out`.
pub fn run<I, T>(args: I, out: &mut dyn Write) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        // `--help` and `--version` arrive here too. clap knows whether the
        // text belongs on stdout or stderr and which exit code fits.
        Err(error) => error.exit(),
    };

    if cli.verbose {
        // Fails only if logging is already set up, which is just as good.
        let _ = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_max_level(tracing::Level::INFO)
            .try_init();
    }

    let Some(locale) = Locale::from_tag(&cli.lang) else {
        let known: Vec<_> = Locale::ALL.iter().map(|locale| locale.code()).collect();
        bail!(
            "`{}` is not a supported language (supported: {})",
            cli.lang,
            known.join(", ")
        );
    };
    let messages = Messages::new(locale);

    match cli.command {
        Command::Scan(args) => {
            let options = ScanOptions {
                game: Some(cli.game.clone()),
                game_dir: args.game_dir,
                local_dir: args.local_dir,
                extender_log: args.extender_log,
                deployment_manifest: args.deployment_manifest,
                manager_state: args.manager_state,
                manager_live_state: None,
                downloads_dir: args.downloads_dir,
                staging_dir: args.staging_dir,
                unpacker: args.unpacker,
                masterlist: args.masterlist,
            };
            let report = if args.no_cache {
                mallet_engine::scan(&options)?
            } else {
                mallet_engine::scan_and_store(&options, &cache_path(args.output.cache)?)?
            };
            print(
                &report,
                args.output.format,
                args.output.notes || cli.verbose,
                cli.verbose,
                &messages,
                out,
            )
        }
        Command::Report(args) => {
            let cache = cache_path(args.output.cache)?;
            let options = ScanOptions {
                game: Some(cli.game.clone()),
                ..ScanOptions::default()
            };
            if let Some(report) = mallet_engine::last_report(&options, &cache)? {
                print(
                    &report,
                    args.output.format,
                    args.output.notes || cli.verbose,
                    cli.verbose,
                    &messages,
                    out,
                )
            } else {
                writeln!(out, "{}", messages.text("report-no-scan", &[]))?;
                Ok(())
            }
        }
        Command::Explain { kind } => explain(&kind, &messages, out),
        Command::Kinds => {
            for kind in kinds::ALL {
                writeln!(out, "{}  {}", kind.id, messages.kind(kind).title)?;
            }
            Ok(())
        }
        Command::Vortex(args) => vortex(args, &cli.game, &messages, out),
    }
}

fn vortex(
    args: VortexArgs,
    game: &str,
    messages: &Messages,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let options = ManagerOptions {
        game: Some(game.to_owned()),
        vortex_data_dir: args.vortex_data_dir,
        link_dir: args.link_dir,
        timeout: None,
        setup: None,
    };
    match args.command {
        VortexCommand::Status => {
            let link = mallet_engine::manager_link(&options);
            let rendered = messages.render_link(link.installed, link.running, link.restart_needed);
            writeln!(out, "{}", rendered.message)?;
            if !link.installed {
                writeln!(out, "{}", rendered.explanation)?;
            }
        }
        VortexCommand::InstallExtension => {
            let path = mallet_engine::install_manager_link(&options)?;
            let path = ArgValue::from(path.display().to_string());
            writeln!(
                out,
                "{}",
                messages.text("link-installed", &[("path", path)])
            )?;
        }
        VortexCommand::Reinstall { mod_id } => {
            let action = Action::ReinstallMod {
                mod_ref: ModRef {
                    source: "vortex".to_owned(),
                    name: mod_id.clone(),
                    id: mod_id,
                },
                tick: Vec::new(),
            };
            let outcome = mallet_engine::run_action(&action, &options);
            let message = messages.render_outcome(&outcome).message;
            if outcome.status != ActionStatus::Started {
                bail!("{message}");
            }
            writeln!(out, "{message}")?;
        }
    }
    Ok(())
}

fn cache_path(chosen: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    chosen
        .or_else(mallet_engine::default_cache_path)
        .context("no place for the cache is known on this system; pass one with --cache")
}

fn print(
    report: &ScanReport,
    format: Format,
    show_notes: bool,
    show_locations: bool,
    messages: &Messages,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match format {
        Format::Text => {
            if show_locations {
                text::write_locations(report, messages, out)?;
            }
            write_report(report, messages, show_notes, out)?;
        }
        Format::Json => {
            serde_json::to_writer_pretty(&mut *out, report)?;
            writeln!(out)?;
        }
    }
    Ok(())
}

fn explain(kind_id: &str, messages: &Messages, out: &mut dyn Write) -> anyhow::Result<()> {
    let Some(kind) = kinds::find(kind_id) else {
        bail!(
            "{}",
            messages.text("explain-unknown-kind", &[("kind", ArgValue::from(kind_id))])
        );
    };
    let text = messages.kind(kind);
    writeln!(out, "{}", text.title)?;
    writeln!(out)?;
    writeln!(out, "{}: {}", messages.text("label-kind", &[]), kind.id)?;
    writeln!(
        out,
        "{}: {}",
        messages.text("label-severity", &[]),
        messages.severity(kind.severity)
    )?;
    writeln!(
        out,
        "{}: {}",
        messages.text("label-confidence", &[]),
        messages.confidence(kind.confidence)
    )?;
    writeln!(out)?;
    writeln!(out, "{}", text.explanation)?;
    writeln!(out)?;
    writeln!(
        out,
        "{}: {}",
        messages.text("label-remedy", &[]),
        text.remedy
    )?;
    Ok(())
}
