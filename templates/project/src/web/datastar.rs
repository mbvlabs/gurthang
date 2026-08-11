use std::convert::Infallible;

use axum::{
    extract::State,
    response::{Sse, sse::Event},
};
use datastar::{axum::ReadSignals, prelude::PatchElements};
use serde::{Deserialize, Serialize};
use tokio_stream::Stream;

use crate::{app::AppState, error::Result};

#[derive(Debug, Deserialize)]
pub struct CounterSignals {
    #[serde(default)]
    count: i64,
}

#[derive(Serialize)]
struct CounterView {
    count: i64,
}

pub async fn counter(
    State(state): State<AppState>,
    ReadSignals(signals): ReadSignals<CounterSignals>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    let fragment = state.templates.render_to_string(
        "fragments/counter.html",
        &CounterView {
            count: signals.count.saturating_add(1),
        },
    )?;
    let event = PatchElements::new(fragment).selector("#counter");
    Ok(Sse::new(tokio_stream::iter([Ok(event.into())])))
}
