use axum::{
    extract::{Path, State},
    http::{HeaderMap, Method, Uri},
    response::Response,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;

use crate::{
    app::App,
    error::{AppError, Result},
    models::{{ snake }}::{Create{{ pascal }}Data, Update{{ pascal }}Data, {{ pascal }}},
    routes::{{ plural }},
    services::auth::AuthSession,
};
use gurthang_http::AddRoute;
use gurthang_inertia::{InertiaPage, InertiaRenderMode, InertiaRenderer, InertiaRequest, mutation_redirect};

use super::shared::SharedProps;

pub fn register(router: Router<App>) -> Router<App> {
    router
{%- for binding in bindings %}
        .add_route({{ plural }}::{{ binding.ident }}, {{ binding.handler }})
{%- endfor %}
}

#[derive(Deserialize)]
pub struct {{ pascal }}Form {
{%- if form_empty %}
    pub _unused: Option<String>,
{%- else %}
{%- for field in form_fields %}
    pub {{ field.name }}: {{ field.ty }},
{%- endfor %}
{%- endif %}
}

impl From<{{ pascal }}Form> for Create{{ pascal }}Data {
    fn from(form: {{ pascal }}Form) -> Self {
        Self {
{%- for field in form_fields %}
            {{ field.name }}: form.{{ field.name }},
{%- endfor %}
        }
    }
}

impl From<{{ pascal }}Form> for Update{{ pascal }}Data {
    fn from(form: {{ pascal }}Form) -> Self {
        Self {
{%- for field in form_fields %}
            {{ field.name }}: form.{{ field.name }},
{%- endfor %}
        }
    }
}
{%- if has_item_props %}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct {{ pascal }}Props {
{%- for field in item_fields %}
    pub {{ field.name }}: {{ field.ty }},
{%- endfor %}
}

impl From<{{ pascal }}> for {{ pascal }}Props {
    fn from(item: {{ pascal }}) -> Self {
        Self {
{%- for field in item_fields %}
            {{ field.name }}: item.{{ field.name }},
{%- endfor %}
        }
    }
}
{%- endif %}
{% for page in pages %}
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct {{ page.name }} {
{%- if page.kind == "index" %}
    pub {{ plural }}: Vec<{{ pascal }}Props>,
{%- elif page.kind == "item" %}
    pub {{ snake }}: {{ pascal }}Props,
{%- endif %}
}

impl InertiaPage for {{ page.name }} {
    const COMPONENT: &'static str = "{{ page.component }}";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}
{% endfor %}
pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    use ts_rs::TS;
{%- if has_item_props %}
    {{ pascal }}Props::export()?;
{%- endif %}
{%- for page in pages %}
    {{ page.name }}::export()?;
{%- endfor %}
    Ok(())
}
{% for action in actions %}
{% if action == "index" %}
pub async fn index(
    State(database): State<PgPool>,
    State(inertia): State<InertiaRenderer>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let items = {{ pascal }}::list(&database).await?;
    let shared = SharedProps::from_auth(&auth).await?;
    inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            IndexProps {
                {{ plural }}: items.into_iter().map(Into::into).collect(),
            },
            shared,
        )
        .await
        .map_err(Into::into)
}
{% elif action == "show" %}
pub async fn show(
    State(database): State<PgPool>,
    State(inertia): State<InertiaRenderer>,
    auth: AuthSession,
    Path(id): Path<{{ pk_type }}>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let Some({{ snake }}) = {{ pascal }}::find(&database, id).await? else {
        return Err(AppError::NotFound);
    };
    let shared = SharedProps::from_auth(&auth).await?;
    inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            ShowProps {
                {{ snake }}: {{ snake }}.into(),
            },
            shared,
        )
        .await
        .map_err(Into::into)
}
{% elif action == "new" %}
pub async fn new(
    State(inertia): State<InertiaRenderer>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let shared = SharedProps::from_auth(&auth).await?;
    inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            NewProps {},
            shared,
        )
        .await
        .map_err(Into::into)
}
{% elif action == "create" %}
pub async fn create(
    State(database): State<PgPool>,
    Json(form): Json<{{ pascal }}Form>,
) -> Result<Response> {
    {{ pascal }}::create(&database, form.into()).await?;
    mutation_redirect({{ plural }}::{{ index_ident }}).map_err(Into::into)
}
{% elif action == "edit" %}
pub async fn edit(
    State(database): State<PgPool>,
    State(inertia): State<InertiaRenderer>,
    auth: AuthSession,
    Path(id): Path<{{ pk_type }}>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let Some({{ snake }}) = {{ pascal }}::find(&database, id).await? else {
        return Err(AppError::NotFound);
    };
    let shared = SharedProps::from_auth(&auth).await?;
    inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            EditProps {
                {{ snake }}: {{ snake }}.into(),
            },
            shared,
        )
        .await
        .map_err(Into::into)
}
{% elif action == "update" %}
pub async fn update(
    State(database): State<PgPool>,
    Path(id): Path<{{ pk_type }}>,
    Json(form): Json<{{ pascal }}Form>,
) -> Result<Response> {
    {{ pascal }}::update(&database, id, form.into()).await?;
    mutation_redirect({{ plural }}::{{ show_ident }}.url_with(id)).map_err(Into::into)
}
{% elif action == "destroy" %}
pub async fn destroy(
    State(database): State<PgPool>,
    Path(id): Path<{{ pk_type }}>,
) -> Result<Response> {
    {{ pascal }}::delete(&database, id).await?;
    mutation_redirect({{ plural }}::{{ index_ident }}).map_err(Into::into)
}
{% else %}
// unsupported action {{ action }}
{% endif %}
{%- endfor %}
