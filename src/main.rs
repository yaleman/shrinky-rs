use clap::Parser;
use log::error;
use shrinky_rs::{cli::Cli, imagedata::Geometry, process_image};
use std::{cmp::max, process::ExitCode, str::FromStr};

pub fn setup_logging(debug: bool) {
    let log_level = if debug {
        log::Level::Debug
    } else {
        log::Level::Info
    };
    if let Err(err) = stderrlog::new()
        .verbosity(log_level)
        .show_module_names(debug)
        .init()
    {
        eprintln!("Failed to initialize logger: {}", err);
        std::process::exit(1);
    }
}

fn aggregate_exit_code(current: u8, next: u8) -> u8 {
    max(current, next)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.debug);

    let target_geometry = match cli.geometry.as_deref() {
        Some(target_geometry) => match Geometry::from_str(target_geometry) {
            Ok(geometry) if geometry.is_empty() => None,
            Ok(geometry) => Some(geometry),
            Err(e) => {
                error!("Error parsing geometry: {:?}", e);
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    let mut exit_code = 0;
    for filename in &cli.filenames {
        let current_exit_code = process_image(&cli, target_geometry.as_ref(), filename.as_path());
        exit_code = aggregate_exit_code(exit_code, current_exit_code);
    }

    ExitCode::from(exit_code)
}

#[cfg(test)]
mod tests {
    use super::aggregate_exit_code;

    #[test]
    fn test_aggregate_exit_code_all_success() {
        let mut exit_code = 0;
        for current_code in [0, 0, 0] {
            exit_code = aggregate_exit_code(exit_code, current_code);
        }

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_aggregate_exit_code_uses_highest_failure() {
        let mut exit_code = 0;
        for current_code in [1, 3, 2, 1] {
            exit_code = aggregate_exit_code(exit_code, current_code);
        }

        assert_eq!(exit_code, 3);
    }
}
