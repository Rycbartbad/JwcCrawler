use std::error::Error;
use std::path::Path;
use crate::crawl::jwc::Jwc;
use crate::models::DataSource;

pub mod models;
mod crawl;

pub fn run(out: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let mut jwc = Jwc::new()?;
    jwc.save_to_file(out)
}