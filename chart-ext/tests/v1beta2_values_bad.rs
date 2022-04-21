mod fake_db;
mod utils;

use anyhow::Result;
use utils::load_chart;

#[tokio::test]
async fn test() -> Result<()> {
    let chart_ext = load_chart("v1beta2/chart2").await?;
    println!("{:?}", chart_ext);
    assert!(matches!(chart_ext.values_ui, None));
    Ok(())
}
