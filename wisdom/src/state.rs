use indent::indent_by;
use serde::Deserialize;

use crate::{DynamicBindString, DynamicCallbackString, GameReadError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    id: String,
    data: StateData
}

impl State {
    pub fn js_register(&self) -> Result<String, GameReadError> {
        let data_obj = indent_by(4, self.data.as_json_pairs()?);

        let id = &self.id;
        let compiled_initial_value = self.data.initial_value.compile(crate::SetData::None)?;
        // We can use unwrap here because the "" key is guaranteed to exist since the compilation passed
        let initial_value = compiled_initial_value.get("").unwrap();

        Ok(indoc::formatdoc! {r#"
            slf.states.{id} = State.create({initial_value}, {{
                {data_obj}
            }})
        "#}.trim_end().to_string())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStateData {
    initial_value: DynamicBindString,
    validate: Option<String>,
}

impl From<RawStateData> for StateData {
    fn from(value: RawStateData) -> Self {
        let validate = if let Some(validate) = value.validate {
            Some(DynamicCallbackString::from(validate)
                .with_params(vec!["newState".to_string()])
            )
        } else {
            None
        };

        Self {
            initial_value: value.initial_value,
            validate,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "RawStateData")]
pub struct StateData {
    initial_value: DynamicBindString,
    validate: Option<DynamicCallbackString>,
}

impl StateData {
    fn compile_callback_data(dsc: &DynamicCallbackString, name: &str) -> Result<String, GameReadError> {
        let value = dsc.compile("")?;

        Ok(format!("{name}: {value}"))
    }

    fn compile_bind_data(dsc: &DynamicBindString, name: &str) -> Result<String, GameReadError> {
        let mut compiled = dsc.compile(crate::SetData::None)?;
        if compiled.len() > 1 {
            return Err(GameReadError::State(
                format!("States cannot depend on other states through subscriptions ('{name}')")
            ))
        }

        // We can use unwrap() here because the "" key is guaranteed to exist since the compilation passed
        let value = compiled.remove("").unwrap();

        Ok(format!("{name}: {value}"))
    }

    fn as_json_pairs(&self) -> Result<String, GameReadError> {
        let mut result = Vec::new();

        if let Some(validate) = &self.validate {
            result.push(Self::compile_callback_data(&validate, "validate")?);
        }

        Ok(result.join(",\n"))
    }
}
