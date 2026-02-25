use std::{collections::HashSet, env, fs, io::{IsTerminal, Write}, path::PathBuf, process};

use anyhow::Context;
use clap::Parser;
use regex::Regex;

use crate::ao3::WorkId;

mod ao3;
mod extractor;

#[derive(Parser)]
struct Cli {
    works_file: PathBuf,
}

struct ProgressBar {
    // https://github.com/ghostty-org/ghostty/pull/7975
    // https://conemu.github.io/en/AnsiEscapeCodes.html#ConEmu_specific_OSC
    // https://github.com/ghostty-org/ghostty/pull/8477
    // https://doc.rust-lang.org/std/io/trait.IsTerminal.html
    isatty: bool,
    current: usize,
    max: usize,
}

impl ProgressBar {
    fn new(max: usize) -> ProgressBar {
        ProgressBar {
            isatty: std::io::stdout().is_terminal(),
            current: 0,
            max: max
        }
    }

    fn begin(&mut self) {
        self.current = 0;
        self.write_pb(true, 0);
    }

    fn next(&mut self) {
        self.current += 1;
        let percent = 100 * self.current / self.max;
        self.write_pb(true, percent);
    }

    fn end(&mut self) {
        self.write_pb(false, 0);
        self.isatty = false;
    }

    fn write_pb(&mut self, going: bool, pct: usize) {
        if !self.isatty { return }
        let buf = if going {
            format!("\x1b]9;4;1;{}\x07", pct)
        } else {
            "\x1b]9;4;0\x07".to_string()
        };
        if std::io::stdout().write(buf.as_bytes()).is_err() {
            self.isatty = false;
            return
        }
        if std::io::stdout().flush().is_err() {
            self.isatty = false;
            return
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.end();
    }
}

struct IndeterminateProgressBar {
    isatty: bool,
}

impl IndeterminateProgressBar {
    fn new() -> IndeterminateProgressBar {
        return IndeterminateProgressBar { isatty: std::io::stdout().is_terminal() }
    }

    fn begin(&mut self) {
        self.write_pb(true);
    }

    fn end(&mut self) {
        self.write_pb(false);
        self.isatty = false;
    }

    fn write_pb(&mut self, going: bool) {
        if !self.isatty { return }
        let buf = if going {
            "\x1b]9;4;3\x07"
        } else {
            "\x1b]9;4;0\x07"
        };
        if std::io::stdout().write(buf.as_bytes()).is_err() {
            self.isatty = false;
            return
        }
        if std::io::stdout().flush().is_err() {
            self.isatty = false;
            return
        }
    }
}

impl Drop for IndeterminateProgressBar {
    fn drop(&mut self) {
        self.end();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let args = Cli::parse();

    let _work_regex = Regex::new(r"https://archiveofourown\.org/works/(\d+)").unwrap();
    let raw_work_ids = fs::read_to_string(args.works_file)
        .context("Cannot read works file")?
        .lines()
        .filter_map(|line| {
            if let Ok(work_id) = serde_json::from_str(line){
                Some(work_id)
            } else if let Ok(id) = line.parse::<usize>() {
                Some(ao3::WorkId::Bare(id))
            } else if let Some(captures) = _work_regex.captures(line) {
                captures.get(1)?
                    .as_str()
                    .parse::<usize>()
                    .ok()
                    .map(|id| ao3::WorkId::Bare(id))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    log::trace!("Detected {} works", raw_work_ids.len());

    let (with_timestamps, without_timestamps): (Vec<WorkId>, Vec<WorkId>) = raw_work_ids.iter().partition(|work_id| {
        match work_id {
            ao3::WorkId::Bare(_) => false,
            ao3::WorkId::WithTimestamp { id: _, timestamp: _ } => true,
        }
    });

    log::trace!("Detected {} work(s) with timestamps and {} work(s) without timestamps", with_timestamps.len(), without_timestamps.len());

    let mut matched_ids = HashSet::<usize>::new();
    let mut work_ids = Vec::<ao3::WorkId>::new();
    for work in with_timestamps.iter().chain(without_timestamps.iter()) {
        if matched_ids.insert(*work.id()) {
            // New to the set, add to the final list
            work_ids.push(*work);
        } else {
            // Already in the set, skip
            log::trace!("Found duplicate ID {}", work.id());
            continue;
        }
    }

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

    let mut pb = IndeterminateProgressBar::new();

    pb.begin();
    ao3::login(&client, &username, &password)
        .await
        .context("Could not log in. Check your username/password")?;
    pb.end();

    log::info!("Successfully logged in");

    let mut pb = ProgressBar::new(work_ids.len());

    let mut failed_work_ids = HashSet::<usize>::new();

    pb.begin();
    for work in work_ids {
        let res = download_work(&client, &work)
            .await
            .with_context(|| { format!("Cannot download work with ID {}", &work.id()) });

        if let Err(e) = res {
            let msg = e.chain()
                .map(|link| {
                    link.to_string()
                })
                .collect::<Vec<String>>()
                .join(", because ");
            log::warn!("{}", msg);
            failed_work_ids.insert(*work.id());
        };

        pb.next();
    }
    pb.end();

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

async fn download_work(client: &reqwest::Client, work: &ao3::WorkId) -> anyhow::Result<()> {
    log::debug!("Attempting to download work with ID {}", work.id());

    let bytes = ao3::download(&client, &work)
        .await
        .context("Could not download data")?;

    log::info!("Successfully downloaded work with ID {}", work.id());

    log::debug!("Attempting to parse download as ZIP");

    let mut zipped_epub = extractor::as_zip(bytes)
        .context("Could not parse download as ZIP (this may happen for hidden works)")?;

    log::info!("Successfully parsed download as ZIP");

    log::debug!("Attempting to extract title of work with ID {}", work.id());

    let mut file_path = match extractor::title(&mut zipped_epub) {
        Ok(title) => {
            log::info!("Extracted title '{}' for work with ID {}", &title, work.id());
            format!("{} [ao3 {}].epub", title, work.id())
        },
        Err(e) => {
            let msg = e.chain()
                .map(|link| {
                    link.to_string()
                })
                .collect::<Vec<String>>()
                .join(", because ");
            log::warn!("Could not extract title for fic with ID {}, because {}", work.id(), msg);
            format!("[ao3 {}].epub", work.id())
        },
    };

    let presanitized_len = file_path.len();
    file_path.retain(|c| c != '\0' && c != '/');
    let sanitized_len = file_path.len();
    if sanitized_len < presanitized_len {
        log::info!("Sanitizing destination file path");
    }
    let file_path = file_path; // make non-mut

    log::debug!("Extracting work to path '{}'", &file_path);

    extractor::unzip_to(&mut zipped_epub, &file_path)
        .context("Could not unzip EPUB")?;

    log::info!("Successfully extracted work to path '{}'", &file_path);

    Ok(())
}
