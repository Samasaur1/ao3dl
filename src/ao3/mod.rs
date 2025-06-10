use core::time;

use anyhow::bail;
use reqwest::{multipart, Client, Request, Response, StatusCode};

mod types;

static AO3DL_USER_AGENT: &'static str = "ao3dl/0.1.0";

static AUTHENTICITY_TOKEN_URL: &'static str = "https://archiveofourown.org/token_dispenser.json";
static LOGIN_URL: &'static str = "https://archiveofourown.org/users/login";

async fn execute_with_retries(client: &Client, req: impl Fn() -> anyhow::Result<Request>) -> anyhow::Result<Response> {
    const DELAY_SCALING_FACTOR: f64 = 1.25;
    const DELAY_BASE: f64 = 1.0;
    let mut exponential_delay = DELAY_BASE;

    loop {
        println!("attempting request");
        let possible_response = client.execute(req()?)
            .await;

        match possible_response {
            Ok(resp) => {
                let code = resp.status();
                if code.is_success() {
                    println!("Got OK");
                    return Ok(resp);
                } else if code == StatusCode::TOO_MANY_REQUESTS {
                    println!("Got HTTP 429");
                    match resp.headers().get(reqwest::header::RETRY_AFTER).map(|x| x.to_str()) {
                        Some(Ok(val)) => {
                            if let Ok(delay) = val.parse::<u64>() {
                                // This is ao3's case
                                println!("Sleeping {} secs", delay);
                                tokio::time::sleep(time::Duration::from_secs(delay)).await;
                                exponential_delay = DELAY_BASE;
                                continue;
                            } else {
                                // Technically this header can also be a date
                                bail!("Retry-After header had unparseable value {}", val);
                            }
                        },
                        _ => {
                            bail!("HTTP 429 Too Many Requests without Retry-After header");
                        }
                    }
                } else if code.is_server_error() {
                    println!("got server error, sleeping {} secs", exponential_delay * DELAY_SCALING_FACTOR);
                    exponential_delay *= DELAY_SCALING_FACTOR;
                    tokio::time::sleep(time::Duration::from_secs_f64(exponential_delay)).await;
                    continue;
                } else {
                    bail!("Unhandled HTTP code {} ({:?})", code.as_str(), code.canonical_reason());
                }
            },
            Err(e) => {
                bail!("Got error {:?}", e);
            },
        };
    }
}

async fn get_authenticity_token(client: &Client) -> anyhow::Result<String> {
    let token = execute_with_retries(client, || {
        let req = client.get(AUTHENTICITY_TOKEN_URL).build()?;
        Ok(req)
    })
        .await?
        .json::<types::AuthenticityToken>()
        .await?
        .token;

    println!("got token: {}", token);

    Ok(token)
}

pub async fn login(client: &Client, username: &str, password: &str) -> anyhow::Result<()> {
    let user = username.to_owned();
    let pass = password.to_owned();

    println!("logging in using {}:{}", user, pass);

    let token = get_authenticity_token(client)
        .await?;

    println!("got token {}", token);

    let response = execute_with_retries(client, || {
        let form = multipart::Form::new()
            .text("user[login]", user.clone())
            .text("user[password]", pass.clone())
            .text("user[remember_me]", 1.to_string())
            .text("authenticity_token", token.clone());

        let req = client.post(LOGIN_URL)
            .multipart(form)
            .build()?;

        Ok(req)
    }).await?;

    let logged_in = response
        .text()
        .await
        .unwrap()
        .contains(r#"href="/users/logout""#);

    println!("logged in: {}", logged_in);

    if !logged_in {
        bail!("not logged in");
    }

    Ok(())
}

fn compute_download_url(id: &usize) -> String {
    format!("https://archiveofourown.org/downloads/{}/x.epub", id)
}

pub async fn download(client: &Client, id: &usize) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
    let download_url = compute_download_url(id);

    let bytes = execute_with_retries(client, || {
        let req = client.get(download_url.clone()).build()?;
        Ok(req)
    })
        .await?
        .bytes()
        .await?;

    Ok(bytes)
}

pub fn make_client() -> Result<Client, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent(AO3DL_USER_AGENT)
        .cookie_store(true)
        .build()?;

    Ok(client)
}
