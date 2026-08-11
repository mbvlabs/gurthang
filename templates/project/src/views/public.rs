use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HomeView {
    pub title: &'static str,
    pub message: &'static str,
    pub count: i64,
}

impl HomeView {
    pub fn new() -> Self {
        Self {
            title: "__GURTHANG_PROJECT_NAME__",
            message: "Your Axum, PostgreSQL, and Tera application is ready.",
            count: 0,
        }
    }
}

impl Default for HomeView {
    fn default() -> Self {
        Self::new()
    }
}
