use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::bail;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(clap::Subcommand)]
enum SubCommand {
    List,
}

#[derive(clap::Parser)]
struct Opts {
    #[clap(subcommand)]
    cmd: SubCommand,
    #[clap(short, long, env)]
    config: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct Template {
    source: PathBuf,
    outpath: String,
}

#[derive(Serialize, Deserialize)]
struct Job {
    #[serde(default)]
    matrix: HashMap<String, serde_yaml::Value>,
    command: String,
    #[serde(default)]
    templates: Vec<Template>,
}

type Config = HashMap<String, Job>;

fn load_config(path: Option<PathBuf>) -> anyhow::Result<Config> {
    let path = if let Some(path) = path {
        path
    } else if PathBuf::from("matrix-job.yml").try_exists()? {
        PathBuf::from("matrix-job.yml")
    } else if PathBuf::from("matrix-job.yaml").try_exists()? {
        PathBuf::from("matrix-job.yaml")
    } else {
        bail!("Config not found");
    };
    let config = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    Ok(config)
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let config = load_config(opts.config.clone())?;
    match opts.cmd {
        SubCommand::List => {
            for job in config.keys() {
                println!("{job}");
            }
        }
    }
    Ok(())
}
