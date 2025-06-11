use std::{collections::HashSet, env, fs, io::Write, path::PathBuf, process, sync::{Arc, Mutex}};

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
    works_file: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logger = Box::leak(Box::new(MutexLogger { m: Mutex::new(KdamLogger::new()) }));
    let max_level = logger.m.lock().unwrap().filter();
    log::set_logger(logger)?;
    log::set_max_level(max_level);

    let args = Cli::parse();

    let work_ids = fs::read_to_string(args.works_file)
        .context("Cannot read works file")?
        .lines()
        .filter_map(|line| line.parse::<usize>().ok())
        .collect::<HashSet<_>>();

    log::info!("Detected {} works", work_ids.len());

    if work_ids.is_empty() {
        log::info!("Exiting early since there is nothing to download");
        process::exit(0);
    }

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

    let pb = kdam::Bar::builder()
        .total(work_ids.len())
        .unit("work")
        .inverse_unit(true)
        .build()
        .map(Mutex::new)
        .map(Arc::new)
        .unwrap(); // Only has a potential to error when bar_format is set
    logger.m.lock().unwrap().set_pb(pb.clone());

    let mut failed_work_ids = HashSet::<usize>::new();

    for id in work_ids {
        let res = download_work(&client, &id)
            .await
            .with_context(|| { format!("Cannot download work with ID {}", &id) });

        if let Err(e) = res {
            log::warn!("Cannot download work with ID {}", &id);
            log::warn!("Error: {}", e);
            failed_work_ids.insert(id);
        };

        pb.lock().unwrap().update(1)?;
    }

    logger.m.lock().unwrap().remove_pb();

    if !failed_work_ids.is_empty() {
        log::warn!("Failed to download a total of {} work(s)", failed_work_ids.len());

        fs::write("failed-works.txt",
            failed_work_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join("\n"))
            .context("Cannot write list of works that failed to download to failed-works.txt")?;

        log::info!("IDs of failing-to-download works written to failed-works.txt");
    }

    Ok(())
}

async fn download_work(client: &reqwest::Client, work_id: &usize) -> anyhow::Result<()> {
    log::debug!("Attempting to download work with ID {}", work_id);

    let bytes = ao3::download(&client, &work_id)
        .await
        .context("Could not download data")?;

    log::info!("Successfully downloaded work with ID {}", work_id);

    log::debug!("Attempting to parse download as ZIP");

    let mut zipped_epub = extractor::as_zip(bytes)
        .context("Could not parse download as ZIP (this may happen for hidden works)")?;

    log::info!("Successfully parsed download as ZIP");

    log::debug!("Attempting to extract title of work with ID {}", work_id);

    let file_path = match extractor::title(&mut zipped_epub) {
        Ok(title) => {
            log::info!("Extracted title '{}' for work with ID {}", &title, work_id);
            format!("{} [ao3 {}].epub", title, work_id)
        },
        Err(e) => {
            log::warn!("Could not extract title for fic with ID {}", work_id);
            log::warn!("Error: {}", e);
            format!("[ao3 {}].epub", work_id)
        },
    };

    log::debug!("Extracting work to path '{}'", &file_path);

    extractor::unzip_to(&mut zipped_epub, &file_path)
        .context("Could not unzip EPUB")?;

    log::info!("Successfully extracted work to path '{}'", &file_path);

    Ok(())
}
