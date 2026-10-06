use axum::{
    Router,
    routing::{delete, get, post},
};
use gurthang_http::Route;

use crate::{app::AppState, controllers::auth};

pub const REGISTER: Route = Route {
    name: "register",
    method: "GET",
    path: "/register",
};

pub const REGISTER_CREATE: Route = Route {
    name: "register.create",
    method: "POST",
    path: "/register",
};

pub const LOGIN: Route = Route {
    name: "login",
    method: "GET",
    path: "/login",
};

pub const LOGIN_CREATE: Route = Route {
    name: "login.create",
    method: "POST",
    path: "/login",
};

pub const LOGOUT: Route = Route {
    name: "logout",
    method: "DELETE",
    path: "/logout",
};

pub fn mount(router: Router<AppState>) -> Router<AppState> {
    router
        .route(REGISTER.path, get(auth::new_register))
        .route(REGISTER_CREATE.path, post(auth::register))
        .route(LOGIN.path, get(auth::new_login))
        .route(LOGIN_CREATE.path, post(auth::login))
        .route(LOGOUT.path, delete(auth::logout))
}
