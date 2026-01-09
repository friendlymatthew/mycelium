use crate::schema::GpsPoint;
use anyhow::{Context, Result};
use fitparser::profile::MesgNum;
use std::path::Path;
use uuid::Uuid;

pub struct FitIngester;

pub struct GpsPointIter {
    data: Vec<fitparser::FitDataRecord>,
    activity_id: String,
    activity_type: Option<String>,
    current_index: usize,
}

impl Iterator for GpsPointIter {
    type Item = GpsPoint;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_index < self.data.len() {
            let record = &self.data[self.current_index];
            self.current_index += 1;

            if record.kind() == MesgNum::Record {
                let fields = record.fields();

                let timestamp =
                    fields
                        .iter()
                        .find(|f| f.name() == "timestamp")
                        .and_then(|f| match f.value() {
                            fitparser::Value::Timestamp(ts) => Some(ts.timestamp()),
                            _ => None,
                        });

                let latitude = fields
                    .iter()
                    .find(|f| f.name() == "position_lat")
                    .and_then(|f| match f.value() {
                        fitparser::Value::SInt32(val) => {
                            Some((*val) as f64 * (180.0 / 2_147_483_648.0))
                        }
                        _ => None,
                    });

                let longitude = fields
                    .iter()
                    .find(|f| f.name() == "position_long")
                    .and_then(|f| match f.value() {
                        fitparser::Value::SInt32(val) => {
                            Some((*val) as f64 * (180.0 / 2_147_483_648.0))
                        }
                        _ => None,
                    });

                let altitude = fields
                    .iter()
                    .find(|f| f.name() == "altitude" || f.name() == "enhanced_altitude")
                    .and_then(|f| match f.value() {
                        fitparser::Value::Float64(val) => Some((*val) as f32),
                        fitparser::Value::Float32(val) => Some(*val),
                        fitparser::Value::UInt16(val) => Some((*val) as f32),
                        fitparser::Value::UInt32(val) => Some((*val) as f32),
                        fitparser::Value::SInt16(val) => Some((*val) as f32),
                        fitparser::Value::SInt32(val) => Some((*val) as f32),
                        _ => {
                            // debug: log unmatched altitude types
                            eprintln!("unmatched altitude type: {:?}", f.value());
                            None
                        }
                    });

                if let (Some(ts), Some(lat), Some(lon)) = (timestamp, latitude, longitude) {
                    return Some(GpsPoint {
                        timestamp: ts,
                        latitude: lat,
                        longitude: lon,
                        altitude,
                        activity_id: self.activity_id.clone(),
                        activity_type: self.activity_type.clone(),
                    });
                }
            }
        }

        None
    }
}

impl FitIngester {
    pub fn stream_fit_file<P: AsRef<Path>>(path: P, activity_id: String) -> Result<GpsPointIter> {
        let path = path.as_ref();

        let mut fp = std::fs::File::open(path)
            .with_context(|| format!("failed to open FIT file: {}", path.display()))?;

        let data = fitparser::from_reader(&mut fp)
            .with_context(|| format!("failed to parse FIT file: {}", path.display()))?;

        let activity_type = data
            .iter()
            .find(|r| r.kind() == MesgNum::Session)
            .and_then(|record| {
                record
                    .fields()
                    .iter()
                    .find(|f| f.name() == "sport")
                    .map(|sport| format!("{:?}", sport.value()))
            });

        Ok(GpsPointIter {
            data,
            activity_id,
            activity_type,
            current_index: 0,
        })
    }

    pub fn ingest_fit_file<P: AsRef<Path>>(path: P, activity_id: String) -> Result<Vec<GpsPoint>> {
        Ok(Self::stream_fit_file(path, activity_id)?.collect())
    }

    pub fn ingest_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<GpsPoint>> {
        let dir = dir.as_ref();
        let mut all_points = Vec::new();

        let mut num_files = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("fit") {
                let activity_id = Uuid::new_v4().to_string();

                let filename = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                println!("Ingesting {} with activity_id: {}", filename, activity_id);

                match Self::ingest_fit_file(&path, activity_id) {
                    Ok(mut points) => {
                        all_points.append(&mut points);
                        num_files += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to ingest {:?}: {}", path, e);
                    }
                }
            }
        }

        println!("ingested {num_files} files");

        Ok(all_points)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_ingest_example_fit() {
        let out = FitIngester::ingest_fit_file("../example.fit", "example".to_string()).unwrap();

        assert_eq!(out.len(), 4773);

        // note: maybe it's better to impl Hash and assert via fingerprint...
        insta::assert_debug_snapshot!(out[..2], @r#"
        [
            GpsPoint {
                timestamp: 1757856886,
                latitude: 40.757603980600834,
                longitude: -73.99709788151085,
                altitude: Some(
                    9.6,
                ),
                activity_id: "example",
                activity_type: Some(
                    "String(\"cycling\")",
                ),
            },
            GpsPoint {
                timestamp: 1757856888,
                latitude: 40.757715962827206,
                longitude: -73.99703769944608,
                altitude: Some(
                    8.8,
                ),
                activity_id: "example",
                activity_type: Some(
                    "String(\"cycling\")",
                ),
            },
        ]
        "#);
        insta::assert_debug_snapshot!(out[out.len()-2..], @r#"
        [
            GpsPoint {
                timestamp: 1757876281,
                latitude: 40.75702688656747,
                longitude: -73.99820856750011,
                altitude: Some(
                    20.4,
                ),
                activity_id: "example",
                activity_type: Some(
                    "String(\"cycling\")",
                ),
            },
            GpsPoint {
                timestamp: 1757876285,
                latitude: 40.75701196677983,
                longitude: -73.99816498160362,
                altitude: Some(
                    20.6,
                ),
                activity_id: "example",
                activity_type: Some(
                    "String(\"cycling\")",
                ),
            },
        ]
        "#);
    }
}
