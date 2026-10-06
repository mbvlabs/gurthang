use async_trait::async_trait;
use gurthang::prelude::*;

use crate::services::auth::AuthBackend;

pub struct AuthInitializer;

#[async_trait]
impl Initializer for AuthInitializer {
    fn name(&self) -> &'static str {
        "auth"
    }

    async fn after_routes(
        &self,
        router: Router<Context>,
        ctx: &Context,
    ) -> gurthang::Result<Router<Context>> {
        Ok(session_auth::layer(
            router,
            ctx,
            AuthBackend::new(ctx.db.clone()),
        ))
    }
}
