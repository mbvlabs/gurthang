use serde::Serialize;
use ts_rs::TS;

use super::InertiaPage;
use super::shared::SafeUser;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct DashboardProps {
    pub title: String,
    pub status: String,
    pub user: SafeUser,
}

impl InertiaPage for DashboardProps {
    const COMPONENT: &'static str = "Dashboard";
}
