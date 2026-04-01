use crate::crawl::jwc::{Crawler, SelectionConfig, SiteConfig};
use crate::models::DataSource;
use chrono::NaiveDate;
use clap::Parser;
use std::error::Error;
use std::fs;
use crate::crawl::Category;

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
    let base_url =  "https://jwc.seu.edu.cn".to_string();
    let categories =  vec![
        Category {
            label: "最新动态".to_string(),
            path: "/zxdt/list.htm".to_string(),
        },
        Category {
            label: "教务信息".to_string(),
            path: "/jwxx/list.htm".to_string(),
        },
        Category {
            label: "学籍管理".to_string(),
            path: "/xjgl/list.htm".to_string(),
        },
        Category {
            label: "教学研究".to_string(),
            path: "/jxyj/list.htm".to_string(),
        },
        Category {
            label: "实践教学".to_string(),
            path: "/sjjx/list.htm".to_string(),
        },
        Category {
            label: "国际交流".to_string(),
            path: "/gjjl/list.psp".to_string(),
        },
        Category {
            label: "文化素质教育".to_string(),
            path: "/cbxx/list.htm".to_string(),
        },
    ];

    let config = SiteConfig{
        name: "教务处".to_string(),
        base_url,
        categories,
        selectors: SelectionConfig {
            list_row: "#wp_news_w8 table.main tr".to_string(),
            list_title_link: "a[title]".to_string(),
            list_date: "td.main div".to_string(),
            content_body: "div.Article_Content".to_string(),
        },
    };
    let jwc = Crawler::new(config)?;
    let date = args
        .date
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()?;

    let items = jwc.fetch(date, args.with_contents_only)?;

    let s = serde_json::to_string_pretty(&items)?;
    fs::write(args.out, s)?;
    Ok(())
}
