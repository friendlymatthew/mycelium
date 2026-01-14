use std::fs;
use storage::GpsStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading parquet data...");
    let parquet_path = "data/gps_data.parquet";

    if !std::path::Path::new(parquet_path).exists() {
        eprintln!("Error: Parquet file not found at {}", parquet_path);
        eprintln!("Run 'cargo run -p storage' first to generate it.");
        return Ok(());
    }

    let store = GpsStore::with_parquet(parquet_path).await?;

    println!("Getting activity IDs...");
    let activity_ids = store.get_activity_ids().await?;
    println!("Found {} activities", activity_ids.len());

    let mut all_activities = Vec::new();

    for (idx, activity_id) in activity_ids.iter().enumerate() {
        let gps_points = store.query_by_activity(activity_id).await?;
        println!(
            "Activity {} [{}]: {} points",
            idx + 1,
            activity_id,
            gps_points.len()
        );

        let activity = gps_points
            .iter()
            .map(|point| {
                [
                    point.latitude,
                    point.longitude,
                    point.altitude.unwrap_or(0.0) as f64,
                ]
            })
            .collect::<Vec<_>>();

        all_activities.push(activity);
    }

    println!("\nExporting to JSON...");
    let json = serde_json::to_string_pretty(&all_activities)?;

    let output_path = "web/public/gps-data.json";
    fs::write(output_path, json)?;

    let total_points = all_activities.iter().map(|a| a.len()).sum::<usize>();
    println!(
        "Exported {} activities with {} total points",
        all_activities.len(),
        total_points
    );
    println!("Saved to {}", output_path);

    Ok(())
}
