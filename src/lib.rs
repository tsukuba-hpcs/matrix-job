use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

use liquid::model::KString;
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
    #[serde(default)]
    pub squash: bool,
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
    let src = fs::read_to_string(&template.source)?;
    let globals = matrix
        .into_iter()
        .map(|object| {
            let object = object
                .into_iter()
                .map(|(k, v)| {
                    let k = KString::from_str(k)?;
                    Ok((k, v.clone()))
                })
                .collect::<Result<HashMap<_, _>, anyhow::Error>>()?;
            Ok::<_, anyhow::Error>(liquid::model::Object::from_iter(object))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let contents_template = LIQUID_PARSER.parse(&src)?;
    let path_tempalte = LIQUID_PARSER.parse(&template.outpath)?;
    if template.squash {
        let globals = globals
            .into_iter()
            .map(|obj| liquid::model::Value::Object(obj))
            .collect();
        let globals = liquid::model::Value::Array(globals);
        let global = liquid::object!({
            "squash": globals,
        });
        let contents = contents_template.render(&global)?;
        let path = path_tempalte.render(&global)?;
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    } else {
        for global in globals {
            let contents = contents_template.render(&global)?;
            let path = path_tempalte.render(&global)?;
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, contents)?;
        }
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
