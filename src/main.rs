use anyhow::{bail, Context, Result};
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

enum Output {
    Path(String),
    Stdout,
}

fn make_http_request(url: &str) -> Result<String> {
    ureq::get(url)
        .set("User-Agent", "komootgpx")
        .call()
        .with_context(|| format!("HTTP request to {} failed", url))?
        .into_string()
        .map_err(anyhow::Error::from)
}

fn extract_json_from_html(html: String) -> Result<serde_json::Value> {
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

    if let Some(coords_array) = coords.as_array() {
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
            .collect::<Result<Vec<Waypoint>>>()?;

        let segment = TrackSegment { points: waypoints };

        let track = Track {
            name: Some(tour_name),
            segments: vec![segment],
            ..Default::default()
        };

        Ok(track)
    } else {
        bail!("Coordinates are not an array")
    }
}

fn write_gpx(track: Track, output: Output) -> Result<()> {
    let gpx = Gpx {
        version: GpxVersion::Gpx11,
        creator: Some("komootgpx".to_string()),
        tracks: vec![track],
        ..Default::default()
    };

    let buf: Box<dyn Write> = match output {
        Output::Path(file_name) => {
            let file = File::create(file_name)?;
            Box::new(BufWriter::new(file))
        }
        Output::Stdout => Box::new(BufWriter::new(std::io::stdout())),
    };

    gpx::write(&gpx, buf)?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output = match args.output.as_deref() {
        Some("-") | None => Output::Stdout,
        Some(file_name) => Output::Path(file_name.to_string()),
    };

    let response = make_http_request(&args.url)?;
    let json = extract_json_from_html(response)?;
    let track = json_to_track(json)?;

    write_gpx(track, output)?;

    Ok(())
}
