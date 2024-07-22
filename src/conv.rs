use std::{collections::HashMap, str::FromStr};

use anyhow::{bail, Context};
use itertools::Itertools;
use liquid::model::KString;

use crate::ExpandedMatrix;

pub fn to_json_model(value: serde_yaml::Value) -> anyhow::Result<serde_json::Value> {
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
                .map(to_json_model)
                .collect::<Result<_, _>>()?;
            serde_json::Value::Array(seq)
        }
        serde_yaml::Value::Mapping(map) => {
            let map = map
                .into_iter()
                .map(|(k, v)| {
                    let v = to_json_model(v)?;
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

pub fn to_liquid_model(value: serde_yaml::Value) -> anyhow::Result<liquid::model::Value> {
    Ok(match value {
        serde_yaml::Value::Null => liquid::model::Value::Nil,
        serde_yaml::Value::Bool(b) => liquid::model::Value::scalar(b),
        serde_yaml::Value::Number(n) => liquid::model::Value::Scalar(if let Some(n) = n.as_u64() {
            liquid::model::to_scalar(&n)?
        } else if let Some(n) = n.as_i64() {
            liquid::model::to_scalar(&n)?
        } else if let Some(n) = n.as_f64() {
            liquid::model::to_scalar(&n)?
        } else {
            bail!("invalid number")
        }),
        serde_yaml::Value::String(s) => liquid::model::Value::scalar(s),
        serde_yaml::Value::Sequence(seq) => {
            let seq = seq
                .into_iter()
                .map(to_liquid_model)
                .collect::<Result<_, _>>()?;
            liquid::model::Value::Array(seq)
        }
        serde_yaml::Value::Mapping(map) => {
            let map = map
                .into_iter()
                .map(|(k, v)| {
                    let k = k.as_str().with_context(|| "object key must be string")?;
                    let k = KString::from_str(k)?;
                    let v = to_liquid_model(v)?;
                    Ok::<_, anyhow::Error>((k, v))
                })
                .collect::<Result<Vec<_>, _>>()?;
            liquid::model::Value::Object(liquid::Object::from_iter(map))
        }
        serde_yaml::Value::Tagged(_) => bail!("tag is not supported"),
    })
}

fn convert_map<F, T>(
    mapping: HashMap<String, serde_yaml::Value>,
    convert: F,
) -> anyhow::Result<HashMap<String, T>>
where
    F: Fn(serde_yaml::Value) -> anyhow::Result<T>,
{
    mapping
        .into_iter()
        .map(|(k, v)| {
            let v = convert(v)?;
            Ok::<_, anyhow::Error>((k, v))
        })
        .collect()
}

pub fn convert_matrix<F, T>(
    matrix: ExpandedMatrix<serde_yaml::Value>,
    convert: F,
    filter: &Option<String>,
) -> anyhow::Result<ExpandedMatrix<T>>
where
    F: Fn(serde_yaml::Value) -> anyhow::Result<T>,
{
    let matrix = matrix
        .into_iter()
        .map(|env| {
            if let Some(filter) = filter {
                let json_model = env
                    .clone()
                    .into_iter()
                    .map(|(k, v)| to_json_model(v).map(|v| (k, v)));
                let json_model = serde_json::Value::Object(json_model.collect::<Result<_, _>>()?);
                if zen_expression::evaluate_expression(filter, &json_model)?
                    == serde_json::Value::Bool(true)
                {
                    Ok(Some(convert_map(env, &convert)?))
                } else {
                    Ok(None)
                }
            } else {
                Ok(Some(convert_map(env, &convert)?))
            }
        })
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect_vec();
    Ok(matrix)
}
