pub mod app;
pub mod config;
pub mod controllers;
pub mod error;
pub mod models;
pub mod routes;
pub mod services;
pub mod views;
pub mod web;

pub const fn application_name() -> &'static str {
    "__GURTHANG_PROJECT_NAME__"
}
