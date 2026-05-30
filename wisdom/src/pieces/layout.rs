use serde::Deserialize;

use crate::{*, pieces::*};

#[derive(Debug, Deserialize)]
pub struct Layout {
    id: String,
    #[serde(default)]
    data: LayoutData,
    #[serde(default)]
    states: Vec<State>,
    #[serde(default)]
    events: Vec<Event>,
    #[serde(default)]
    pieces: Vec<Pieces>
}

impl Piece for Layout {
    type Data = LayoutData;

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
        format!("Layout${id}")
    }

    fn piece_name(&self) -> String {
        "Layout".to_string()
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

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum PlaceX {
    #[default]
    SpaceEvenly,
}

impl From<PlaceX> for String {
    fn from(value: PlaceX) -> Self {
        match value {
            PlaceX::SpaceEvenly => "space-evenly".to_string(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutData {
    place_x: PlaceX,
}

impl PieceData for LayoutData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        let mut data_inits = Vec::new();

        data_inits.push(format!("slf.element.style.justifyContent = \"{}\";", String::from(self.place_x)));

        Ok((data_inits.join("\n"), "".to_string()))
    }
}
