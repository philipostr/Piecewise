use serde::Deserialize;

use crate::{*, pieces::*};

#[derive(Debug, Deserialize)]
pub struct Section {
    id: String,
    #[serde(default)]
    data: SectionData,
    #[serde(default)]
    states: Vec<State>,
    #[serde(default)]
    events: Vec<Event>,
    #[serde(default)]
    pieces: Vec<Pieces>
}

impl Piece for Section {
    type Data = SectionData;

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
        format!("Section${id}")
    }

    fn piece_name(&self) -> String {
        "Section".to_string()
    }

    fn html_tag(&self) -> String {
        "div".to_string()
    }

    fn js_load_children(&self) -> Option<String> {
        let load_fns = self.pieces.iter()
            .map(|p| format!("unloads.push({});", p.js_load_call()))
            .collect::<Vec<_>>();

        Some(load_fns.join("\n"))
    }

    fn children(&self) -> Option<&[Pieces]> {
        Some(&self.pieces)
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SectionData {}

impl PieceData for SectionData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        Ok(("".to_string(), "".to_string()))
    }
}
