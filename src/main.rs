use anyhow::{bail, Context, Result};
use clap::Parser;
use gpx::{Gpx, GpxVersion, Track, TrackSegment, Waypoint};
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
    let response = ureq::get(url).set("User-Agent", "komootgpx").call();

    match response {
        Ok(res) => Ok(res.into_string()?),
        Err(e) => bail!("HTTP Request failed: {:?}", e),
    }
}

fn parse_komoot_html(html: String) -> Result<Vec<Waypoint>> {
    let start_marker = "kmtBoot.setProps(\"";
    let end_marker = "\");";
    let start = html.find(start_marker).context("Start marker not found")? + start_marker.len();
    let end = html[start..]
        .find(end_marker)
        .context("End marker not found")?
        + start;

    let json_str = unescape::unescape(&html[start..end]).context("Cannot unescape JSON")?;
    let json: serde_json::Value = serde_json::from_str(&json_str)?;

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

        Ok(waypoints)
    } else {
        bail!("Coordinates are not an array")
    }
}

fn make_gpx(waypoints: Vec<Waypoint>) -> Gpx {
    let segment = TrackSegment { points: waypoints };

    let track = Track {
        segments: vec![segment],
        ..Default::default()
    };

    Gpx {
        version: GpxVersion::Gpx11,
        creator: Some("komootgpx".to_string()),
        tracks: vec![track],
        ..Default::default()
    }
}

fn write_gpx(gpx: &Gpx, output: &str) -> Result<()> {
    let buf: Box<dyn Write> = if output == "-" {
        Box::new(BufWriter::new(std::io::stdout()))
    } else {
        let file = std::fs::File::create(output)?;
        Box::new(BufWriter::new(file))
    };

    gpx::write(gpx, buf)?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let response = make_http_request(&args.url)?;
    let coords = parse_komoot_html(response)?;
    let gpx = make_gpx(coords);
    write_gpx(&gpx, &args.output.unwrap_or_else(|| "-".to_string()))?;

    Ok(())
}
