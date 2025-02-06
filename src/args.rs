use std::path::PathBuf;
use std::sync::OnceLock;

use clap::Parser;

/// Custom greetd greeter for the Open Computing Facility
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Enable demo mode, which mocks the greetd connection
    #[arg(long)]
    pub demo: bool,

    /// The default session, e.g. "plasma"
    #[arg(long)]
    pub default_session: Option<String>,

    /// The background image to display, if any
    #[arg(long)]
    pub background: Option<PathBuf>,

    /// The logo image to display, if any
    #[arg(long)]
    pub logo: Option<PathBuf>,
}

static ARGS: OnceLock<Args> = OnceLock::new();

pub fn get_args() -> &'static Args {
    ARGS.get_or_init(Args::parse)
}
