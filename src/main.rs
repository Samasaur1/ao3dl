use std::{env, io::Write, path::PathBuf, process, sync::{Arc, Mutex}};

use anyhow::Context;
use clap::Parser;
use env_logger::builder;
use kdam::{tqdm, BarExt};

mod ao3;
mod extractor;

struct PbWriter {
    pb: Arc<Mutex<kdam::Bar>>,
}

impl std::io::Write for PbWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut pb = self.pb.lock().unwrap();
        pb.clear()?;
        pb.writer.print(b"\r")?;
        pb.writer.print(buf)?;
        // pb.writer.print(b"\n")?;
        pb.refresh()?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct KdamLogger {
    logger: pretty_env_logger::env_logger::Logger
}

impl KdamLogger {
    fn new() -> KdamLogger {
        Self {
            logger: Self::logger_builder().build(),
        }
    }

    fn set_pb(&mut self, pb: Arc<Mutex<kdam::Bar>>) {
        let writer = PbWriter {
            pb: pb
        };
        self.logger = Self::logger_builder()
            .target(pretty_env_logger::env_logger::Target::Pipe(Box::new(writer)))
            .build();
    }

    fn remove_pb(&mut self) {
        self.logger = Self::logger_builder().build();
    }

    fn filter(&self) -> log::LevelFilter {
        self.logger.filter()
    }

    fn logger_builder() -> pretty_env_logger::env_logger::Builder {
        let mut builder = pretty_env_logger::formatted_builder();
        
        if let Ok(s) = ::std::env::var("RUST_LOG") {
            builder.parse_filters(&s);
        }

        builder
    }
}

impl log::Log for KdamLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.logger.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.logger.log(record)
    }

    fn flush(&self) {
        self.logger.flush()
    }
}

struct MutexLogger {
    m: Mutex<KdamLogger>
}

impl log::Log for MutexLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.m.lock().unwrap().enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.m.lock().unwrap().log(record)
    }

    fn flush(&self) {
        self.m.lock().unwrap().flush()
    }
}

#[derive(Parser)]
struct Cli {
    // works: PathBuf,
    id: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logger = Box::leak(Box::new(MutexLogger { m: Mutex::new(KdamLogger::new()) }));
    let max_level = logger.m.lock().unwrap().filter();
    log::set_logger(logger)?;
    log::set_max_level(max_level);

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

    let pb = Arc::new(Mutex::new(tqdm!(total = 10)));
    logger.m.lock().unwrap().set_pb(pb.clone());

    for i in 0..10 {
        std::thread::sleep(std::time::Duration::from_secs_f32(0.1));

        pb.lock().unwrap().update(1)?;
        log::info!("i: {}", i);
    }

    logger.m.lock().unwrap().remove_pb();

    log::info!("Done with pb");

    process::exit(0);

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
