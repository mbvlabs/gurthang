use gurthang::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::{AppError, Result},
    models::{{ snake }}::{Create{{ pascal }}Data, Update{{ pascal }}Data, {{ pascal }}},
    routes::{{ plural }},
    services::auth::AuthSession,
};

use super::shared::SharedProps;

#[derive(Clone)]
pub struct {{ struct_name }} {
    pub database: PgPool,
    pub inertia: InertiaRenderer,
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
{% endfor %}
pub fn routes(ctx: &Context) -> Router<Context> {
    let controller = {{ struct_name }} {
        database: ctx.db.clone(),
        inertia: ctx.inertia.clone(),
    };
    mount!(controller, {
{%- for route in mount_routes %}
        {{ plural }}::{{ route.ident }} => {{ route.verb }}(controller, {{ struct_name }}::{{ route.handler }}),
{%- endfor %}
    })
}

impl {{ struct_name }} {
{%- for action in actions %}
{%- if action == "index" %}
    pub async fn index(
        self,
        auth: AuthSession,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        let items = {{ pascal }}::list(&self.database).await?;
        let shared = SharedProps::from_auth(&auth).await?;
        self.inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "{{ index_component }}",
                IndexProps {
                    {{ plural }}: items.into_iter().map(Into::into).collect(),
                },
                shared,
            )
            .await
            .map_err(Into::into)
    }
{%- elif action == "show" %}
    pub async fn show(
        self,
        auth: AuthSession,
        Path(id): Path<{{ pk_type }}>,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        let Some({{ snake }}) = {{ pascal }}::find(&self.database, id).await? else {
            return Err(AppError::NotFound);
        };
        let shared = SharedProps::from_auth(&auth).await?;
        self.inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "{{ show_component }}",
                ShowProps {
                    {{ snake }}: {{ snake }}.into(),
                },
                shared,
            )
            .await
            .map_err(Into::into)
    }
{%- elif action == "new" %}
    pub async fn new(
        self,
        auth: AuthSession,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        let shared = SharedProps::from_auth(&auth).await?;
        self.inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "{{ new_component }}",
                NewProps {},
                shared,
            )
            .await
            .map_err(Into::into)
    }
{%- elif action == "create" %}
    pub async fn create(self, Json(form): Json<{{ pascal }}Form>) -> Result<Response> {
        {{ pascal }}::create(&self.database, form.into()).await?;
        mutation_redirect({{ plural }}::{{ index_ident }}).map_err(Into::into)
    }
{%- elif action == "edit" %}
    pub async fn edit(
        self,
        auth: AuthSession,
        Path(id): Path<{{ pk_type }}>,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        let Some({{ snake }}) = {{ pascal }}::find(&self.database, id).await? else {
            return Err(AppError::NotFound);
        };
        let shared = SharedProps::from_auth(&auth).await?;
        self.inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "{{ edit_component }}",
                EditProps {
                    {{ snake }}: {{ snake }}.into(),
                },
                shared,
            )
            .await
            .map_err(Into::into)
    }
{%- elif action == "update" %}
    pub async fn update(
        self,
        Path(id): Path<{{ pk_type }}>,
        Json(form): Json<{{ pascal }}Form>,
    ) -> Result<Response> {
        {{ pascal }}::update(&self.database, id, form.into()).await?;
        mutation_redirect({{ plural }}::{{ show_ident }}.url_with(id)).map_err(Into::into)
    }
{%- elif action == "destroy" %}
    pub async fn destroy(self, Path(id): Path<{{ pk_type }}>) -> Result<Response> {
        {{ pascal }}::delete(&self.database, id).await?;
        mutation_redirect({{ plural }}::{{ index_ident }}).map_err(Into::into)
    }
{%- endif %}
{%- endfor %}
}
