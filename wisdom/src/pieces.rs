pub mod button;
use std::{collections::HashMap, io::Write};

pub use button::*;
pub mod game;
pub use game::*;
pub mod text;
use indent::indent_by;
use serde::Deserialize;
pub use text::*;
pub mod view;
pub use view::*;

use crate::*;

pub trait PieceData {
    fn js_register(&self) -> Result<(String, String), GameReadError>;
}

trait Piece {
    type Data: PieceData;

    fn states(&self) -> &Vec<State>;
    fn events(&self) -> &Vec<Event>;
    fn data(&self) -> &Self::Data;
    fn piece_id(&self, escaped_dollar: bool) -> String;
    fn html_tag(&self) -> String;

    fn html_skeleton(&self) -> String {
        format!(r#"<{tag} id=\"{id}\"></{tag}>"#, tag = self.html_tag(), id = self.piece_id(false))
    }

    /// Most pieces are terminal, meaning they don't have children. Override
    /// the default implementation for collection pieces.
    fn children(&self) -> Option<&Vec<Pieces>> {
        None
    }

    /// Most pieces are terminal, meaning they don't have children. Override
    /// the default implementation for collection pieces.
    fn js_load_children(&self) -> Option<(String, String)> {
        None
    }

    fn js_load_fn(&self, writer: &mut std::io::BufWriter<std::fs::File>) -> Result<(), GameBuildError> {
        let (mut data_inits, mut data_subscriptions) = self.data().js_register()?;
        data_inits = indent_by(4, data_inits);
        data_subscriptions = indent_by(4, data_subscriptions);

        let piece_id = self.piece_id(false);
        let escaped_piece_id = self.piece_id(true);
        let register_states = indent_by(4, self.states().iter()
            .map(|s: &State| s.js_register())
            .collect::<Vec<_>>()
            .join("\n")
        );
        let register_events = indent_by(4, self.events().iter()
            .map(|e| e.js_register())
            .collect::<Vec<_>>()
            .join("\n")
        );
        let load_children = {
            if let Some((children_htmls, children_load_calls)) = self.js_load_children() {
                indent_by(4, indoc::formatdoc! {r#"
                    slf.element.innerHTML = "{children_htmls}";
                    {children_load_calls}
                "#}.trim_end().to_string())
            } else {
                "".to_string()
            }
        };
        let js_load_fn = indoc::formatdoc! {r##"
            function load_{piece_id}(parent) {{
                let slf = {{
                    element: document.querySelector("#{escaped_piece_id}"),
                    states: Object.create(parent.states),
                    events: Object.create(parent.events),
                }};

                // States
                {register_states}

                // Events
                {register_events}

                // State subscriptions
                {data_subscriptions}

                // Data initializations
                {data_inits}

                // Children
                {load_children}
            }}
        "##}.trim_end().to_string();
        writer.write_all(js_load_fn.as_bytes())?;

        if let Some(children) = self.children() {
            for child in children {
                child.js_load_fn(writer)?;
            }
        }

        Ok(())
    }

    fn js_load_call(&self) -> String {
        format!("load_{}(slf);", self.piece_id(false))
    }
}

/// Exists to allow collection pieces (e.g. `View`) to hold all types of pieces
/// polymorphically, without using trait objects. This is important because trait objects
/// are very difficult to deserialize.
#[derive(Debug, Deserialize)]
enum Pieces {
    Button(Button),
    Text(Text),
    View(View),
}

impl Pieces {
    pub fn html_skeleton(&self) -> String {
        match self {
            Pieces::Button(button) => button.html_skeleton(),
            Pieces::Text(text) => text.html_skeleton(),
            Pieces::View(view) => view.html_skeleton(),
        }
    }

    pub fn js_load_fn(&self, writer: &mut std::io::BufWriter<std::fs::File>) -> Result<(), GameBuildError> {
        match self {
            Pieces::Button(button) => button.js_load_fn(writer),
            Pieces::Text(text) => text.js_load_fn(writer),
            Pieces::View(view) => view.js_load_fn(writer),
        }
    }

    pub fn js_load_call(&self) -> String {
        match self {
            Pieces::Button(button) => button.js_load_call(),
            Pieces::Text(text) => text.js_load_call(),
            Pieces::View(view) => view.js_load_call(),
        }
    }
}

fn compile_bindings(
    data: &DynamicBindString, 
    result: &mut Vec<String>, 
    data_var: &str, 
    bindings: &mut HashMap<String, Vec<String>>
) -> Result<(), GameReadError> {
    let mut binds = data.compile(data_var)?;
    result.push(binds.remove("").unwrap());

    for (key, val) in binds {
        if let Some(state_bindings) = bindings.get_mut(&key) {
            state_bindings.push(val);
        } else {
            bindings.insert(key, vec![val]);
        }
    }

    Ok(())
}

fn register_subscriptions(bindings: HashMap<String, Vec<String>>) -> Vec<String> {
    let mut data_subscriptions = Vec::new();
    for (sub, callbacks) in bindings {
        let callbacks = indent_by(4, callbacks.join("\n\n"));
        data_subscriptions.push(indoc::formatdoc! {r#"
            State.subscribe(slf.states.{sub}, ({sub}) => {{
                {callbacks}
            }});
        "#}.trim_end().to_string());
    }

    data_subscriptions
}
