use async_trait::async_trait;
use gurthang::prelude::*;

pub struct ViewEngineInitializer;

#[async_trait]
impl Initializer for ViewEngineInitializer {
    fn name(&self) -> &'static str {
        "view-engine"
    }

    async fn before_run(&self, ctx: &mut Context) -> gurthang::Result<()> {
        let resolver = if ctx.config.is_development() {
            let server_url = ctx.config.inertia.vite_dev_server_url.clone().ok_or_else(|| {
                Error::Config("inertia.vite_dev_server_url is required in development".into())
            })?;
            AssetResolver::development(server_url)
        } else {
            AssetResolver::from_manifest_bytes(crate::assets::manifest_bytes())?
        };
        let ssr = InertiaSsr::from_options(SsrOptions {
            runtime: ctx.config.inertia.ssr_runtime.clone(),
            timeout_ms: ctx.config.inertia.ssr_timeout_ms,
            development_bundle: std::path::PathBuf::from("dist-ssr/ssr.mjs"),
            embedded_bundle: include_bytes!(concat!(env!("OUT_DIR"), "/inertia-ssr.mjs")),
            is_development: ctx.config.is_development(),
        });
        ctx.inertia = InertiaRenderer::new(
            resolver.head_tags(),
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        )
        .with_ssr(ssr);
        Ok(())
    }
}
