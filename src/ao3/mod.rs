use core::time;

use anyhow::{Context, bail};
use reqwest::{Client, Request, Response, StatusCode, multipart};

pub use types::WorkId;

mod types;

static AO3DL_USER_AGENT: &'static str = "ao3dl/1.1.2";

static AUTHENTICITY_TOKEN_URL: &'static str = "https://archiveofourown.org/token_dispenser.json";
static LOGIN_URL: &'static str = "https://archiveofourown.org/users/login";

async fn execute_with_retries(
    client: &Client,
    build_req: impl Fn() -> anyhow::Result<Request>,
) -> anyhow::Result<Response> {
    const DELAY_SCALING_FACTOR: f64 = 1.25;
    const DELAY_BASE: f64 = 1.0;
    let mut exponential_delay = DELAY_BASE;

    loop {
        if exponential_delay > 64.0 {
            log::error!(target: "ao3dl::ao3::retrier", "Retried too many times; giving up");
            bail!("Retried too many times (hit delay limit of 64s)");
        }

        log::trace!(target: "ao3dl::ao3::retrier", "Building request");
        let req = build_req().context("Cannot (re)build request to (re)try it")?;
        log::trace!(target: "ao3dl::ao3::retrier", "Attempting request");
        let possible_response = client.execute(req).await;

        match possible_response {
            Ok(resp) => {
                let code = resp.status();
                if code.is_success() {
                    log::trace!(target: "ao3dl::ao3::retrier", "Got successful response to request");
                    return Ok(resp);
                } else if code == StatusCode::TOO_MANY_REQUESTS {
                    log::debug!(target: "ao3dl::ao3::retrier", "Got HTTP 429");
                    match resp
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .map(|x| x.to_str())
                    {
                        Some(Ok(val)) => {
                            if let Ok(delay) = val.parse::<u64>() {
                                // This is ao3's case
                                log::info!(target: "ao3dl::ao3::retrier", "Sleeping {} secs", delay);
                                tokio::time::sleep(time::Duration::from_secs(delay)).await;
                                exponential_delay = DELAY_BASE;
                                continue;
                            } else {
                                // Technically this header can also be a date
                                bail!("Retry-After header had unparseable value {}", val);
                            }
                        }
                        _ => {
                            bail!("HTTP 429 Too Many Requests without Retry-After header");
                        }
                    }
                } else if code.is_server_error() {
                    exponential_delay *= DELAY_SCALING_FACTOR;
                    log::trace!(target: "ao3dl::ao3::retrier", "got server error ({}), sleeping {} secs", code.as_str(), exponential_delay);
                    tokio::time::sleep(time::Duration::from_secs_f64(exponential_delay)).await;
                    continue;
                } else {
                    bail!(
                        "Unhandled HTTP code {} ({:?})",
                        code.as_str(),
                        code.canonical_reason()
                    );
                }
            }
            Err(e) => {
                bail!("Got error {:?}", e);
            }
        };
    }
}

async fn get_authenticity_token(client: &Client) -> anyhow::Result<String> {
    let req_builder = || {
        let req = client
            .get(AUTHENTICITY_TOKEN_URL)
            .build()
            .context("Cannot build authenticity token URL")?;
        Ok(req)
    };
    let token = execute_with_retries(client, req_builder)
        .await
        .context("Could not fetch authenticity token")?
        .json::<types::AuthenticityToken>()
        .await
        .context("Could not parse authenticity token from response")?
        .token;

    Ok(token)
}

pub async fn login(client: &Client, username: &str, password: &str) -> anyhow::Result<()> {
    let user = username.to_owned();
    let pass = password.to_owned();

    log::info!("Attempting to login as {}", user);

    log::trace!("Attempting to fetch authenticity token");

    let token = get_authenticity_token(client)
        .await
        .context("Cannot fetch authenticity token")?;

    log::trace!("Got authenticity token: {}", token);

    log::trace!("Making login request");

    let req_builder = || {
        let form = multipart::Form::new()
            .text("user[login]", user.clone())
            .text("user[password]", pass.clone())
            .text("user[remember_me]", 1.to_string())
            .text("authenticity_token", token.clone());

        let req = client
            .post(LOGIN_URL)
            .multipart(form)
            .build()
            .context("Cannot build login request")?;

        Ok(req)
    };
    let response = execute_with_retries(client, req_builder)
        .await
        .context("Cannot make login request")?;

    log::trace!("Successfully made login request");

    let logged_in = response
        .text()
        .await
        .context("Cannot get body of response to login request as text")?
        .contains(r#"href="/users/logout"#);

    if logged_in {
        log::info!("Successfully logged in");
    } else {
        bail!("Could not log in (check your username/password)");
    }

    Ok(())
}

async fn compute_download_url(client: &Client, work: &WorkId) -> anyhow::Result<String> {
    log::trace!("Computing download URL for work with ID {}", &work.id());

    let download_path = match work {
        WorkId::Bare(id) => {
            log::trace!("Fetching work page to determine updated_at timestamp for work");
            let work_url = format!("https://archiveofourown.org/works/{}", id);

            let req_builder = || {
                let req = client
                    .get(work_url.clone())
                    .build()
                    .context("Cannot build work request")?;
                Ok(req)
            };

            let regex = regex::Regex::new(
                format!(
                    r#"<a href="(/downloads/{}/.+\.epub\?updated_at=\d+)">EPUB</a>"#,
                    id
                )
                .as_str(),
            )
            .context("Cannot create regex!")?;

            let work_html = execute_with_retries(client, req_builder)
                .await
                .with_context(|| format!("Cannot fetch main work page for ID {}", id))?
                .text()
                .await
                .context("Work body not convertible to string")?;

            let download_path = regex
                .captures(&work_html)
                .context("Cannot find EPUB download URL in work HTML")?
                .get(1)
                .unwrap()
                .as_str();

            download_path.to_string()
        }
        WorkId::WithTimestamp { id, timestamp } => {
            log::trace!(
                "Work comes annotated with timestamp {}; short-circuiting",
                timestamp
            );
            format!("/downloads/{}/x.epub?updated_at={}", id, timestamp)
        }
    };

    log::trace!(
        "Computed download URL for work with ID {} as {}",
        &work.id(),
        download_path
    );

    Ok(format!("https://archiveofourown.org{}", download_path))
}

pub async fn download(client: &Client, work: &WorkId) -> anyhow::Result<bytes::Bytes> {
    log::trace!("Attempting to download work with ID {}", &work.id());

    let download_url = compute_download_url(&client, work)
        .await
        .with_context(|| format!("Cannot determine download URL for ID {}", work.id()))?;

    let req_builder = || {
        let req = client
            .get(download_url.clone())
            .build()
            .context("Cannot build download request")?;
        Ok(req)
    };
    let bytes = execute_with_retries(client, req_builder)
        .await
        .with_context(|| format!("Cannot download work with ID {}", work.id()))?
        .bytes()
        .await
        .context("Cannot get body of response to download request as bytes")?;

    log::trace!("Successfully downloaded work with ID {}", &work.id());

    Ok(bytes)
}

pub fn make_client() -> anyhow::Result<Client> {
    let client = Client::builder()
        .user_agent(AO3DL_USER_AGENT)
        .cookie_store(true)
        .build()
        .context("Cannot build client")?;

    Ok(client)
}
