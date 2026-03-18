use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Deserialize, Serialize)]
#[derive(Debug)]
pub struct NewsItem {
    pub id: String,
    pub label: String,
    pub title: String,
    pub date: String, // Should be "YYYY-mm-dd", e.g. "2026-03-18"
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


pub trait DataSource {
    fn fetch(&self, date_after: Option<&String>, with_contents_only: bool) -> Result<Vec<NewsItem>, Box<dyn Error>>;
}
