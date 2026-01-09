use storage::{FitIngester, GpsStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Ingesting GPS data from data/raw directory...");

    let points = FitIngester::ingest_directory("data/raw")?;
    println!("Ingested {} GPS points from all activities", points.len());

    let mut store = GpsStore::new();
    store.insert(points).await?;

    let output_path = "data/gps_data.parquet";
    store.save_to_parquet(output_path).await?;
    println!("Saved GPS data to {}", output_path);

    let count = store.count().await?;
    println!("Total points in store: {}", count);

    Ok(())
}
