use std::{env, fs, io::Write, process};

use clap::Parser;

mod ao3;
mod extractor;

#[derive(Parser)]
struct Cli {
    id: usize,
}

#[tokio::main]
async fn main() {
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
            eprintln!("Found USERNAME env var, but the contents were not valid Unicode!");
            process::exit(1);
        },
    };

    let password = match env::var("PASSWORD") {
        Ok(p) => p,
        Err(env::VarError::NotPresent) => {
            rpassword::prompt_password("Password? ").unwrap()
        },
        Err(env::VarError::NotUnicode(_)) => {
            eprintln!("Found PASSWORD env var, but the contents were not valid Unicode!");
            process::exit(1);
        },
    };

    let client = ao3::make_client().unwrap();
    ao3::login(&client, &username, &password).await.unwrap();

    let bytes = ao3::download(&client, &args.id)
        .await
        .unwrap();

    fs::write("/tmp/dl.dat", &bytes);

    // let b = std::fs::read("/tmp/test.epub").unwrap();
    // let bytes = bytes::Bytes::copy_from_slice(&b);

    let mut zipped_epub = extractor::as_zip(bytes)
        .unwrap();

    let title = extractor::title(&mut zipped_epub).unwrap();

    println!("Extracted title '{}'", &title);

    extractor::unzip_to(&mut zipped_epub, format!("{} [ao3 {}].epub", title, &args.id)).unwrap();
}
