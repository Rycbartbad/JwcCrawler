use std::error::Error;
use crate::crawl::jwc::Jwc;
use crate::save::save;

pub mod models;
mod crawl;
mod save;

pub fn run() -> Result<(), Box<dyn Error>> {
    let jwc = Jwc::new()?;
    save(Box::new(jwc), "output.json")
}