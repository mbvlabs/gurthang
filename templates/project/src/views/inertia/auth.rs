use serde::Serialize;
use ts_rs::TS;

use gurthang_inertia::{InertiaPage, InertiaRenderMode};

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct LoginProps {
    pub email: Option<String>,
}

impl InertiaPage for LoginProps {
    const COMPONENT: &'static str = "Auth/Login";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct RegisterProps {
    pub email: Option<String>,
}

impl InertiaPage for RegisterProps {
    const COMPONENT: &'static str = "Auth/Register";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}
