use crate::models::DataSource;
use chrono::NaiveDate;
use htmd::HtmlToMarkdown;
use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use url::Url;

pub mod jwc;
pub mod xsxy;

#[derive(Deserialize, Serialize, Debug)]
pub struct NewsItem {
    pub id: String,
    pub label: String,
    pub title: String,
    pub date: NaiveDate,
    pub detail_url: String,
    pub is_page: bool,
    pub content: Option<Content>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Content {
    pub text: String,
    pub attachment_urls: Vec<String>,
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Category {
    pub label: String,
    pub path: String,
}

pub struct FetchStatus {
    pub news_items: Vec<NewsItem>,
    pub has_next_page: bool,
}

pub struct Crawler {
    config: SiteConfig,
    client: Client,
    attachment_extensions: Vec<String>,
}

#[derive(Clone)]
pub struct SiteConfig {
    pub name: String,
    pub base_url: String,
    pub categories: Vec<Category>,
    pub selectors: SelectionConfig,
}

#[derive(Clone)]
pub struct SelectionConfig {
    pub list_row: String,
    pub list_title_link: String,
    pub list_date: String,
    pub content_body: String,
    pub current_page: String,
    pub all_pages: String,
}

impl DataSource for Crawler {
    fn fetch(
        &self,
        date_after: Option<NaiveDate>,
        with_contents_only: bool,
    ) -> Result<Vec<NewsItem>, Box<dyn Error>> {
        let mut all_news = Vec::new();

        let client = &self.client;
        let base_url = &self.config.base_url;
        let categories = &self.config.categories;
        let extensions = &self.attachment_extensions;
        let selectors = &self.config.selectors;
        let mut end_reached_map: HashMap<&Category, bool> = HashMap::new();
        let mut page_map: HashMap<&Category, i32> = HashMap::new();
        for category in categories {
            end_reached_map.insert(category, false);
            page_map.insert(category, 1);
        }

        for category in categories {
            while !end_reached_map[category] {
                let current_page = page_map[category];
                let status = Self::fetch_pages(
                    base_url,
                    client,
                    category,
                    extensions,
                    selectors,
                    current_page,
                    date_after,
                    with_contents_only,
                )?;

                if status.news_items.is_empty() {
                    end_reached_map.insert(category, true);
                    break;
                }
                println!("{:#?}", status.news_items);
                all_news.extend(status.news_items);

                if status.has_next_page {
                    page_map.insert(category, page_map[category] + 1);
                } else {
                    end_reached_map.insert(category, true);
                }
            }
        }
        Ok(all_news)
    }
}

impl Crawler {
    pub fn new(config: SiteConfig) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config,
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(Duration::from_secs(10))
                .build()?,
            attachment_extensions: [".pdf", ".docx", ".doc", ".xlsx", ".xls", ".zip", ".rar"]
                .iter()
                .map(|x| x.to_string())
                .collect(),
        })
    }

    fn generate_key(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let result = hasher.finalize();
        result
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    fn fetch_pages(
        base_url: &String,
        client: &Client,
        category: &Category,
        attachment_extensions: &[String],
        selection_config: &SelectionConfig,
        page: i32,
        date_after: Option<NaiveDate>,
        with_contents_only: bool,
    ) -> Result<FetchStatus, Box<dyn Error>> {
        let final_path = if page == 1 {
            &category.path
        } else {
            &category.path.replace("list", &format!("list{}", page))
        };
        let url = format!("{}{}", base_url, final_path);

        let response_text = client.get(&url).send()?.text()?;
        let document = Html::parse_document(&response_text);

        let row_sel = Selector::parse(&selection_config.list_row).unwrap();
        let link_sel = Selector::parse(&selection_config.list_title_link).unwrap();
        let date_sel = Selector::parse(&selection_config.list_date).unwrap();

        let rows_data: Vec<_> = document
            .select(&row_sel)
            .filter_map(|row| {
                let link_el = row.select(&link_sel).next()?;
                let title = link_el.value().attr("title")?.to_string();
                let href = link_el.value().attr("href")?;
                let date = row
                    .select(&date_sel)
                    .next()
                    .map(|d| d.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                let detail_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("{}{}", base_url, href)
                };

                Some((title, date, detail_url))
            })
            .collect();

        let items: Vec<NewsItem> = rows_data
            .into_par_iter()
            .filter_map(move |(title, date, detail_url)| {
                let news_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").unwrap_or_else(|e1| {
                    eprintln!("Error when parsing date from {title}: {e1}");
                    NaiveDate::default()
                });
                if let Some(target) = date_after
                    && news_date < target
                {
                    return None;
                }

                let url_lower = detail_url.to_lowercase();
                let is_web_page = !attachment_extensions.iter().any(|x| url_lower.ends_with(x));

                let mut content = None;
                if is_web_page && detail_url.starts_with(base_url) {
                    content = Crawler::fetch_content(
                        client,
                        &detail_url,
                        attachment_extensions,
                        &selection_config.content_body,
                    )
                    .ok();
                }

                if with_contents_only && content.is_none() {
                    return None;
                }

                Some(NewsItem {
                    id: Self::generate_key(&detail_url),
                    label: category.label.clone(),
                    title,
                    date: news_date,
                    detail_url,
                    content,
                    is_page: is_web_page,
                })
            })
            .collect();

        let curr_sel = Selector::parse(&selection_config.current_page).unwrap();
        let all_sel = Selector::parse(&selection_config.all_pages).unwrap();

        let extract_num = |sel: &Selector| {
            document
                .select(sel)
                .next()
                .and_then(|e| e.text().collect::<String>().trim().parse::<i32>().ok())
                .unwrap_or(1)
        };

        let current = extract_num(&curr_sel);
        let total = extract_num(&all_sel);

        Ok(FetchStatus {
            news_items: items,
            has_next_page: current < total,
        })
    }

    fn fetch_content(
        client: &Client,
        url: &str,
        extensions: &[String],
        content_body_sel: &String, // Content Body selection string, e.g. div.Article_Content
    ) -> Result<Content, Box<dyn Error>> {
        let base_url = Url::parse(url)?;

        let text = client.get(url).send()?.text()?;
        let document = Html::parse_document(&text);

        let content_sel = Selector::parse(content_body_sel).unwrap();

        if let Some(content_element) = document.select(&content_sel).next() {
            let plain_text = get_pretty_text(content_element, &base_url);
            let mut attachment_urls = Vec::new();
            let all_elements_sel = Selector::parse("*").unwrap();

            for element in content_element.select(&all_elements_sel) {
                let mut process_link = |raw_url: &str| {
                    if let Ok(full_url) = base_url.join(raw_url) {
                        let url_str = full_url.to_string();
                        let lower_url = url_str.to_lowercase();
                        if extensions.iter().any(|ext| lower_url.ends_with(ext)) {
                            attachment_urls.push(url_str);
                        }
                    }
                };

                if let Some(href) = element.value().attr("href") {
                    process_link(href);
                }
                if let Some(pdfsrc) = element.value().attr("pdfsrc") {
                    process_link(pdfsrc);
                }
            }

            attachment_urls.sort();
            attachment_urls.dedup();

            Ok(Content {
                text: plain_text,
                attachment_urls,
            })
        } else {
            Err("Content not found".into())
        }
    }
}

