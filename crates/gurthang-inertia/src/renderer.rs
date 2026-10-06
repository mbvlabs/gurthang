use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    error::{Error, Result},
    page::{InertiaPage, InertiaRenderMode, Page},
    request::InertiaRequest,
    ssr::InertiaSsr,
};

const ROOT: &str = include_str!("root.html");

#[derive(Clone)]
pub struct InertiaRenderer {
    head_tags: String,
    title: String,
    version: String,
    ssr: InertiaSsr,
}

impl InertiaRenderer {
    pub fn new(
        head_tags: impl Into<String>,
        title: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            head_tags: head_tags.into(),
            title: title.into(),
            version: version.into(),
            ssr: InertiaSsr::default(),
        }
    }

    pub fn with_ssr(mut self, ssr: InertiaSsr) -> Self {
        self.ssr = ssr;
        self
    }

    pub async fn render<P, S>(
        &self,
        request: &InertiaRequest,
        props: P,
        shared: S,
    ) -> Result<Response>
    where
        P: InertiaPage,
        S: Serialize,
    {
        if request.is_inertia
            && request.is_get
            && request
                .version
                .as_deref()
                .is_some_and(|version| version != self.version)
        {
            return external_location(&request.url);
        }

        let mut values = object(serde_json::to_value(shared)?)?;
        values.extend(object(serde_json::to_value(props)?)?);
        filter_partial(request, P::COMPONENT, &mut values);
        values
            .entry("errors")
            .or_insert_with(|| Value::Object(Map::new()));

        let page = Page {
            component: P::COMPONENT,
            props: Value::Object(values),
            url: request.url.clone(),
            version: self.version.clone(),
        };

        if request.is_inertia {
            let mut response = Json(page).into_response();
            response
                .headers_mut()
                .insert("x-inertia", HeaderValue::from_static("true"));
            append_vary(response.headers_mut(), "X-Inertia")?;
            return Ok(response);
        }

        let ssr = if P::RENDER_MODE == InertiaRenderMode::Ssr {
            self.ssr.render(&page).await
        } else {
            None
        };
        let page_json = serde_json::to_string(&page)?.replace('/', "\\/");
        Ok(document(&self.title, &self.head_tags, &page_json, ssr.as_ref()).into_response())
    }
}

pub fn external_location(location: &str) -> Result<Response> {
    let value = HeaderValue::from_str(location)
        .map_err(|_| Error::BadRequest("location contains invalid header characters".into()))?;
    let mut response = StatusCode::CONFLICT.into_response();
    response.headers_mut().insert("x-inertia-location", value);
    append_vary(response.headers_mut(), "X-Inertia")?;
    Ok(response)
}

pub fn mutation_redirect(location: &str) -> Result<Response> {
    let value = HeaderValue::from_str(location)
        .map_err(|_| Error::BadRequest("redirect contains invalid header characters".into()))?;
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, value)]).into_response())
}

fn document(
    title: &str,
    asset_tags: &str,
    page_json: &str,
    ssr: Option<&crate::ssr::SsrOutput>,
) -> String {
    let (ssr_head, body) = match ssr {
        Some(ssr) => (
            ssr.head.join("\n"),
            ssr.body.clone(),
        ),
        None => (
            String::new(),
            format!(
                r#"<script data-page="app" type="application/json">{page_json}</script>
    <div id="app"></div>"#
            ),
        ),
    };
    ROOT.replace("__TITLE__", title)
        .replace("__SSR_HEAD__", &ssr_head)
        .replace("__ASSET_TAGS__", asset_tags)
        .replace("__BODY__", &body)
}

fn object(value: Value) -> Result<Map<String, Value>> {
    value.as_object().cloned().ok_or(Error::Internal)
}

fn filter_partial(request: &InertiaRequest, component: &str, props: &mut Map<String, Value>) {
    if request.partial_component.as_deref() != Some(component) {
        return;
    }
    if !request.partial_data.is_empty() {
        props.retain(|key, _| request.partial_data.contains(key) || key == "errors");
    }
    if !request.partial_except.is_empty() {
        props.retain(|key, _| !request.partial_except.contains(key) || key == "errors");
    }
}

fn append_vary(headers: &mut HeaderMap, value: &str) -> Result<()> {
    let existing = headers
        .get(header::VARY)
        .and_then(|value| value.to_str().ok());
    if existing
        .into_iter()
        .flat_map(|values| values.split(','))
        .any(|item| item.trim().eq_ignore_ascii_case(value))
    {
        return Ok(());
    }
    let combined = existing.map_or_else(
        || value.to_owned(),
        |existing| format!("{existing}, {value}"),
    );
    let combined = HeaderValue::from_str(&combined).map_err(|_| Error::Internal)?;
    headers.insert(header::VARY, combined);
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::{HeaderMap, HeaderName, Method, Uri},
    };

    use super::*;

    #[derive(Serialize)]
    struct TestProps {
        keep: &'static str,
        drop: &'static str,
    }

    impl InertiaPage for TestProps {
        const COMPONENT: &'static str = "Test";
    }

    #[derive(Serialize)]
    struct EmptyShared {}

    fn request(headers: &[(&str, &str)]) -> InertiaRequest {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        InertiaRequest::from_parts(&Method::GET, &Uri::from_static("/test?one=1"), &map)
    }

    fn renderer() -> InertiaRenderer {
        InertiaRenderer::new("", "Test", "v1")
    }

    #[tokio::test]
    async fn inertia_json_has_protocol_headers_and_empty_errors() {
        let response = renderer()
            .render(
                &request(&[("x-inertia", "true"), ("x-inertia-version", "v1")]),
                TestProps {
                    keep: "yes",
                    drop: "no",
                },
                EmptyShared {},
            )
            .await
            .unwrap();
        assert_eq!(response.headers()["x-inertia"], "true");
        assert_eq!(response.headers()[header::VARY], "X-Inertia");
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let page: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(page["component"], "Test");
        assert_eq!(page["url"], "/test?one=1");
        assert_eq!(page["props"]["errors"], serde_json::json!({}));
    }

    #[tokio::test]
    async fn partial_include_and_exclude_are_applied_with_errors_retained() {
        let response = renderer()
            .render(
                &request(&[
                    ("x-inertia", "true"),
                    ("x-inertia-partial-component", "Test"),
                    ("x-inertia-partial-data", "keep,drop"),
                    ("x-inertia-partial-except", "drop"),
                ]),
                TestProps {
                    keep: "yes",
                    drop: "no",
                },
                EmptyShared {},
            )
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let page: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(page["props"]["keep"], "yes");
        assert!(page["props"].get("drop").is_none());
        assert_eq!(page["props"]["errors"], serde_json::json!({}));
    }

    #[tokio::test]
    async fn initial_html_escapes_script_termination_without_entities() {
        let response = renderer()
            .render(
                &request(&[]),
                TestProps {
                    keep: "</script>",
                    drop: "&",
                },
                EmptyShared {},
            )
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains(r#"<\/script>"#));
        assert!(html.contains(r#""drop":"&""#));
        assert!(!html.contains("&quot;"));
    }

    #[tokio::test]
    async fn version_mismatch_uses_conflict() {
        let response = renderer()
            .render(
                &request(&[("x-inertia", "true"), ("x-inertia-version", "old")]),
                TestProps {
                    keep: "yes",
                    drop: "no",
                },
                EmptyShared {},
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response.headers()["x-inertia-location"], "/test?one=1");
    }

    #[test]
    fn redirects_after_mutations_use_see_other() {
        let response = mutation_redirect("/dashboard").unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/dashboard");
    }
}
