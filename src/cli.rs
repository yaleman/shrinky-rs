use std::path::PathBuf;

use clap::Parser;

use crate::ImageFormat;

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
}
