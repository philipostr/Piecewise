use serde::Deserialize;
use std::collections::HashMap;

use crate::{*, pieces::*};

#[derive(Debug, Deserialize)]
pub struct Text {
    id: String,
    #[serde(default)]
    data: TextData,
    #[serde(default)]
    states: Vec<State>,
    #[serde(default)]
    events: Vec<Event>,
}

impl Piece for Text {
    type Data = TextData;

    fn states(&self) -> &[State] {
        &self.states
    }

    fn events(&self) -> &[Event] {
        &self.events
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn piece_id(&self) -> String {
        let id = &self.id;
        format!("Text${id}")
    }

    fn piece_name(&self) -> String {
        "Text".to_string()
    }

    fn html_tag(&self) -> String {
        "p".to_string()
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct TextData {
    text: Option<DynamicBindString>,
}

impl PieceData for TextData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        let mut data_inits = Vec::new();
        let mut bindings = HashMap::new();

        if let Some(text) = &self.text {
            compile_bindings(text, &mut data_inits, SetData::Variable("slf.element.innerHTML".to_string()), &mut bindings)?;
        }

        let data_subscriptions = register_subscriptions(bindings);

        Ok((data_inits.join("\n"), data_subscriptions.join("\n\n")))
    }
}
