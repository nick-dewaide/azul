mod init;
mod plugin;
mod pull;
mod serve;
mod status;

use std::{borrow::Cow, env, path::Path, str::FromStr};

use clap::Parser;
use thiserror::Error;

pub use self::init::InitCommand;
pub use self::plugin::{PluginCommand, PluginSubcommand};
pub use self::pull::PullCommand;
pub use self::serve::ServeCommand;
pub use self::status::StatusCommand;

/// ScriptSync — bidirectional Roblox script synchronization.
#[derive(Debug, Parser)]
#[clap(name = "ScriptSync", version, about)]
pub struct Options {
    #[clap(flatten)]
    pub global: GlobalOptions,

    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

impl Options {
    pub fn run(self) -> anyhow::Result<()> {
        match self.subcommand {
            Subcommand::Init(subcommand) => subcommand.run(),
            Subcommand::Serve(subcommand) => subcommand.run(self.global),
            Subcommand::Pull(subcommand) => subcommand.run(),
            Subcommand::Status(subcommand) => subcommand.run(),
            Subcommand::Plugin(subcommand) => subcommand.run(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct GlobalOptions {
    /// Sets verbosity level. Can be specified multiple times.
    #[clap(long("verbose"), short, global(true), parse(from_occurrences))]
    pub verbosity: u8,

    /// Set color behavior. Valid values are auto, always, and never.
    #[clap(long("color"), global(true), default_value("auto"))]
    pub color: ColorChoice,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl FromStr for ColorChoice {
    type Err = ColorChoiceParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "auto" => Ok(ColorChoice::Auto),
            "always" => Ok(ColorChoice::Always),
            "never" => Ok(ColorChoice::Never),
            _ => Err(ColorChoiceParseError {
                attempted: source.to_owned(),
            }),
        }
    }
}

impl From<ColorChoice> for termcolor::ColorChoice {
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Auto => termcolor::ColorChoice::Auto,
            ColorChoice::Always => termcolor::ColorChoice::Always,
            ColorChoice::Never => termcolor::ColorChoice::Never,
        }
    }
}

impl From<ColorChoice> for env_logger::WriteStyle {
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Auto => env_logger::WriteStyle::Auto,
            ColorChoice::Always => env_logger::WriteStyle::Always,
            ColorChoice::Never => env_logger::WriteStyle::Never,
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid color choice '{attempted}'. Valid values are: auto, always, never")]
pub struct ColorChoiceParseError {
    attempted: String,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Initialize a new ScriptSync project
    Init(InitCommand),
    /// Start the sync server
    Serve(ServeCommand),
    /// Pull scripts from a .rbxl/.rbxlx file
    Pull(PullCommand),
    /// Show tracked scripts and sync state
    Status(StatusCommand),
    /// Manage the Roblox Studio plugin
    Plugin(PluginCommand),
}

pub(super) fn resolve_path(path: &Path) -> Cow<'_, Path> {
    if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(env::current_dir().unwrap().join(path))
    }
}
