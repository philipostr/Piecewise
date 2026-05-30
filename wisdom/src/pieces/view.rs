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
        format!("View${id}")
    }

    fn piece_name(&self) -> String {
        "View".to_string()
    }

    fn html_tag(&self) -> String {
        "div".to_string()
    }

    fn extra_slf_props(&self) -> Option<String> {
        let mut options_load = Vec::new();
        let options = self.pieces.iter()
            .map(|p| (p.html_skeleton(), format!("unloads.push({});", p.js_load_call())));

        for (i, (html, load_call)) in options.enumerate() {
            options_load.push(indoc::formatdoc! {r#"
                if (index == {i}) {{
                    this.element.innerHTML = "{html}";
                    {load_call}
                }}
            "#}.trim_end().to_string());
        }

        let options_load = indent_by(4, options_load.join("\n"));
        Some(indoc::formatdoc! {r#"
            unloads: () => {{}},
            curr_index: undefined,
            swapView: function(index) {{
                if (index == this.curr_index) {{
                    return;
                }}

                this.unloads();
                let unloads = [];

                this.curr_index = index;
                {options_load}

                this.unloads = () => {{
                    unloads.forEach((unload) => unload());
                }};
            }}
        "#}.trim_end().to_string())
    }

    fn children(&self) -> Option<&[Pieces]> {
        Some(&self.pieces)
    }
}

#[derive(Debug, Deserialize)]
pub struct ViewData {
    index: DynamicBindString
}

impl PieceData for ViewData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        let mut data_inits = Vec::new();
        let mut bindings = HashMap::new();

        compile_bindings(&self.index, &mut data_inits, SetData::Function("slf.swapView".to_string()), &mut bindings)?;

        let data_subscriptions = register_subscriptions(bindings);

        Ok((data_inits.join("\n"), data_subscriptions.join("\n\n")))
    }
}
