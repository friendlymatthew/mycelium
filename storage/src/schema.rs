use datafusion::arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GpsPoint {
    pub timestamp: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f32>,
    pub activity_id: String,
    pub activity_type: Option<String>,
}

impl GpsPoint {
    pub fn arrow_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("timestamp", DataType::Int64, false),
            Field::new("latitude", DataType::Float64, false),
            Field::new("longitude", DataType::Float64, false),
            Field::new("altitude", DataType::Float32, true),
            Field::new("activity_id", DataType::Utf8View, false),
            Field::new("activity_type", DataType::Utf8View, true),
        ]))
    }
}
