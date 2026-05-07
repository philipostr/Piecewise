use indent::indent_by;
use serde::Deserialize;
use std::io::{BufWriter, Write};

use crate::{*, pieces::*};

#[derive(Debug, Deserialize)]
pub struct Game {
    data: GameData,
    #[serde(default)]
    states: Vec<State>,
    #[serde(default)]
    events: Vec<Event>,
    #[serde(default)]
    pieces: Vec<Pieces>
}

impl Game {
    pub fn from_path(path: &str) -> Result<Self, GameReadError> {
        Ok(serde_saphyr::from_reader(
            std::fs::File::open(path)?
        )?)
    }

    fn js_load_children(&self) -> (String, String) {
        let (htmls, load_fns): (Vec<_>, Vec<_>) = self.pieces.iter()
            .map(|p| (p.html_skeleton(), p.js_load_call()))
            .unzip();

        (htmls.join(""), load_fns.join("\n"))
    }

    pub fn build(&self, target_dirpath: &str) -> Result<(), GameBuildError> {
        let html_contents = indoc::formatdoc! {r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>{}</title>
            </head>
            <body>
                <div id="Game"></div>
                <script type="module" src="script.js"></script>
            </body>
            </html>
        "#, self.data.title}.trim_end().to_string();
        std::fs::write(&format!("{target_dirpath}/index.html"), html_contents)?;

        let mut js_writer = BufWriter::new(
            std::fs::OpenOptions::new()
                .append(true)
                .open(&format!("{target_dirpath}/script.js"))?
        );

        let register_states = indent_by(4, self.states.iter()
            .map(|s: &State| s.js_register())
            .collect::<Vec<_>>()
            .join("\n")
        );
        let register_events = indent_by(4, self.events.iter()
            .map(|e| e.js_register())
            .collect::<Vec<_>>()
            .join("\n")
        );
        let load_children = {
            let (children_htmls, children_load_calls) = self.js_load_children();
            indent_by(4, indoc::formatdoc! {r#"
                slf.element.innerHTML = "{children_htmls}";
                {children_load_calls}
            "#}.trim_end().to_string())
        };
        let js_load_fn = indoc::formatdoc! {r##"
            function load_Game() {{
                let slf = {{
                    element: document.querySelector("#Game"),
                    states: Object.create(null),
                    events: Object.create(null),
                }};

                // States
                {register_states}

                // Events
                {register_events}

                // Children
                {load_children}
            }}


        "##}.trim_end().to_string();
        js_writer.write_all(js_load_fn.as_bytes())?;

        for piece in &self.pieces {
            piece.js_load_fn(&mut js_writer)?;
        }

        js_writer.flush().unwrap();

        Ok(())
    }
}

#[serde_inline_default::serde_inline_default]
#[derive(Debug, Deserialize)]
struct GameData {
    #[serde_inline_default("My Game".to_string())]
    title: String,
}