pub(crate) fn get_pretty_text(element: ElementRef, base_url: &Url) -> String {
    let html_fragment = element.html();

    let pre_cleaned_html = html_fragment.replace("&nbsp;", " ").replace("&#160;", " ");

    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "colgroup", "col"])
        .build();

    let raw_markdown = converter.convert(&pre_cleaned_html).unwrap_or_default();

    let cleaned = fix_markdown_links(&raw_markdown, base_url).replace("||", "|\n|");
    let re_multi_spaces = Regex::new(r"[ \t]{2,}").unwrap();

    let lines: Vec<String> = cleaned
        .lines()
        .map(|line| {
            let t = line.trim().replace("&nbsp;", " ").replace("\u{a0}", " ");
            re_multi_spaces.replace_all(&t, " ").to_string()
        })
        .filter(|line| !line.is_empty())
        .collect();
    let mut result = String::new();
    for i in 0..lines.len() {
        result.push_str(&lines[i]);
        if i + 1 < lines.len() {
            if lines[i].starts_with('|') && lines[i + 1].starts_with('|') {
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

    let cleaned = re
        .replace_all(md, |caps: &regex::Captures| {
            let prefix = &caps["p"];
            let link = &caps["u"];
            if let Ok(absolute_url) = base_url.join(link) {
                let url_str = absolute_url.to_string();
                if url_str.contains("icon_") {
                    return "".to_string();
                }
                format!("{}({})", prefix, url_str)
            } else {
                format!("{}({})", prefix, link)
            }
        })
        .to_string();
    cleaned
}

fn fix_markdown_table_separator(md: &str) -> String {
    let mut lines: Vec<String> = md.lines().map(|s| s.to_string()).collect();
    if lines.len() < 2 {
        return md.to_string();
    }

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
