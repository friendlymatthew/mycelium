use crate::schema::GpsPoint;
use anyhow::{Context, Result};
use datafusion::arrow::array::{
    Array, Float32Array, Float64Array, Int64Array, RecordBatch, StringViewArray, StringViewBuilder,
};
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::functions_aggregate::expr_fn::count;
use datafusion::prelude::*;
use std::path::Path;
use std::sync::Arc;

pub struct GpsStore {
    ctx: SessionContext,
    table_name: String,
}

impl GpsStore {
    pub fn new() -> Self {
        let ctx = SessionContext::new();
        Self {
            ctx,
            table_name: "gps_points".to_string(),
        }
    }

    pub async fn with_parquet<P: AsRef<Path>>(parquet_path: P) -> Result<Self> {
        let ctx = SessionContext::new();
        let table_name = "gps_points".to_string();

        let parquet_path = parquet_path.as_ref();
        if parquet_path.exists() {
            ctx.register_parquet(
                &table_name,
                parquet_path.to_str().unwrap(),
                Default::default(),
            )
            .await?;
        }

        Ok(Self { ctx, table_name })
    }

    pub async fn insert(&mut self, points: Vec<GpsPoint>) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }

        let batch = Self::points_to_record_batch(&points)?;
        let to_insert = self.ctx.read_batch(batch)?;

        if !self.ctx.table_exist(&self.table_name)? {
            self.ctx
                .register_table(&self.table_name, to_insert.into_view())?;
            return Ok(());
        }

        let existing = self.ctx.table(&self.table_name).await?;
        let combined = existing.union(to_insert)?;
        self.ctx.deregister_table(&self.table_name)?;
        self.ctx
            .register_table(&self.table_name, combined.into_view())?;

        Ok(())
    }

    pub async fn save_to_parquet<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let df = self.ctx.table(&self.table_name).await?;

        df.write_parquet(
            path.as_ref().to_str().unwrap(),
            DataFrameWriteOptions::new(),
            None,
        )
        .await
        .context("Failed to write parquet file")?;

        Ok(())
    }

    pub async fn count(&self) -> Result<usize> {
        if !self.ctx.table_exist(&self.table_name)? {
            return Ok(0);
        }

        let df = self.ctx.table(&self.table_name).await?;
        let count_df = df.aggregate(vec![], vec![count(col("timestamp"))])?;
        let batches = count_df.collect().await?;

        if let Some(batch) = batches.first() {
            if let Some(count_array) = batch.column(0).as_any().downcast_ref::<Int64Array>() {
                if !count_array.is_empty() {
                    return Ok(count_array.value(0) as usize);
                }
            }
        }

        Ok(0)
    }

    pub async fn query_bbox(
        &self,
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
    ) -> Result<Vec<GpsPoint>> {
        let df = self.ctx.table(&self.table_name).await?;

        let filtered = df.filter(
            col("latitude")
                .gt_eq(lit(min_lat))
                .and(col("latitude").lt_eq(lit(max_lat)))
                .and(col("longitude").gt_eq(lit(min_lon)))
                .and(col("longitude").lt_eq(lit(max_lon))),
        )?;

        let batches = filtered.collect().await?;
        Self::record_batches_to_points(&batches)
    }

    pub async fn query_by_activity(&self, activity_id: &str) -> Result<Vec<GpsPoint>> {
        let df = self.ctx.table(&self.table_name).await?;
        let filtered = df.filter(col("activity_id").eq(lit(activity_id)))?;
        let batches = filtered.collect().await?;
        Self::record_batches_to_points(&batches)
    }

    pub async fn query_all(&self) -> Result<Vec<GpsPoint>> {
        let df = self.ctx.table(&self.table_name).await?;
        let batches = df.collect().await?;
        Self::record_batches_to_points(&batches)
    }

    pub async fn get_activity_ids(&self) -> Result<Vec<String>> {
        if !self.ctx.table_exist(&self.table_name)? {
            return Ok(Vec::new());
        }

        let df = self.ctx.table(&self.table_name).await?;
        let distinct = df.select(vec![col("activity_id")])?.distinct()?;
        let batches = distinct.collect().await?;

        let mut activity_ids = Vec::new();
        for batch in batches {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringViewArray>()
                .context("Invalid activity_id column")?;

            for i in 0..batch.num_rows() {
                activity_ids.push(ids.value(i).to_string());
            }
        }

        Ok(activity_ids)
    }

    fn points_to_record_batch(points: &[GpsPoint]) -> Result<RecordBatch> {
        let schema: SchemaRef = GpsPoint::arrow_schema();

        let timestamps: Int64Array = points.iter().map(|p| p.timestamp).collect();
        let latitudes: Float64Array = points.iter().map(|p| p.latitude).collect();
        let longitudes: Float64Array = points.iter().map(|p| p.longitude).collect();
        let altitudes: Float32Array = points.iter().map(|p| p.altitude).collect();

        let activity_ids =
            StringViewArray::from_iter(points.iter().map(|p| Some(p.activity_id.as_str())));

        let mut activity_types = StringViewBuilder::with_capacity(points.len());
        for point in points {
            if let Some(ref t) = point.activity_type {
                activity_types.append_value(t);
            } else {
                activity_types.append_null();
            }
        }
        let activity_types = activity_types.finish();

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(timestamps),
                Arc::new(latitudes),
                Arc::new(longitudes),
                Arc::new(altitudes),
                Arc::new(activity_ids),
                Arc::new(activity_types),
            ],
        )
        .context("Failed to create RecordBatch")
    }

    fn record_batches_to_points(batches: &[RecordBatch]) -> Result<Vec<GpsPoint>> {
        let mut points = Vec::new();

        for batch in batches {
            let timestamps = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid timestamp column")?;

            let latitudes = batch
                .column(1)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid latitude column")?;

            let longitudes = batch
                .column(2)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid longitude column")?;

            let altitudes = batch
                .column(3)
                .as_any()
                .downcast_ref::<Float32Array>()
                .context("Invalid altitude column")?;

            let activity_ids = batch
                .column(4)
                .as_any()
                .downcast_ref::<StringViewArray>()
                .context("Invalid activity_id column")?;

            let activity_types = batch
                .column(5)
                .as_any()
                .downcast_ref::<StringViewArray>()
                .context("Invalid activity_type column")?;

            for i in 0..batch.num_rows() {
                points.push(GpsPoint {
                    timestamp: timestamps.value(i),
                    latitude: latitudes.value(i),
                    longitude: longitudes.value(i),
                    altitude: if altitudes.is_null(i) {
                        None
                    } else {
                        Some(altitudes.value(i))
                    },
                    activity_id: activity_ids.value(i).to_string(),
                    activity_type: if activity_types.is_null(i) {
                        None
                    } else {
                        Some(activity_types.value(i).to_string())
                    },
                });
            }
        }

        Ok(points)
    }
}

