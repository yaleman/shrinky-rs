use crate::ImageFormat;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "Shrinky", version = env!("CARGO_PKG_VERSION"), author = "James Hodgkinson", about = "A simple image optimization tool")]
pub struct Cli {
    /// Activate debug mode
    #[arg(long, default_value = "false", env = "SHRINKY_DEBUG")]
    pub debug: bool,

    /// Set the output format
    #[arg(short = 't', long, env = "SHRINKY_TYPE")]
    pub output_type: Option<ImageFormat>,

    /// Delete the source file
    #[arg(short, long, default_value = "false", env = "SHRINKY_DELETE")]
    pub delete: bool,

    /// Geometry options, eg. 800x, x800, 800x600
    #[arg(short, long, env = "SHRINKY_GEOMETRY")]
    pub geometry: Option<String>,

    /// input filename
    pub filename: PathBuf,

    /// Overwrite existing files without prompting
    #[arg(short, long, default_value = "false", env = "SHRINKY_FORCE")]
    pub force: bool,

    /// Show image info and return
    #[arg(short, long, default_value = "false")]
    pub info: bool,
}

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

pub fn test_setup_logging() {
    let _ = stderrlog::new()
        .verbosity(log::Level::Debug)
        .show_module_names(true)
        .init();
}
