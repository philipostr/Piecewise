use indent::indent_by;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", rename_all_fields = "camelCase")]
pub enum State {
    Int {
        id: String,
        initial_value: i128,
        data: IntStateData
    }
}

impl State {
    pub fn js_register(&self) -> String {
        match self {
            Self::Int { 
                id, 
                initial_value,
                data 
            } => {
                let data_obj = indent_by(4, data.as_json_pairs());
                indoc::formatdoc! {r#"
                    slf.states.{id} = State.createInt({initial_value}, {{
                        {data_obj}
                    }})
                "#}.trim_end().to_string()
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct IntStateData {
    minimum: Option<i128>,
    maximum: Option<i128>,
}

impl IntStateData {
    fn as_json_pairs(&self) -> String {
        let mut result = Vec::new();

        if let Some(min) = self.minimum {
            result.push(format!("minimum: {min}"));
        }
        if let Some(max) = self.maximum {
            result.push(format!("maximum: {max}"));
        }

        result.join(",\n")
    }
}
