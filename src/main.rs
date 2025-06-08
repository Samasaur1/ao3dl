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

    // let client = ao3::make_client().unwrap();
    // ao3::login(&client).await.unwrap();
    //
    // let bytes = ao3::download(&client, &args.id)
    //     .await
    //     .unwrap();


    let b = std::fs::read("/tmp/test.epub").unwrap();
    let bytes = bytes::Bytes::copy_from_slice(&b);

    let mut zipped_epub = extractor::as_zip(bytes)
        .unwrap();

    let title = extractor::title(&mut zipped_epub).unwrap();

    println!("Extracted title '{}'", &title);

    extractor::unzip_to(&mut zipped_epub, format!("{} [ao3 {}].epub", title, &args.id)).unwrap();
}
