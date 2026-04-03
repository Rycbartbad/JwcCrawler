use crate::crawl::{Category, Crawler, SelectionConfig, SiteConfig};
use std::error::Error;

pub fn get_cs() -> Result<Crawler, Box<dyn Error>> {
    let base_url = "https://cs.seu.edu.cn".to_string();
    let categories = vec![
        Category {
            label: "学院新闻".to_string(),
            path: "/news/list.htm".to_string(),
        },
        Category {
            label: "通知公告".to_string(),
            path: "/49342/list.htm".to_string(),
        },
        Category {
            label: "学术活动".to_string(),
            path: "/xshd_53564/list.htm".to_string(),
        },
    ];
    let config = SiteConfig {
        name: "计算机科学与工程学院".to_string(),
        base_url,
        categories,
        selectors: SelectionConfig {
            list_row: "ul.news_list li.news".to_string(),
            list_title_link: "span.news_title a".to_string(),
            list_date: "span.news_meta".to_string(),
            content_body: "div.wp_articlecontent".to_string(),
            current_page: "em.curr_page".to_string(),
            all_pages: "em.all_pages".to_string(),
        },
    };
    Crawler::new(config)
}
