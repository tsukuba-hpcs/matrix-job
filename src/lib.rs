use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

use liquid::{model::KString, Object};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub type ExpandedMatrix<T> = Vec<HashMap<String, T>>;
pub type MatrixDefinition = HashMap<String, Vec<serde_yaml::Value>>;

pub mod conv;

const LIQUID_PARSER: Lazy<liquid::Parser> =
    Lazy::new(|| liquid::ParserBuilder::with_stdlib().build().unwrap());

#[derive(Serialize, Deserialize, Debug)]
pub struct Template {
    pub source: PathBuf,
    pub outpath: String,
}

#[derive(Serialize, Deserialize)]
pub struct Job {
    #[serde(default)]
    pub matrix: MatrixDefinition,
    pub command: Option<String>,
    #[serde(default)]
    pub templates: Vec<Template>,
}

fn extend(
    base: Vec<HashMap<String, serde_yaml::Value>>,
    var_name: &str,
    vars: &[serde_yaml::Value],
) -> Vec<HashMap<String, serde_yaml::Value>> {
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

pub fn matrix(matrix: &MatrixDefinition) -> ExpandedMatrix<serde_yaml::Value> {
    let mut elements = vec![HashMap::default()];
    for (var_name, vars) in matrix {
        elements = extend(elements, var_name, &vars);
    }
    elements
}

pub fn render_template(
    matrix: &ExpandedMatrix<liquid::model::Value>,
    template: &Template,
) -> anyhow::Result<()> {
    dbg!(template);
    let src = fs::read_to_string(&template.source)?;
    for env in matrix {
        let env = env
            .into_iter()
            .map(|(k, v)| {
                let k = KString::from_str(k)?;
                let v = v.clone();
                Ok::<_, anyhow::Error>((k, v))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let env = Object::from_iter(env);
        let contents = LIQUID_PARSER.parse(&src)?.render(&env)?;
        let path = LIQUID_PARSER.parse(&template.outpath)?.render(&env)?;
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    Ok(())
}

pub fn render(
    matrix: &ExpandedMatrix<liquid::model::Value>,
    templates: &[Template],
) -> anyhow::Result<()> {
    for template in templates {
        render_template(matrix, template)?;
    }
    Ok(())
}
