use std::error::Error;
use std::fs;
use std::path::Path;
use crate::models::DataSource;

pub fn save(mut source: Box<dyn DataSource>, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>>{
    let v  = source.fetch()?;

    let s = serde_json::to_string_pretty(&v)?;
    fs::write(path, s)?;
    Ok(())

}