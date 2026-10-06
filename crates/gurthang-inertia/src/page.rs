use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InertiaRenderMode {
    Client,
    Ssr,
}

pub trait InertiaPage: Serialize {
    const COMPONENT: &'static str;
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub component: &'static str,
    pub props: Value,
    pub url: String,
    pub version: String,
}
