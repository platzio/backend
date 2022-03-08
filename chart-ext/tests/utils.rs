use std::path::PathBuf;

pub fn chart_dir(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("charts")
        .join(relative)
}
