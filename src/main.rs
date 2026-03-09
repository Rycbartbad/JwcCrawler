use std::error::Error;
use clap::Parser;
use jwc_crawler::run;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    out: String,
}

fn main() -> Result<(), Box<dyn Error>>{
    let args = Args::parse();
    run(args.out)
}
