use std::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[derive(Debug)]
pub(crate) struct NewsItem {
    pub label: String,
    pub title: String,
    pub date: String,
    pub detail_url: String,
    pub content: Option<String>
}

pub trait DataSource {
    fn fetch(&mut self) -> Result<Vec<NewsItem>, Box<dyn Error>>;
}