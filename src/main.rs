use anyhow::{bail, Context, Result};
use clap::Parser;
use gpx::{Gpx, GpxVersion, Track, TrackSegment, Waypoint};
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

fn make_http_request(url: &str) -> Result<String> {
    let response = ureq::get(url)
        .set("User-Agent", "Mozilla/5.0")
        .set("Accept", "text/html,application/xhtml+xml")
        .set("Accept-Language", "en-US,en")
        .call();

    match response {
        Ok(res) => Ok(res.into_string()?),
        Err(e) => bail!("HTTP Request failed: {:?}", e),
    }
}

fn parse_komoot_html(html: &str) -> Result<Vec<Waypoint>> {
    let start_marker = "kmtBoot.setProps(\"";
    let end_marker = "\");";
    let start = html.find(start_marker).context("Start marker not found")? + start_marker.len();
    let end = html[start..]
        .find(end_marker)
        .context("End marker not found")?
        + start;
    let json_str = unescape::unescape(&html[start..end]).context("Cannot unescape JSON")?;
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

fn main() -> Result<()> {
    let args = Args::parse();

    let response = make_http_request(&args.url)?;
    let coords = parse_komoot_html(&response)?;
    let gpx = make_gpx(&coords);
    write_gpx(&gpx, &args.output.unwrap_or_else(|| "-".to_string()))?;

    Ok(())
}
