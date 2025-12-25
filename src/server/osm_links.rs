use anyhow::anyhow;
use dvb::point::Point;
use proj::Proj;

use crate::server::args::DVBPointCoords;

/// Convert DVB projected coordinates to WGS84 latitude/longitude
///
/// DVB uses EPSG:31468 (DHDN / Gauss-Krüger zone 4) projection.
/// The coords tuple appears to be (northing, easting) based on observed values.
///
/// Returns: (latitude, longitude) in WGS84 decimal degrees
fn dvb_coords_to_wgs84(coords: (i64, i64)) -> Result<(f64, f64), String> {
    // DVB coords appear to be (northing, easting) in EPSG:31468 (Gauss-Krüger Zone 4)
    let (northing, easting) = coords;

    // Create projection from EPSG:31468 (Gauss-Krüger Zone 4) to EPSG:4326 (WGS84)
    let proj = Proj::new_known_crs("EPSG:31468", "EPSG:4326", None)
        .map_err(|e| format!("Failed to create projection: {}", e))?;

    // Convert to f64 and transform
    // proj expects (x, y) which in UTM is (easting, northing)
    let result = proj
        .convert((easting as f64, northing as f64))
        .map_err(|e| format!("Failed to transform coordinates: {}", e))?;

    // Result is (longitude, latitude) in WGS84
    let (longitude, latitude) = result;

    // Basic validation for WGS84 coordinate ranges
    if !((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)) {
        return Err(format!(
            "Converted coordinates ({}, {}) are outside valid WGS84 ranges",
            latitude, longitude
        ));
    }

    Ok((latitude, longitude))
}

#[derive(Debug)]
pub struct OsmCoords {
    pub latitude: f64,
    pub longitude: f64,
}

impl OsmCoords {
    pub fn url(self) -> String {
        let OsmCoords {
            latitude,
            longitude,
        } = self;

        format!("https://www.openstreetmap.org/?mlat={latitude}&mlon={longitude}&zoom=17")
    }
}

impl TryFrom<DVBPointCoords> for OsmCoords {
    type Error = anyhow::Error;

    fn try_from(coords: DVBPointCoords) -> Result<Self, Self::Error> {
        let (latitude, longitude) = dvb_coords_to_wgs84((coords.latitude, coords.longitude))
            .map_err(|error| anyhow!("Failed to converting coordinates: {error}"))?;

        Ok(OsmCoords {
            latitude,
            longitude,
        })
    }
}

impl TryFrom<Point> for OsmCoords {
    type Error = anyhow::Error;

    fn try_from(Point { coords, name, .. }: Point) -> anyhow::Result<Self> {
        let (latitude, longitude) = dvb_coords_to_wgs84(coords)
            .map_err(|error| anyhow!("{name}: Failed to converting coordinates: {error}"))?;

        Ok(OsmCoords {
            latitude,
            longitude,
        })
    }
}

#[test]
fn test_dvb_coords_conversion() {
    // Dresden Hauptbahnhof coords from DVB: (5657516, 4621644)
    // Expected WGS84: approximately 51.040° N, 13.732° E
    let coords = (5657516, 4621644);
    let result = dvb_coords_to_wgs84(coords);

    if let Err(e) = &result {
        eprintln!("Conversion error: {}", e);
    }
    assert!(result.is_ok(), "Conversion should succeed: {:?}", result);
    let (lat, lon) = result.unwrap();

    println!("Converted coords: lat={:.6}, lon={:.6}", lat, lon);

    // Check it's in the Dresden area
    assert!(
        (50.0..=52.0).contains(&lat),
        "Latitude should be in Dresden area"
    );
    assert!(
        (13.0..=15.0).contains(&lon),
        "Longitude should be in Dresden area"
    );

    // More specific check for Hauptbahnhof area
    assert!(
        (51.03..=51.05).contains(&lat),
        "Should be near Dresden Hbf latitude"
    );
    assert!(
        (13.72..=13.74).contains(&lon),
        "Should be near Dresden Hbf longitude"
    );
}
