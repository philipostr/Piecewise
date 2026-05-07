use serde::Deserialize;

use crate::{*, pieces::*};

#[derive(Debug, Deserialize)]
pub struct View {
    id: String,
    data: ViewData,
    #[serde(default)]
    states: Vec<State>,
    #[serde(default)]
    events: Vec<Event>,
    #[serde(default)]
    pieces: Vec<Pieces>
}

impl Piece for View {
    type Data = ViewData;

    fn states(&self) -> &Vec<State> {
        &self.states
    }

    fn events(&self) -> &Vec<Event> {
        &self.events
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn piece_id(&self, escaped_dollar: bool) -> String {
        let id = &self.id;
        if escaped_dollar {
            format!(r#"View\\${id}"#)
        } else {
            format!("View${id}")
        }
    }

    fn html_tag(&self) -> String {
        "div".to_string()
    }

    fn js_load_children(&self) -> Option<(String, String)> {
        let (htmls, load_fns): (Vec<_>, Vec<_>) = self.pieces.iter()
            .map(|p| (p.html_skeleton(), p.js_load_call()))
            .unzip();

        Some((htmls.join(""), load_fns.join("\n")))
    }

    fn children(&self) -> Option<&Vec<Pieces>> {
        Some(&self.pieces)
    }
}

#[derive(Debug, Deserialize)]
pub struct ViewData {}

impl PieceData for ViewData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        Ok(("".to_string(), "".to_string()))
    }
}
