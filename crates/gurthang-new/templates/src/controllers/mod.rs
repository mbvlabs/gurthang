pub mod auth;
pub mod dashboard;
pub mod shared;
pub mod welcome;

pub fn register(router: axum::Router<crate::app::App>) -> axum::Router<crate::app::App> {
    let router = auth::register(router);
    let router = dashboard::register(router);
    let router = welcome::register(router);
    router
}

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    auth::export_payloads()?;
    dashboard::export_payloads()?;
    shared::export_payloads()?;
    welcome::export_payloads()?;
    Ok(())
}
