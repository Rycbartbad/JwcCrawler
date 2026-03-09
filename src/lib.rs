use std::error::Error;
use std::path::Path;
use crate::crawl::jwc::Jwc;
use crate::save::save;

pub mod models;
mod crawl;
mod save;

pub fn run(out: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let jwc = Jwc::new()?;
    save(Box::new(jwc), out)
}