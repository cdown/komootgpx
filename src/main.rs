use anyhow::{Context, Result};
use clap::Parser;
use gpx::{Gpx, GpxVersion, Track, TrackSegment, Waypoint};
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// The Komoot URL to make a GPX for
    url: String,

    /// The GPX file to create. By default (or on "-") print to stdout
    #[clap(short, long)]
    output: Option<String>,
}

fn make_http_request(url: &str) -> Result<String> {
    ureq::get(url)
        .header("User-Agent", "komootgpx")
        .call()
        .with_context(|| format!("HTTP request to {} failed", url))?
        .body_mut()
        .read_to_string()
        .map_err(anyhow::Error::from)
}

fn extract_json_from_html(html: &str) -> Result<serde_json::Value> {
    let start_marker = "kmtBoot.setProps(\"";
    let end_marker = "\");";
    let start = html.find(start_marker).context("Start marker not found")? + start_marker.len();
    let end = html[start..]
        .find(end_marker)
        .context("End marker not found")?
        + start;

    let json_str = unescape::unescape(&html[start..end]).context("Cannot unescape JSON")?;
    serde_json::from_str(&json_str).map_err(anyhow::Error::from)
}

fn json_to_track(json: serde_json::Value) -> Result<Track> {
    let tour_name = json["page"]["_embedded"]["tour"]["name"]
        .as_str()
        .context("Tour name not found")?
        .to_string();

    let coords = &json["page"]["_embedded"]["tour"]["_embedded"]["coordinates"]["items"];

    let coords_array = coords.as_array().context("Coordinates are not an array")?;

    fn get_coord(coord: &serde_json::Value, key: &str) -> Result<f64> {
        coord[key]
            .as_f64()
            .context(format!("{key} is not a valid f64"))
    }
    let waypoints = coords_array
        .iter()
        .map(|coord| {
            let lat = get_coord(coord, "lat")?;
            let lng = get_coord(coord, "lng")?;
            let alt = get_coord(coord, "alt")?;

            let mut waypoint = Waypoint::new(geo_types::Point::new(lng, lat));
            waypoint.elevation = Some(alt);
            Ok(waypoint)
        })
        .collect::<Result<Vec<_>>>()?;

    let segment = TrackSegment { points: waypoints };

    let track = Track {
        name: Some(tour_name),
        segments: vec![segment],
        ..Default::default()
    };

    Ok(track)
}

fn write_gpx<W: Write>(track: Track, mut writer: W) -> Result<()> {
    let gpx = Gpx {
        version: GpxVersion::Gpx11,
        creator: Some("komootgpx".to_string()),
        tracks: vec![track],
        ..Default::default()
    };

    gpx::write(&gpx, &mut writer)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let track = {
        let response = make_http_request(&args.url)?;
        let json = extract_json_from_html(&response)?;
        json_to_track(json)?
    };

    match args.output.as_deref() {
        Some("-") | None => write_gpx(track, BufWriter::new(std::io::stdout()))?,
        Some(file_name) => {
            let file = File::create(file_name)
                .with_context(|| format!("Failed to create file: {file_name}"))?;
            write_gpx(track, BufWriter::new(file))?
        }
    };

    Ok(())
}
