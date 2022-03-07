use super::creds::rusoto_client;
use anyhow::{anyhow, Result};
pub use rusoto_core::region::{ParseRegionError, Region};
use rusoto_ec2::{Ec2, Ec2Client};
use std::env;
use std::str::FromStr;

pub async fn get_regions() -> Result<Vec<Region>> {
    let client = rusoto_client(env!("CARGO_PKG_NAME").into())?;
    let ec2 = Ec2Client::new_with_client(client, Default::default());

    let ec2_regions = match ec2.describe_regions(Default::default()).await?.regions {
        None => return Err(anyhow!("Got an empty region list")),
        Some(regions) => regions,
    };

    Ok(ec2_regions
        .into_iter()
        .filter_map(|region| region.region_name)
        .map(|region_name| Region::from_str(&region_name))
        .map(Result::from)
        .collect::<std::result::Result<Vec<_>, ParseRegionError>>()?)
}
