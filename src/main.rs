use std::{env, io::Write, process};

use anyhow::Context;
use clap::Parser;

mod ao3;
mod extractor;

#[derive(Parser)]
struct Cli {
    id: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let args = Cli::parse();

    let username = match env::var("USERNAME") {
        Ok(u) => u,
        Err(env::VarError::NotPresent) => {
            let mut tmp = String::new();
            print!("Username? ");
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_line(&mut tmp).unwrap();
            tmp.pop(); // the newline
            tmp
        },
        Err(env::VarError::NotUnicode(_)) => {
            log::error!("Found USERNAME env var, but the contents were not valid Unicode!");
            process::exit(1);
        },
    };

    let password = match env::var("PASSWORD") {
        Ok(p) => p,
        Err(env::VarError::NotPresent) => {
            rpassword::prompt_password("Password? ").unwrap()
        },
        Err(env::VarError::NotUnicode(_)) => {
            log::error!("Found PASSWORD env var, but the contents were not valid Unicode!");
            process::exit(1);
        },
    };

    log::debug!("Got username and password");

    let client = ao3::make_client()
        .context("Could not make client (this is not user error and should never happen)")?;

    log::debug!("Successfully created client");

    log::debug!("Attempting to log in");

    ao3::login(&client, &username, &password)
        .await
        .context("Could not log in. Check your username/password")?;

    log::info!("Successfully logged in");

    log::debug!("Attempting to download work with ID {}", &args.id);

    let bytes = ao3::download(&client, &args.id)
        .await
        .context("Could not download data")?;

    log::info!("Successfully downloaded work with ID {}", &args.id);

    log::debug!("Attempting to parse download as ZIP");

    let mut zipped_epub = extractor::as_zip(bytes)
        .context("Could not parse download as ZIP")?;

    log::info!("Successfully parsed download as ZIP");

    log::debug!("Attempting to extract title of work with ID {}", &args.id);

    let file_path = match extractor::title(&mut zipped_epub) {
        Ok(title) => {
            log::info!("Extracted title '{}' for work with ID {}", &title, &args.id);
            format!("{} [ao3 {}].epub", title, &args.id)
        },
        Err(e) => {
            log::warn!("Could not extract title for fic with ID {}", &args.id);
            log::warn!("Error: {}", e);
            format!("[ao3 {}].epub", &args.id)
        },
    };

    log::debug!("Extracting work to path '{}'", &file_path);

    extractor::unzip_to(&mut zipped_epub, &file_path)
        .context("Could not unzip EPUB")?;

    log::info!("Successfully extracted work to path '{}'", &file_path);

    Ok(())
}
