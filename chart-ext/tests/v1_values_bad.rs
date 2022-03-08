mod fake_db;
mod utils;

use anyhow::Result;
use platz_chart_ext::ChartExt;
use utils::chart_dir;

#[tokio::test]
async fn test() -> Result<()> {
    let chart_ext = ChartExt::from_path(&chart_dir("v1/chart2")).await?;
    println!("{:?}", chart_ext);
    assert!(matches!(chart_ext.values_ui, None));
    Ok(())
}
