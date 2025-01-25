use anyhow::Result;
use platz_db::{Db, DbTable, HelmChart, HelmChartFilters, HelmChartTagInfo, HelmTagFormat};
use regex::Regex;
use tracing::{debug, info, warn};

pub async fn run(db: &Db) -> Result<()> {
    info!("Starting tag parser task");
    let mut db_rx = db.subscribe_to_events();

    update_all_charts().await?;

    loop {
        let event = db_rx.recv().await?;

        if event.table == DbTable::HelmTagFormats {
            update_all_charts().await?;
        }
    }
}

struct CompiledTagFormats(Vec<(Regex, HelmTagFormat)>);

impl CompiledTagFormats {
    async fn new() -> Result<Self> {
        let tag_formats = HelmTagFormat::all().await?;
        Ok(Self(
            tag_formats
                .into_iter()
                .map(|tag_format| Ok((Regex::new(&tag_format.pattern)?, tag_format)))
                .collect::<Result<_>>()?,
        ))
    }

    fn test(&self, tag: &str) -> HelmChartTagInfo {
        for (regex, tag_format) in self.0.iter() {
            if let Some(captures) = regex.captures(tag) {
                return HelmChartTagInfo {
                    tag_format_id: Some(tag_format.id),
                    parsed_version: Some(
                        captures
                            .name("version")
                            .map(|mat| mat.as_str().to_owned())
                            .unwrap_or_default(),
                    ),
                    parsed_revision: Some(
                        captures
                            .name("revision")
                            .map(|mat| mat.as_str().to_owned())
                            .unwrap_or_default(),
                    ),
                    parsed_branch: Some(
                        captures
                            .name("branch")
                            .map(|mat| mat.as_str().to_owned())
                            .unwrap_or_default(),
                    ),
                    parsed_commit: Some(
                        captures
                            .name("commit")
                            .map(|mat| mat.as_str().to_owned())
                            .unwrap_or_default(),
                    ),
                };
            }
        }

        Default::default()
    }
}

async fn update_all_charts() -> Result<()> {
    let formats = CompiledTagFormats::new().await?;

    if formats.0.is_empty() {
        warn!("No tag formats found, not updating charts");
        return Ok(());
    }

    info!(
        "Testing all charts for changes against {} formats",
        formats.0.len()
    );

    for page in 1.. {
        let helm_charts = get_charts_page(page).await?;
        if helm_charts.is_empty() {
            break;
        }
        for helm_chart in helm_charts {
            debug!("Testing chart {}", helm_chart.id);
            let current = formats.test(&helm_chart.image_tag);
            if helm_chart.tag_format_id != current.tag_format_id
                && helm_chart.parsed_version != current.parsed_version
                && helm_chart.parsed_revision != current.parsed_revision
                && helm_chart.parsed_branch != current.parsed_branch
                && helm_chart.parsed_commit != current.parsed_commit
            {
                warn!(
                    "Chart {} has new parsed tag info: {:?}",
                    helm_chart.id, current
                );
                current.save(helm_chart.id).await?;
            }
        }
    }

    Ok(())
}

async fn get_charts_page(page: i64) -> Result<Vec<HelmChart>> {
    let result = HelmChart::all_filtered(
        HelmChartFilters {
            helm_registry_id: None,
            parsed_branch: None,
            page: Some(page),
            per_page: Some(50),
        },
        Default::default(),
    )
    .await?;
    debug!(
        "Fetched Helm charts page {}/{}",
        page,
        result.num_total / result.per_page.max(1)
    );
    Ok(result.items)
}

pub async fn parse_image_tag(image_tag: &str) -> Result<HelmChartTagInfo> {
    let formats = CompiledTagFormats::new().await?;
    info!("Testing {} formats for tag: {}", formats.0.len(), image_tag);
    Ok(formats.test(image_tag))
}