impl Default for GpsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FitIngester;

    #[tokio::test]
    async fn test_store_ingest_and_query() {
        let points = FitIngester::ingest_fit_file("../example.fit", "example".to_string())
            .expect("Failed to ingest FIT file");

        println!("Ingested {} GPS points", points.len());

        let mut store = GpsStore::new();
        store.insert(points).await.expect("Failed to insert points");

        let count = store.count().await.expect("Failed to count points");
        println!("Store contains {} points", count);
        assert!(count > 0, "Store should contain points");

        let activity_points = store
            .query_by_activity("example")
            .await
            .expect("Failed to query by activity");
        println!(
            "Found {} points for activity 'example'",
            activity_points.len()
        );
        assert_eq!(activity_points.len(), count);

        let bbox_points = store
            .query_bbox(40.7, 41.1, -74.0, -73.9)
            .await
            .expect("Failed to query bbox");
        println!("Found {} points in NYC bbox", bbox_points.len());
        assert!(bbox_points.len() > 0, "Should find points in NYC area");
    }

    #[tokio::test]
    async fn test_store_parquet_persistence() {
        let points = FitIngester::ingest_fit_file("../example.fit", "example".to_string())
            .expect("Failed to ingest FIT file");

        let mut store = GpsStore::new();
        store
            .insert(points.clone())
            .await
            .expect("Failed to insert");

        let temp_path = "../data/test_gps.parquet";
        store
            .save_to_parquet(temp_path)
            .await
            .expect("Failed to save parquet");

        let loaded_store = GpsStore::with_parquet(temp_path)
            .await
            .expect("Failed to load from parquet");

        let loaded_count = loaded_store.count().await.expect("Failed to count");
        println!("Loaded {} points from parquet", loaded_count);
        assert_eq!(loaded_count, points.len());

        std::fs::remove_file(temp_path).ok();
    }
}
