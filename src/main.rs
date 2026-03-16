use std::{env, panic, process};

use backtrace::Backtrace;
use clap::Parser;

use libscriptsync::cli::Options;

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(&message) => message.to_string(),
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(message) => message.clone(),
                None => "<no message>".to_string(),
            },
        };

        log::error!(
            "ScriptSync crashed! You are running ScriptSync {}.",
            env!("CARGO_PKG_VERSION")
        );
        log::error!("Details: {}", message);

        if let Some(location) = panic_info.location() {
            log::error!("in file {} on line {}", location.file(), location.line());
        }

        let should_backtrace = env::var("RUST_BACKTRACE")
            .map(|var| var == "1")
            .unwrap_or(false);

        if should_backtrace {
            eprintln!("{:?}", Backtrace::new());
        } else {
            eprintln!(
                "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace."
            );
        }

        process::exit(1);
    }));

    let options = Options::parse();

    let log_filter = match options.global.verbosity {
        0 => "info",
        1 => "info,libscriptsync=debug",
        2 => "info,libscriptsync=trace",
        _ => "trace",
    };

    let log_env = env_logger::Env::default().default_filter_or(log_filter);

    env_logger::Builder::from_env(log_env)
        .format_module_path(false)
        .format_timestamp(None)
        .format_indent(Some(8))
        .write_style(options.global.color.into())
        .init();

    if let Err(err) = options.run() {
        log::error!("{:?}", err);
        process::exit(1);
    }
}
