use serde::Serialize;
use ts_rs::TS;

use gurthang_inertia::{InertiaPage, InertiaRenderMode};

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct WelcomeProps {
    pub title: String,
    pub status: String,
}

impl InertiaPage for WelcomeProps {
    const COMPONENT: &'static str = "Welcome";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}
