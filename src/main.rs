use std::error::Error;
use jwc_crawler::run;

fn main() -> Result<(), Box<dyn Error>>{
    run()
}
