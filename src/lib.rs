use crate::crawl::jwc::Jwc;
use crate::models::DataSource;
use clap::Parser;
use std::error::Error;
use std::fs;

pub mod models;
mod crawl;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    out: String,
    #[arg(short, long, help = "Fetch news after the given date. Fetch all if not passed")]
    date: Option<String>,
    #[arg(long, help = "Only fetch news with contents")]
    with_contents_only: bool
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let jwc = Jwc::new()?;
    let items = jwc.fetch(
        args.date.as_ref(),
        args.with_contents_only
    )?;
    
    let s = serde_json::to_string_pretty(&items)?;
    fs::write(args.out, s)?;
    Ok(())
}