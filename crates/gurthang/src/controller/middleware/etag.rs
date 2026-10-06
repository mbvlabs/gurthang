use std::task::{Context as TaskContext, Poll};

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{
        StatusCode,
        header::{CACHE_CONTROL, CONTENT_LOCATION, DATE, ETAG, EXPIRES, IF_NONE_MATCH, VARY},
    },
    response::Response,
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Etag {
    #[serde(default)]
    pub enable: bool,
}

impl MiddlewareLayer for Etag {
    fn name(&self) -> &'static str {
        "etag"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Post
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app.layer(EtagLayer))
    }
}

#[derive(Clone, Default)]
struct EtagLayer;

impl<S> Layer<S> for EtagLayer {
    type Service = EtagMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EtagMiddleware { inner }
    }
}

#[derive(Clone)]
struct EtagMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for EtagMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut TaskContext<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let if_none_match = request.headers().get(IF_NONE_MATCH).cloned();
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;
            let etag = response.headers().get(ETAG).cloned();
            if let (Some(if_none_match), Some(etag)) = (if_none_match, etag)
                && if_none_match == etag
            {
                let mut builder = Response::builder().status(StatusCode::NOT_MODIFIED);
                if let Some(headers) = builder.headers_mut() {
                    for name in [ETAG, CACHE_CONTROL, VARY, EXPIRES, DATE, CONTENT_LOCATION] {
                        if let Some(value) = response.headers().get(&name) {
                            headers.insert(name, value.clone());
                        }
                    }
                }
                return Ok(builder.body(Body::empty()).expect("empty 304 body"));
            }
            Ok(response)
        })
    }
}
