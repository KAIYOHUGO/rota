mod config;
mod iio;
mod libinput;
mod runtime;

use std::env::args;

use anyhow::{Context, Result};
use config::Config;
use runtime::Runtime;
use tokio::fs::read_to_string;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let config_path = args()
        .nth(1)
        .context("Require config file path: rota {path}")?;

    let buf = read_to_string(&config_path).await?;

    let con: Config = knus::parse(&config_path, &buf)?;

    log::debug!("Load config : {:#?}", &con);

    let run = Runtime::new(con)?;

    run.run().await?;

    Ok(())
}
