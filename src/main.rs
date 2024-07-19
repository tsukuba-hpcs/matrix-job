use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{bail, Context};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    matrix: HashMap<String, Vec<serde_yaml::Value>>,
    command: Option<String>,
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

fn extend(
    base: Vec<HashMap<String, serde_json::Value>>,
    var_name: &str,
    vars: &[Value],
) -> Vec<HashMap<String, serde_json::Value>> {
    base.into_iter()
        .flat_map(|base| {
            vars.into_iter()
                .map(|var| {
                    let mut obj = base.clone();
                    obj.insert(var_name.to_owned(), var.clone());
                    obj
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn yamlmodel_to_jsonmodel(value: serde_yaml::Value) -> anyhow::Result<serde_json::Value> {
    Ok(match value {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            let num = if let Some(n) = n.as_u64() {
                serde_json::Number::from(n)
            } else if let Some(n) = n.as_i64() {
                serde_json::Number::from(n)
            } else if let Some(n) = n.as_f64() {
                serde_json::Number::from_f64(n).with_context(|| "invalid floating point number")?
            } else {
                bail!("invalid number")
            };
            serde_json::Value::Number(num)
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            let seq = seq
                .into_iter()
                .map(yamlmodel_to_jsonmodel)
                .collect::<Result<_, _>>()?;
            serde_json::Value::Array(seq)
        }
        serde_yaml::Value::Mapping(map) => {
            let map = map
                .into_iter()
                .map(|(k, v)| {
                    let v = yamlmodel_to_jsonmodel(v)?;
                    let k = k
                        .as_str()
                        .with_context(|| "object key must be string")?
                        .to_owned();
                    Ok::<_, anyhow::Error>((k, v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?;
            serde_json::Value::Object(map)
        }
        serde_yaml::Value::Tagged(_) => bail!("tags are unsupported"),
    })
}

fn calc_matrix(job: &Job) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
    let mut elements = vec![HashMap::default()];
    for (var_name, vars) in &job.matrix {
        let vars = vars
            .into_iter()
            .map(|yaml| yamlmodel_to_jsonmodel(yaml.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        elements = extend(elements, var_name, &vars);
    }
    Ok(elements)
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let config = load_config(opts.config.clone())?;
    match opts.cmd {
        SubCommand::List => {
            for (job_name, job) in &config {
                let job_count = calc_matrix(job)?.len();
                println!("{job_name}: {job_count}");
            }
        }
    }
    Ok(())
}
