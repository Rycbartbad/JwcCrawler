use crate::models::{DataSource, NewsItem};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::cell::Cell;
use std::error::Error;
use std::time::Duration;

pub struct Category {
    label: String,
    path: String,
    page: Cell<i32>,
    end_reached: Cell<bool>,
}
pub struct Jwc {
    base_url: String,
    categories: Vec<Category>,
    client: Client,
}

impl DataSource for Jwc {
    fn fetch(&mut self) -> Result<Vec<NewsItem>, Box<dyn Error>> {
        let mut all_news = Vec::new();

        for category in &self.categories {
            while !category.end_reached.get() {
                let current_page = category.page.get();

                let status = self.fetch_pages(category, current_page)?;

                if status.news_items.is_empty() {
                    category.end_reached.set(true);
                    break;
                }
                println!("{:#?}", status.news_items);
                all_news.extend(status.news_items);

                if status.has_next_page {
                    category.page.set(current_page + 1);
                } else {
                    category.end_reached.set(true);
                }
            }
        }

        Ok(all_news)
    }
}

pub struct FetchStatus {
    pub news_items: Vec<NewsItem>,
    pub has_next_page: bool,
}

impl Jwc {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            base_url: "https://jwc.seu.edu.cn".to_string(),
            categories: vec![
                Category {
                    label: "最新动态".to_string(),
                    path: "/zxdt/list.htm".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
                Category {
                    label: "教务信息".to_string(),
                    path: "/jwxx/list.htm".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
                Category {
                    label: "学籍管理".to_string(),
                    path: "/xjgl/list.htm".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
                Category {
                    label: "教学研究".to_string(),
                    path: "/jxyj/list.htm".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
                Category {
                    label: "实践教学".to_string(),
                    path: "/sjjx/list.htm".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
                Category {
                    label: "国际交流".to_string(),
                    path: "/gjjl/list.psp".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
                Category {
                    label: "文化素质教育".to_string(),
                    path: "/cbxx/list.htm".to_string(),
                    page: Cell::from(1),
                    end_reached: Cell::from(false),
                },
            ],
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(Duration::from_secs(10))
                .build()?,
        })
    }

    fn fetch_pages(
        &self,
        category: &Category,
        page: i32,
    ) -> Result<FetchStatus, Box<dyn Error>> {
        let final_path = if page == 1 {
            &category.path
        } else {
            &category.path.replace("list", &format!("list{}", page))
        };
        let url = format!("{}{}", self.base_url, final_path);

        let response_text = self.client.get(&url).send()?.text()?;
        let document = Html::parse_document(&response_text);

        let row_sel = Selector::parse("#wp_news_w8 table.main tr").unwrap();
        let link_sel = Selector::parse("a[title]").unwrap();
        let date_sel = Selector::parse("td.main div").unwrap();

        let mut items = Vec::new();

        // 5. 解析列表
        for row in document.select(&row_sel) {
            if let Some(link_el) = row.select(&link_sel).next() {
                let title = link_el.value().attr("title").unwrap_or("").to_string();
                let href = link_el.value().attr("href").unwrap_or("");

                let detail_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("{}{}", self.base_url, href)
                };

                let date = row
                    .select(&date_sel)
                    .next()
                    .map(|d| d.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                let content = if detail_url.starts_with("https://jwc.seu.edu.cn/"){
                    Some(self.fetch_content(&detail_url)?)
                } else {
                    None
                };
                items.push(NewsItem {
                    label: category.label.clone(),
                    title,
                    date,
                    detail_url,
                    content
                });
            }
        }

        // 6. 分页判断
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

    fn fetch_content(&self, url: &str) -> Result<String, Box<dyn Error>>{
        let text = self.client.get(url).send()?.text()?;
        let document = Html::parse_document(&text);

        let content_sel = Selector::parse("div.Article_Content").unwrap();

        if let Some(content_element) = document.select(&content_sel).next() {
            return Ok(content_element.inner_html())
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
        let s = jwc.fetch_content("https://jwc.seu.edu.cn/2021/1103/c21681a389469/page.psp").unwrap();
        println!("{s}");
    }
}