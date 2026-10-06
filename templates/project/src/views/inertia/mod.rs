// gurthang:generated:modules:start
pub mod auth;
pub mod dashboard;
pub mod welcome;
// gurthang:generated:modules:end
pub mod shared;

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    use ts_rs::TS;
    welcome::WelcomeProps::export()?;
    auth::LoginProps::export()?;
    auth::RegisterProps::export()?;
    dashboard::DashboardProps::export()?;
    shared::SafeUser::export()?;
    shared::AuthProps::export()?;
    shared::FlashProps::export()?;
    shared::SharedProps::export()?;
    // gurthang:generated:exports:start
    // gurthang:generated:exports:end
    Ok(())
}
