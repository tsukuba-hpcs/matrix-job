use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{bail, Context as _};
use clap::Parser;
use matrix_job::conv::{convert_matrix, to_liquid_model};

#[derive(clap::Subcommand)]
enum SubCommand {
    List,
    Render { job: String },
    Run { job: String },
}

#[derive(clap::Parser)]
struct Opts {
    #[clap(subcommand)]
    cmd: SubCommand,
    #[clap(short, long, env)]
    config: Option<PathBuf>,
}

type Config = HashMap<String, matrix_job::Job>;

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
    let config = serde_yaml::from_str(&fs::read_to_string(&path)?)?;
    if let Some(parent) = path.parent() {
        if parent.as_os_str() != "" {
            std::env::set_current_dir(parent)?;
        }
    }
    Ok(config)
}

fn main() -> anyhow::Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();
    let opts = Opts::parse();
    let config = load_config(opts.config.clone())?;
    match opts.cmd {
        SubCommand::List => {
            for (job_name, job) in &config {
                let job_count = matrix_job::matrix(&job.matrix).len();
                println!("{job_name}: {job_count}");
            }
        }
        SubCommand::Render { job: job_name } => {
            let job = config
                .get(&job_name)
                .with_context(|| format!("job {job_name} not found"))?;
            let matrix = matrix_job::matrix(&job.matrix);
            let matrix = convert_matrix(matrix, to_liquid_model, &job.filter)?;
            matrix_job::render(&matrix, &job.templates)?;
        }
        SubCommand::Run { job: job_name } => {
            let job = config
                .get(&job_name)
                .with_context(|| format!("job {job_name} not found"))?;
            let matrix = matrix_job::matrix(&job.matrix);
            let matrix = convert_matrix(matrix, to_liquid_model, &job.filter)?;
            matrix_job::render(&matrix, &job.templates)?;
            matrix_job::execute(&matrix, &job.commands)?;
        }
    }
    Ok(())
}
