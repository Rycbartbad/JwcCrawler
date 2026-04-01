use chrono::NaiveDate;
use htmd::HtmlToMarkdown;
use regex::Regex;
use scraper::ElementRef;
use serde::{Deserialize, Serialize};
use url::Url;

pub mod jwc;

#[derive(Deserialize, Serialize)]
#[derive(Debug)]
pub struct NewsItem {
    pub id: String,
    pub label: String,
    pub title: String,
    pub date: NaiveDate,
    pub detail_url: String,
    pub is_page: bool,
    pub content: Option<Content>,
}

#[derive(Deserialize, Serialize)]
#[derive(Debug)]
pub struct Content {
    pub text: String,
    pub attachment_urls: Vec<String>,
}

#[derive(Eq, Hash, PartialEq)]
#[derive(Clone)]
pub struct Category {
    pub label: String,
    pub path: String,
}

pub struct FetchStatus {
    pub news_items: Vec<NewsItem>,
    pub has_next_page: bool,
}
pub(crate) fn get_pretty_text(element: ElementRef, base_url: &Url) -> String {
    let html_fragment = element.html();

    let pre_cleaned_html = html_fragment
        .replace("&nbsp;", " ")
        .replace("&#160;", " ");

    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "colgroup", "col"])
        .build();

    let raw_markdown = converter
        .convert(&pre_cleaned_html)
        .unwrap_or_default();

    let cleaned = fix_markdown_links(&raw_markdown, base_url).replace("||", "|\n|");
    let re_multi_spaces = Regex::new(r"[ \t]{2,}").unwrap();

    let lines: Vec<String> = cleaned
        .lines()
        .map(|line| {
            let t = line.trim()
                .replace("&nbsp;", " ")
                .replace("\u{a0}", " ");
            re_multi_spaces.replace_all(&t, " ").to_string()
        })
        .filter(|line| !line.is_empty())
        .collect();
    let mut result = String::new();
    for i in 0..lines.len() {
        result.push_str(&lines[i]);
        if i + 1 < lines.len() {
            if lines[i].starts_with('|') && lines[i+1].starts_with('|') {
                result.push('\n');
            } else {
                result.push_str("\n\n");
            }
        }
    }
    fix_markdown_table_separator(&result)
}

fn fix_markdown_links(md: &str, base_url: &Url) -> String {
    let re = Regex::new(r"(?P<p>!?\[.*?])\((?P<u>[^ )]+)(?:\s+.*?)?\)").unwrap();

    let cleaned = re.replace_all(md, |caps: &regex::Captures| {
        let prefix = &caps["p"];
        let link = &caps["u"];
        if let Ok(absolute_url) = base_url.join(link) {
            let url_str = absolute_url.to_string();
            if url_str.contains("icon_") { return "".to_string(); }
            format!("{}({})", prefix, url_str)
        } else {
            format!("{}({})", prefix, link)
        }
    }).to_string();
    cleaned
}

fn fix_markdown_table_separator(md: &str) -> String {
    let mut lines: Vec<String> = md.lines().map(|s| s.to_string()).collect();
    if lines.len() < 2 { return md.to_string(); }

    if let Some(header_idx) = lines.iter().position(|l| l.trim().starts_with('|')) {
        let column_count = lines[header_idx].matches('|').count().saturating_sub(1);

        if column_count > 0 {
            let separator = format!("| {} |", vec!["---"; column_count].join(" | "));
            let has_sep = if header_idx + 1 < lines.len() {
                lines[header_idx + 1].contains("---")
            } else {
                false
            };

            if !has_sep {
                lines.insert(header_idx + 1, separator);
            }
        }
    }
    lines.join("\n")
}