use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum Command {
    #[serde(rename = "navigate")]
    Navigate { url: String },

    #[serde(rename = "click")]
    Click { x: i64, y: i64 },

    #[serde(rename = "key")]
    Key { key: String },
}
