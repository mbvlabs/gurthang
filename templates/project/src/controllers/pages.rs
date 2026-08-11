use axum::{extract::State, response::Html};

use crate::{app::AppState, error::Result, views::public::HomeView};

pub async fn home(State(state): State<AppState>) -> Result<Html<String>> {
    state.templates.render("pages/home.html", &HomeView::new())
}
