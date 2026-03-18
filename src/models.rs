use std::error::Error;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[derive(Debug)]
pub struct NewsItem {
    pub id: String,
    pub label: String,
    pub title: String,
    pub date: String,
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
    fn fetch(&mut self) -> Result<Vec<NewsItem>, Box<dyn Error>>;

    fn save_to_file(&mut self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let v  = self.fetch()?;

        let s = serde_json::to_string_pretty(&v)?;
        fs::write(path, s)?;
        Ok(())
    }
}
