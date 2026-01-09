use storage::GpsStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading parquet file...");
    let store = GpsStore::with_parquet("data/gps_data.parquet").await?;

    let count = store.count().await?;
    println!("Total points: {}", count);

    let activity_ids = store.get_activity_ids().await?;
    println!("Found {} activities:", activity_ids.len());
    for (idx, id) in activity_ids.iter().enumerate() {
        let points = store.query_by_activity(id).await?;
        println!("  Activity {}: {} ({} points)", idx + 1, id, points.len());
    }

    Ok(())
}
