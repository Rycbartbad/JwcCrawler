use clap::Parser;
use jwc_crawler::{Args, run};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    run(args)
}
