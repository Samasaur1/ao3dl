use reqwest::{multipart, Client};

mod types;

static AO3DL_USER_AGENT: &'static str = "ao3dl/0.1.0";

static AUTHENTICITY_TOKEN_URL: &'static str = "https://archiveofourown.org/token_dispenser.json";
static LOGIN_URL: &'static str = "https://archiveofourown.org/users/login";

async fn get_authenticity_token(client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    let resp = client.get(AUTHENTICITY_TOKEN_URL)
        .send()
        .await?;

    println!("token status code: {}", resp.status());

    let token = resp
        .json::<types::AuthenticityToken>()
        .await?
        .token;

    Ok(token)
}

pub async fn login(client: &Client, username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let user = username.to_owned();
    let pass = password.to_owned();

    println!("logging in using {}:{}", user, pass);

    let token = get_authenticity_token(client)
        .await?;

    println!("got token {}", token);

    let form = multipart::Form::new()
        .text("user[login]", user)
        .text("user[password]", pass)
        .text("user[remember_me]", 1.to_string())
        .text("authenticity_token", token);

    let response = client
        .post(LOGIN_URL)
        .multipart(form)
        .send()
        .await
        .unwrap();

    let logged_in = response
        .text()
        .await
        .unwrap()
        .contains(r#"href="/users/logout""#);

    println!("logged in: {}", logged_in);

    Ok(())
}

fn compute_download_url(id: &usize) -> String {
    format!("https://archiveofourown.org/downloads/{}/x.epub", id)
}

pub async fn download(client: &Client, id: &usize) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
    let download_url = compute_download_url(id);

    let bytes = client.get(download_url)
        .send()
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
