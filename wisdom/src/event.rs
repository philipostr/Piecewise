use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Event {}

impl Event {
    pub fn js_register(&self) -> String {
        "".to_string()
    }
}
