use std::error::Error;
use clap::Parser;
use jwc_crawler::{run, Args};

fn main() -> Result<(), Box<dyn Error>>{
    let args = Args::parse();
    run(args)
}
