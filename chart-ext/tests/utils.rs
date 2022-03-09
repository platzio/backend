use anyhow::Result;
use platz_chart_ext::ChartExt;
use std::path::PathBuf;

fn chart_dir(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("charts")
        .join(relative_path)
}

pub async fn load_chart(relative_path: &str) -> Result<ChartExt> {
    let chart_ext = ChartExt::from_path(&chart_dir(relative_path)).await?;
    println!("{:#?}", chart_ext);
    Ok(chart_ext)
}
