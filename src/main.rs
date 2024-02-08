use anyhow::Context;
use regex::Regex;
use reqwest::header::{HeaderMap, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use serde_json::Value;
use std::env;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: program <URL>");
        return Ok(());
    }
    let url = &args[1];

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/<version> Safari/537.36".parse().unwrap());
    headers.insert(
        ACCEPT,
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8"
            .parse()
            .unwrap(),
    );
    headers.insert(ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());

    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await?
        .text()
        .await?;

    let regex = Regex::new(r#"kmtBoot\.setProps\("(.+?)"\)"#).unwrap();
    let json_str = regex
        .captures(&response)
        .and_then(|cap| cap.get(1)).context("Cannot find kmtBoot.setProps in HTML")?
        .as_str();

    let json_str = unescape::unescape(json_str).context("Cannot unescape JSON")?;

    let json: Value = serde_json::from_str(&json_str)?;

    let coords = &json["page"]["_embedded"]["tour"]["_embedded"]["coordinates"]["items"];

    println!("{:?}", coords);

    Ok(())
}
