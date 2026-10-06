use std::collections::BTreeSet;

use axum::http::{HeaderMap, Method, Uri};

#[derive(Clone, Debug)]
pub struct InertiaRequest {
    pub is_inertia: bool,
    pub is_get: bool,
    pub url: String,
    pub version: Option<String>,
    pub partial_component: Option<String>,
    pub partial_data: BTreeSet<String>,
    pub partial_except: BTreeSet<String>,
    pub error_bag: Option<String>,
}

impl InertiaRequest {
    pub fn from_parts(method: &Method, uri: &Uri, headers: &HeaderMap) -> Self {
        Self {
            is_inertia: header(headers, "x-inertia").is_some_and(|value| value == "true"),
            is_get: method == Method::GET,
            url: uri
                .path_and_query()
                .map_or_else(|| "/".into(), ToString::to_string),
            version: header(headers, "x-inertia-version").map(str::to_owned),
            partial_component: header(headers, "x-inertia-partial-component").map(str::to_owned),
            partial_data: list(headers, "x-inertia-partial-data"),
            partial_except: list(headers, "x-inertia-partial-except"),
            error_bag: header(headers, "x-inertia-error-bag").map(str::to_owned),
        }
    }
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn list(headers: &HeaderMap, name: &str) -> BTreeSet<String> {
    header(headers, name)
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
