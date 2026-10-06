use axum::{Router, routing::get};
use gurthang_http::Route;

use crate::{app::AppState, controllers::welcome};

pub const WELCOME: Route = Route {
    name: "welcome",
    method: "GET",
    path: "/",
};

pub fn mount(router: Router<AppState>) -> Router<AppState> {
    router.route(WELCOME.path, get(welcome::show))
}
