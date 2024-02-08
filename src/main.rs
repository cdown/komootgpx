use anyhow::{bail, Context, Result};
use clap::Parser;
use gpx::{Gpx, GpxVersion, Track, TrackSegment, Waypoint};
use regex::Regex;
use reqwest::header::{HeaderMap, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use serde_json::Value;
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The Komoot URL to make a GPX for
    url: String,

    /// The GPX file to create. By default (or on "-") print to stdout
    #[clap(short, long)]
    output: Option<String>,
}

async fn make_http_request(url: &str) -> Result<String> {
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

    Ok(response)
}

fn parse_komoot_html(html: &str) -> Result<Vec<Waypoint>> {
    let regex = Regex::new(r#"kmtBoot\.setProps\("(.+?)"\)"#).unwrap();
    let json_str = regex
        .captures(html)
        .and_then(|cap| cap.get(1))
        .context("Cannot find kmtBoot.setProps in HTML")?
        .as_str();

    let json_str = unescape::unescape(json_str).context("Cannot unescape JSON")?;
    let json: Value = serde_json::from_str(&json_str)?;

    let coords = &json["page"]["_embedded"]["tour"]["_embedded"]["coordinates"]["items"];

    if let Some(coords_array) = coords.as_array() {
        let waypoints = coords_array
            .iter()
            .map(|coord| {
                let lat = coord["lat"]
                    .as_f64()
                    .context("Latitude is not a valid f64")?;
                let lng = coord["lng"]
                    .as_f64()
                    .context("Longitude is not a valid f64")?;
                let alt = coord["alt"]
                    .as_f64()
                    .context("Altitude is not a valid f64")?;

                let mut waypoint = Waypoint::new(geo_types::Point::new(lng, lat));
                waypoint.elevation = Some(alt);
                Ok(waypoint)
            })
            .collect::<Result<Vec<Waypoint>>>()?;

        Ok(waypoints)
    } else {
        bail!("Coordinates are not an array")
    }
}

fn make_gpx(waypoints: &[Waypoint]) -> Gpx {
    let mut track = Track::new();
    let segment = TrackSegment {
        points: waypoints.to_vec(),
    };

    track.segments = vec![segment];

    Gpx {
        version: GpxVersion::Gpx11,
        creator: Some("komootgpx".to_string()),
        metadata: None,
        waypoints: vec![],
        tracks: vec![track],
        routes: vec![],
    }
}

fn write_gpx(gpx: &Gpx, output: &str) -> Result<()> {
    let buf: Box<dyn Write> = if output == "-" {
        Box::new(BufWriter::new(std::io::stdout()))
    } else {
        let file = File::create(output)?;
        Box::new(BufWriter::new(file))
    };

    gpx::write(gpx, buf)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let response = make_http_request(&args.url).await?;
    let coords = parse_komoot_html(&response)?;
    let gpx = make_gpx(&coords);
    write_gpx(&gpx, &args.output.unwrap_or_else(|| "-".to_string()))?;

    Ok(())
}
