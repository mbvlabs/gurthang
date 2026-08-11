use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub component: &'static str,
    pub props: Value,
    pub url: String,
    pub version: String,
}
