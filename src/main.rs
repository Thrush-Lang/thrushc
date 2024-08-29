mod cli;
mod compiler;

use {
    self::cli::{logging::Logging, Cli},
    std::env::args,
};

fn main() {
    Cli::new(args().collect()).eval();
}
