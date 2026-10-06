use axum::{Router, routing::get};
use gurthang_http::Route;

use crate::{app::AppState, controllers::dashboard};

pub const DASHBOARD: Route = Route {
    name: "dashboard",
    method: "GET",
    path: "/dashboard",
};

pub fn mount(router: Router<AppState>) -> Router<AppState> {
    router.route(DASHBOARD.path, get(dashboard::show))
}
