use serde::Deserialize;

use crate::{*, pieces::*};

#[derive(Debug, Deserialize)]
pub struct Generator {
    id: String,
    data: GeneratorData,
    #[serde(default)]
    states: Vec<State>,
    #[serde(default)]
    events: Vec<Event>,
    base: Box<Pieces>
}

impl Piece for Generator {
    type Data = GeneratorData;

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
            format!(r#"Generator\\${id}"#)
        } else {
            format!("Generator${id}")
        }
    }

    fn piece_name(&self) -> String {
        "Generator".to_string()
    }

    fn html_tag(&self) -> String {
        "div".to_string()
    }

    fn extra_slf_props(&self) -> Option<String> {
        let idx = self.data.index.clone();
        let it = self.data.iterator.clone();

        let load_child = indent_by(8, indoc::formatdoc! {r#"
            let child = document.createElement(null);
            slf.element.appendChild(child);
            child.outerHTML = {};
            {}
        "#, self.base.html_skeleton(), format!("unloads.push({});", self.base.js_load_call())}.trim_end().to_string());

        Some(indoc::formatdoc! {r#"
            unloads: () => {{}},
            update: function(list) {{
                this.unloads();
                slf.element.innerHTML = "";
                let unloads = [];

                list.forEach((it, idx) => {{
                    let sub_id = crypto.randomUUID().split("-")[0];
                    let inputs = Object.create(null);
                    if ("{it}" != "_") {{
                        inputs.{it} = it;
                    }}
                    if ("{idx}" != "_") {{
                        inputs.{idx} = idx;
                    }}

                    // Base
                    {load_child}
                }});

                this.unloads = () => {{
                    unloads.forEach((unload) => unload());
                }};
            }}
        "#}.trim_end().to_string())
    }

    fn children(&self) -> Option<&[Pieces]> {
        Some(std::slice::from_ref(&self.base))
    }
}

#[serde_inline_default::serde_inline_default]
#[derive(Debug, Deserialize)]
pub struct GeneratorData {
    #[serde_inline_default("_".to_string())]
    index: String,
    #[serde_inline_default("_".to_string())]
    iterator: String,
    list: DynamicBindString,
}

impl PieceData for GeneratorData {
    fn js_register(&self) -> Result<(String, String), GameReadError> {
        let mut data_inits = Vec::new();
        let mut bindings = HashMap::new();

        compile_bindings(&self.list, &mut data_inits, SetData::Function("slf.update".to_string()), &mut bindings)?;

        let data_subscriptions = register_subscriptions(bindings);

        Ok((data_inits.join("\n"), data_subscriptions.join("\n\n")))
    }
}
