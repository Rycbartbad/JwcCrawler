use crate::models::{Content, DataSource, NewsItem};
use chrono::NaiveDate;
use htmd::HtmlToMarkdown;
use rayon::prelude::*;
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use url::Url;

#[derive(Eq, Hash, PartialEq)]
struct Category {
    label: String,
    path: String,
}
pub struct Jwc {
    base_url: String,
    categories: Vec<Category>,
    client: Client,
    attachment_extensions: Vec<String>,
}

impl DataSource for Jwc {
    fn fetch(
        &self,
        date_after: Option<NaiveDate>,
        with_contents_only: bool,
    ) -> Result<Vec<NewsItem>, Box<dyn Error>> {
        let mut all_news = Vec::new();

        let client = &self.client;
        let base_url = &self.base_url;
        let mut end_reached_map: HashMap<&Category, bool> = HashMap::new();
        let mut page_map: HashMap<&Category, i32> = HashMap::new();
        for category in &self.categories {
            end_reached_map.insert(category, false);
            page_map.insert(category, 1);
        }

        for category in &self.categories {
            while !end_reached_map[category] {
                let current_page = page_map[category];
                let status = Self::fetch_pages(
                    base_url,
                    client,
                    category,
                    &self.attachment_extensions,
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

struct FetchStatus {
    news_items: Vec<NewsItem>,
    has_next_page: bool,
}

fn get_pretty_text(element: ElementRef, base_url: &Url) -> String {
    let html_fragment = element.html();
    let converter = HtmlToMarkdown::new();
    let raw_markdown = converter
        .convert(&html_fragment)
        .unwrap_or_else(|_| "".to_string());

    let cleaned = fix_markdown_links(&raw_markdown, base_url);

    let re_multi_spaces = Regex::new(r"[ \t]{2,}").unwrap(); // 匹配 2 个及以上的连续空格或制表符
    let re_extra_newlines = Regex::new(r"\n{3,}").unwrap(); // 匹配 3 个及以上的连续换行

    let result = cleaned
        .lines()
        .map(|line| {
            let t = line.trim();
            re_multi_spaces.replace_all(t, " ").to_string()
        })
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("![")) // 过滤图标
        .collect::<Vec<_>>()
        .join("\n");

    re_extra_newlines.replace_all(&result, "\n\n").to_string()
}

fn fix_markdown_links(md: &str, base_url: &Url) -> String {
    // 1. (!?\[.*?\]) 匹配链接文本部分，如 [附件1...]
    // 2. \( 匹配左括号
    // 3. (?P<u>[^ \)]+) 匹配真正的 URL 部分，直到遇到空格或右括号为止
    // 4. (?:\s+.*?)? 匹配并丢弃括号里的空格及其后的 title/attr
    // 5. \) 匹配右括号
    let re = Regex::new(r"(?P<p>!?\[.*?\])\((?P<u>[^ \)]+)(?:\s+.*?)?\)").unwrap();

    re.replace_all(md, |caps: &regex::Captures| {
        let prefix = &caps["p"];
        let link = &caps["u"];

        // 尝试转为绝对路径
        if let Ok(absolute_url) = base_url.join(link) {
            // 过滤掉教务处常见的 UI 图标链接
            let url_str = absolute_url.to_string();
            if url_str.contains("default/images/icon_") {
                return "".to_string();
            }
            format!("{}({})", prefix, url_str)
        } else {
            format!("{}({})", prefix, link)
        }
    })
    .to_string()
}

impl Jwc {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            base_url: "https://jwc.seu.edu.cn".to_string(),
            categories: vec![
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
            ],
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

        let row_sel = Selector::parse("#wp_news_w8 table.main tr").unwrap();
        let link_sel = Selector::parse("a[title]").unwrap();
        let date_sel = Selector::parse("td.main div").unwrap();

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
                    content = Jwc::fetch_content(client, &detail_url, attachment_extensions).ok();
                }

                if with_contents_only && content.is_none() {
                    return None;
                }

                // 5. 构建结果
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

        let curr_sel = Selector::parse("em.curr_page").unwrap();
        let all_sel = Selector::parse("em.all_pages").unwrap();

        let current = document
            .select(&curr_sel)
            .next()
            .and_then(|e| e.text().collect::<String>().parse::<i32>().ok())
            .unwrap_or(1);

        let total = document
            .select(&all_sel)
            .next()
            .and_then(|e| e.text().collect::<String>().parse::<i32>().ok())
            .unwrap_or(1);

        Ok(FetchStatus {
            news_items: items,
            has_next_page: current < total,
        })
    }

    fn fetch_content(
        client: &Client,
        url: &str,
        extensions: &[String],
    ) -> Result<Content, Box<dyn Error>> {
        let base_url = Url::parse(url)?;

        let text = client.get(url).send()?.text()?;
        let document = Html::parse_document(&text);

        let content_sel = Selector::parse("div.Article_Content").unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fetch_summary() {
        let jwc = Jwc::new().unwrap();
        let s = Jwc::fetch_content(
            &jwc.client,
            "https://jwc.seu.edu.cn/2026/0126/c21676a553741/page.htm",
            &[".pdf", ".docx", ".doc", ".xlsx", ".xls", ".zip", ".rar"].map(|s| s.to_string()),
        )
        .unwrap();
        println!("{s:?}");
    }
}
