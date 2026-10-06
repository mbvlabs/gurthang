use async_trait::async_trait;
use axum::routing::get;
use gurthang::prelude::*;

pub struct AssetsInitializer;

#[async_trait]
impl Initializer for AssetsInitializer {
    fn name(&self) -> &'static str {
        "assets"
    }

    async fn after_routes(
        &self,
        router: Router<Context>,
        ctx: &Context,
    ) -> gurthang::Result<Router<Context>> {
        Ok(if ctx.config.is_development() {
            gurthang::boot::serve_dev_assets(router)
        } else {
            router
                .route("/assets/{*path}", get(crate::assets::serve_public))
                .route("/build/{*path}", get(crate::assets::serve_build))
        })
    }
}
