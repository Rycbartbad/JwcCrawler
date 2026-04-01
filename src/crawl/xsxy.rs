use std::error::Error;
use crate::crawl::{Category, Crawler, SelectionConfig, SiteConfig};

pub fn get_xsxy() -> Result<Crawler, Box<dyn Error>> {
    let base_url =  "https://xsxy.seu.edu.cn".to_string();
    let categories = vec![
        Category {
            label: "新闻动态".to_string(),
            path: "/57140/list.htm".to_string(),
        },
        Category {
            label: "通知公告".to_string(),
            path: "/57141/list.htm".to_string()
        }
    ];
    let config = SiteConfig{
        name: "xsxy".to_string(),
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