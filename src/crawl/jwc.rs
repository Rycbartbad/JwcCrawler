use crate::models::{DataSource, NewsItem};
use rayon::prelude::*;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::error::Error;
use std::time::Duration;

struct Category {
    label: String,
    path: String,
    page: i32,
    end_reached: bool,
}
pub struct Jwc {
    base_url: String,
    categories: Vec<Category>,
    client: Client,
}

impl DataSource for Jwc {
    fn fetch(&mut self) -> Result<Vec<NewsItem>, Box<dyn Error>> {
        let mut all_news = Vec::new();

        let client = &self.client;
        let base_url = &self.base_url;

        for category in &mut self.categories {
            while !category.end_reached {
                let current_page = category.page;
                let status = Self::fetch_pages(base_url, client, category, current_page)?;

                if status.news_items.is_empty() {
                    category.end_reached = true;
                    break;
                }
                println!("{:#?}", status.news_items);
                all_news.extend(status.news_items);

                if status.has_next_page {
                    category.page += 1;
                } else {
                    category.end_reached = true;
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

impl Jwc {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            base_url: "https://jwc.seu.edu.cn".to_string(),
            categories: vec![
                Category {
                    label: "最新动态".to_string(),
                    path: "/zxdt/list.htm".to_string(),
                    page: 1,
                    end_reached: false,
                },
                Category {
                    label: "教务信息".to_string(),
                    path: "/jwxx/list.htm".to_string(),
                    page: 1,
                    end_reached: false,
                },
                Category {
                    label: "学籍管理".to_string(),
                    path: "/xjgl/list.htm".to_string(),
                    page: 1,
                    end_reached: false,
                },
                Category {
                    label: "教学研究".to_string(),
                    path: "/jxyj/list.htm".to_string(),
                    page: 1,
                    end_reached: false,
                },
                Category {
                    label: "实践教学".to_string(),
                    path: "/sjjx/list.htm".to_string(),
                    page: 1,
                    end_reached: false,
                },
                Category {
                    label: "国际交流".to_string(),
                    path: "/gjjl/list.psp".to_string(),
                    page: 1,
                    end_reached: false,
                },
                Category {
                    label: "文化素质教育".to_string(),
                    path: "/cbxx/list.htm".to_string(),
                    page: 1,
                    end_reached: false,
                },
            ],
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(Duration::from_secs(10))
                .build()?,
        })
    }

    fn fetch_pages(
        base_url: &String,
        client: &Client,
        category: &Category,
        page: i32
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

        // 使用 Rayon 并行抓取详情页正文
        let items: Vec<NewsItem> = rows_data
            .into_par_iter()
            .map(|(title, date, detail_url)| {
                let is_web_page = {
                    let url_lower = detail_url.to_lowercase();
                    !url_lower.ends_with(".pdf")
                        && !url_lower.ends_with(".doc")
                        && !url_lower.ends_with(".docx")
                        && !url_lower.ends_with(".xls")
                        && !url_lower.ends_with(".xlsx")
                        && !url_lower.ends_with(".zip")
                        && !url_lower.ends_with(".rar")
                };

                // 在并行线程中发起网络请求
                let content = if is_web_page && detail_url.starts_with(base_url) {
                    Self::fetch_content(client, &detail_url).ok()
                } else {
                    None
                };

                NewsItem {
                    label: category.label.clone(),
                    title,
                    date,
                    detail_url,
                    content,
                    is_page: is_web_page,
                }
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

    fn fetch_content(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
        let text = client.get(url).send()?.text()?;
        let document = Html::parse_document(&text);

        let content_sel = Selector::parse("div.Article_Content").unwrap();

        if let Some(content_element) = document.select(&content_sel).next() {
            return Ok(content_element.inner_html());
            // 获取纯文本内容 (去除所有 HTML 标签)
            /*let plain_text: String = content_element
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();*/
        }
        Ok("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fetch_summary() {
        let jwc = Jwc::new().unwrap();
        let s = Jwc::fetch_content(&jwc.client, "https://jwc.seu.edu.cn/2021/1103/c21681a389469/page.psp")
            .unwrap();
        println!("{s}");
    }
}
