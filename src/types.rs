use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum Command {
    #[serde(rename = "navigate")]
    Navigate { url: String },

    #[serde(rename = "click")]
    Click { x: f64, y: f64 },

    #[serde(rename = "key")]
    Key { key: String },
}
