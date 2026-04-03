use crate::crawl::Crawler;
use crate::crawl::cs::get_cs;
use crate::crawl::jwc::get_jwc;
use crate::crawl::xsxy::get_xsxy;
use crate::models::DataSource;
use chrono::NaiveDate;
use clap::Parser;
use std::collections::HashMap;
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
        long,
        default_value = "jwc",
        help = "Data Sources, e.g. jwc, xsxy, etc."
    )]
    data_source: String,
    #[arg(
        short,
        long,
        help = "Fetch news after the given date. Fetch all if not passed. e.g. 2026-03-01"
    )]
    date: Option<String>,
    #[arg(long, help = "Only fetch news with contents")]
    with_contents_only: bool,
}

type CrawlerFactory = fn() -> Result<Crawler, Box<dyn Error>>;

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let crawler_map: HashMap<String, CrawlerFactory> = HashMap::from([
        ("jwc".to_string(), get_jwc as CrawlerFactory),
        ("xsxy".to_string(), get_xsxy as CrawlerFactory),
        ("cs".to_string(), get_cs as CrawlerFactory),
    ]);
    let factory = crawler_map.get(&args.data_source).ok_or_else(|| {
        format!(
            "Unsupported data source: {}. Currently support {}.",
            args.data_source,
            crawler_map
                .keys()
                .map(|it| it.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let crawler = factory()?;
    let date = args
        .date
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()?;

    let items = crawler.fetch(date, args.with_contents_only)?;

    let s = serde_json::to_string_pretty(&items)?;
    fs::write(args.out, s)?;
    Ok(())
}
