use crate::crawl::jwc::Jwc;
use crate::models::DataSource;
use chrono::NaiveDate;
use clap::Parser;
use std::error::Error;
use std::fs;

mod crawl;
pub mod models;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, help = "Output file path")]
    out: String,
    #[arg(
        short,
        long,
        help = "Fetch news after the given date. Fetch all if not passed. e.g. 2026-03-01"
    )]
    date: Option<String>,
    #[arg(long, help = "Only fetch news with contents")]
    with_contents_only: bool,
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let jwc = Jwc::new()?;
    let date = args
        .date
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()?;

    let items = jwc.fetch(date, args.with_contents_only)?;

    let s = serde_json::to_string_pretty(&items)?;
    fs::write(args.out, s)?;
    Ok(())
}
