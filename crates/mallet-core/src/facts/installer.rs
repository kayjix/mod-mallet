//! What a mod's installer offers, and what was chosen.
//!
//! Many mods come with a scripted installer: a list of steps, each with
//! groups of options. The installer marks options as recommended depending
//! on what is already installed, typically "tick this patch if that other
//! mod is active". The mod manager shows the dialog once, at installation.
//! A patch for a mod that is installed afterwards is never offered again.
//!
//! These types describe an installer in a form that is independent of the
//! file format it was read from, so that an analysis can ask what the
//! installer would recommend today.

use crate::ModRef;

/// The installer of one installed mod and the options chosen in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerFacts {
    /// The mod the installer belongs to.
    pub mod_ref: ModRef,
    /// The steps of the installer, in order.
    pub steps: Vec<InstallerStep>,
    /// The options that were chosen when the mod was installed.
    pub chosen: Vec<ChosenOption>,
}

/// One page of an installer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerStep {
    /// The step's title.
    pub name: String,
    /// When the step is shown. `None` means always.
    pub visible: Option<Condition>,
    /// The groups of options on the page.
    pub groups: Vec<InstallerGroup>,
}

/// A group of options, with a rule for how many may be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerGroup {
    /// The group's title.
    pub name: String,
    /// How many options of the group may be chosen.
    pub selection: Selection,
    /// The options.
    pub options: Vec<InstallerOption>,
}

/// How many options of a group may be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Any number, including none.
    Any,
    /// One or more.
    AtLeastOne,
    /// None or one.
    AtMostOne,
    /// Exactly one.
    ExactlyOne,
    /// All of them; nothing to choose.
    All,
}

/// One thing the installer can install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerOption {
    /// The option's title.
    pub name: String,
    /// What the installer says about the option, in the mod author's words.
    pub description: String,
    /// Flags the installer sets when the option is chosen. Later steps and
    /// options can depend on them.
    pub sets_flags: Vec<(String, String)>,
    /// How the option is presented when none of the [`rules`] applies.
    ///
    /// [`rules`]: InstallerOption::rules
    pub default_status: OptionStatus,
    /// How the option is presented depending on the setup. The first rule
    /// whose condition holds decides.
    pub rules: Vec<(Condition, OptionStatus)>,
}

/// How an installer presents an option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionStatus {
    /// Always installed.
    Required,
    /// Ticked in advance.
    Recommended,
    /// Offered without a preference.
    Optional,
    /// Offered with a warning.
    CouldBeUsable,
    /// Greyed out.
    NotUsable,
}

/// A condition an installer puts on a step or an option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Every part holds.
    All(Vec<Condition>),
    /// At least one part holds.
    Any(Vec<Condition>),
    /// A file of the game's data directory is in a given state.
    File {
        /// The file, relative to the data directory.
        file: String,
        /// The state it has to be in.
        state: FileState,
    },
    /// An earlier choice set a flag to a value.
    Flag {
        /// The flag's name.
        name: String,
        /// The value it has to have.
        value: String,
    },
    /// A condition Mod Mallet does not evaluate, such as a minimum version
    /// of the game. It is taken to hold.
    Unknown,
}

/// The state of a file that an installer asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    /// The file is installed and, if it is a plugin, active.
    Active,
    /// The file is a plugin that is installed but not active.
    Inactive,
    /// The file is not installed.
    Missing,
}

/// An option that was chosen, named by its place in the installer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChosenOption {
    /// The title of the step.
    pub step: String,
    /// The title of the group.
    pub group: String,
    /// The title of the option.
    pub option: String,
}
