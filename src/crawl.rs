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

pub mod cs;
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
    keep_complex_tables: bool,
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
        let keep_complex_tables = self.keep_complex_tables;
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
                    keep_complex_tables,
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
            keep_complex_tables: false,
        })
    }

    pub fn set_keep_complex_tables(&mut self, value: bool) {
        self.keep_complex_tables = value;
    }

    pub fn fetch_url(&self, url: &str, content_body_sel: &str) -> Result<Content, Box<dyn Error>> {
        Self::fetch_content(
            &self.client,
            url,
            &self.attachment_extensions,
            &content_body_sel.to_string(),
            self.keep_complex_tables,
        )
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
        keep_complex_tables: bool,
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
                        keep_complex_tables,
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
        keep_complex_tables: bool,
    ) -> Result<Content, Box<dyn Error>> {
        let base_url = Url::parse(url)?;

        let text = client.get(url).send()?.text()?;
        let document = Html::parse_document(&text);

        let content_sel = Selector::parse(content_body_sel).unwrap();

        if let Some(content_element) = document.select(&content_sel).next() {
            let plain_text = if keep_complex_tables {
                get_pretty_text_with_complex_tables(content_element, &base_url)
            } else {
                get_pretty_text(content_element, &base_url)
            };
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
    clean_markdown(&fix_markdown_table_separator(&result))
}

fn get_pretty_text_with_complex_tables(element: ElementRef, base_url: &Url) -> String {
    let html_fragment = element.html();
    let pre_cleaned_html = html_fragment.replace("&nbsp;", " ").replace("&#160;", " ");

    let (processed_html, table_replacements) =
        replace_complex_tables_with_placeholders(&pre_cleaned_html);

    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "colgroup", "col"])
        .build();

    let raw_markdown = converter.convert(&processed_html).unwrap_or_default();
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
    let markdown = clean_markdown(&fix_markdown_table_separator(&result));

    let mut result = markdown;
    for (placeholder, cleaned_table_html) in table_replacements {
        result = result.replace(&placeholder, &cleaned_table_html);
    }

    result
}

fn replace_complex_tables_with_placeholders(
    html: &str,
) -> (String, Vec<(String, String)>) {
    let document = Html::parse_document(html);
    let mut replacements: Vec<(String, String)> = Vec::new();
    let mut placeholder_index = 0;


    let table_sel = Selector::parse("table").unwrap();
    let td_th_sel = Selector::parse("td, th").unwrap();

    let mut work_html = html.to_string();

    for table in document.select(&table_sel) {
        let mut has_complex_cell = false;
        for cell in table.select(&td_th_sel) {
            if cell.value().attr("rowspan").is_some() || cell.value().attr("colspan").is_some()
            {
                has_complex_cell = true;
                break;
            }
        }

        if has_complex_cell {
            let table_html = table.html();
            let placeholder = format!("__TABLE_PLACEHOLDER_{}__", placeholder_index);
            let cleaned_table = clean_html_table(&table_html);
            replacements.push((placeholder.clone(), cleaned_table));

            work_html = work_html.replace(&table_html, &placeholder);
            placeholder_index += 1;
        }
    }

    (work_html, replacements)
}

fn clean_html_table(html: &str) -> String {
    let allowed_attrs = [
        "rowspan", "colspan", "valign", "align", "href", "src", "alt", "title", "width", "height",
    ];

    let re_attr = Regex::new(r#"(\w+)=["'][^"']*["']"#).unwrap();

    let result = re_attr.replace_all(html, |caps: &regex::Captures| {
        let attr_name = &caps[1];
        if allowed_attrs.contains(&attr_name) {
            caps[0].to_string()
        } else {
            String::new()
        }
    });

    let re_empty_attrs = Regex::new(r#"\s+\w+=""|\w+=""\s+"#).unwrap();
    let result = re_empty_attrs.replace_all(&result, " ").to_string();

    result.trim().to_string()
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

fn clean_markdown(markdown: &str) -> String {
    let re_extra_asterisks = Regex::new(r"\*{4}").unwrap();

    let result = remove_empty_bold_pairs(markdown);
    let result = re_extra_asterisks.replace_all(&result, "");

    result.to_string()
}

fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(c, '，' | '。' | '！' | '？' | '；' | '：' | '"' | '\'' | '（' | '）' | '【' | '】' | '《' | '》' | '…' | '、')
}

fn remove_empty_bold_pairs(md: &str) -> String {
    let chars: Vec<char> = md.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            i += 2;

            let mut temp = String::new();
            let mut found_end = false;

            while i + 1 < chars.len() {
                if chars[i] == '*' && chars[i + 1] == '*' {
                    found_end = true;
                    i += 2; 
                    break;
                }
                temp.push(chars[i]);
                i += 1;
            }

            if found_end && temp.chars().all(|c| c.is_whitespace()) {
                continue;
            } else if found_end {
                result.push_str("**");
                result.push_str(&temp);
                result.push_str("**");

                if i < chars.len() {
                    let next_char = chars[i];
                    if !next_char.is_whitespace() && !is_punctuation(next_char) {
                        result.push(' ');
                    }
                }
            } else {
                result.push_str("**");
                result.push_str(&temp);
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}
