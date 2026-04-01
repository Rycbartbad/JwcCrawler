use crate::crawl::Category;
use std::error::Error;
use crate::{Crawler, SelectionConfig, SiteConfig};

pub fn get_jwc() -> Result<Crawler, Box<dyn Error>> {
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
            current_page: "em.curr_page".to_string(),
            all_pages: "em.all_pages".to_string(),
        },
    };
    Crawler::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fetch_summary() {
        let jwc = get_jwc().unwrap();
        let s = Crawler::fetch_content(
            &jwc.client,
            "https://jwc.seu.edu.cn/2026/0126/c21676a553741/page.htm",
            &[".pdf", ".docx", ".doc", ".xlsx", ".xls", ".zip", ".rar"].map(|s| s.to_string()),
            &"div.Article_Content".to_string(),
        )
        .unwrap();
        println!("{s:?}");
    }
}
