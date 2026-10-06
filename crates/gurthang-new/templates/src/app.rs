use gurthang::prelude::*;
use gurthang::Result;

use crate::{controllers, initializers, workers};

pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    async fn boot(mode: StartMode, config: Config) -> Result<BootResult> {
        create_app::<Self>(mode, config).await
    }

    fn routes(ctx: &Context) -> AppRoutes {
        AppRoutes::new()
            .add_route(controllers::welcome::routes(ctx))
            .add_route(controllers::auth::routes(ctx))
            .add_route(controllers::dashboard::routes(ctx))
    }

    async fn connect_workers(ctx: &Context) -> Result<()> {
        workers::purge_expired_sessions::register(ctx);
        if matches!(ctx.start_mode, StartMode::Web) {
            return Ok(());
        }
        let config = ctx.config.worker_config()?;
        let worker = JobWorker::<workers::Job>::new(ctx.db.clone(), config);
        let shutdown = ctx.shutdown();
        if ctx.start_mode == StartMode::Worker {
            worker.run(shutdown).await;
        } else {
            tokio::spawn(async move {
                worker.run(shutdown).await;
            });
        }
        Ok(())
    }

    async fn after_routes(router: Router<Context>, ctx: &Context) -> Result<Router<Context>> {
        Ok(crate::assets::mount(router, ctx))
    }

    fn register_tasks(_tasks: &mut Tasks) {}

    fn middlewares(ctx: &Context) -> MiddlewareStack {
        let mut stack = default_middleware_stack(ctx);
        stack.replace(
            "session_auth",
            Box::new(session_auth::SessionAuthLayer::new(
                crate::services::auth::AuthBackend::new(ctx.db.clone()),
                ctx,
            )),
        );
        stack.replace(
            "authn",
            Box::new(authn::RequireAuth::<crate::services::auth::AuthBackend>::new(
                authn::Config {
                    enable: true,
                    public_paths: vec![
                        "/".into(),
                        "/login".into(),
                        "/register".into(),
                        "/assets".into(),
                    ],
                    redirect: Some(crate::routes::auth::LOGIN.path.to_string()),
                    status: 401,
                },
            )),
        );
        stack
    }

    async fn initializers(_ctx: &Context) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(
            initializers::view_engine::ViewEngineInitializer,
        )])
    }

    fn export_payloads() -> Result<()> {
        use ts_rs::TS;

        use crate::controllers::{
            auth::{LoginProps, RegisterProps},
            dashboard::DashboardProps,
            shared::{AuthProps, FlashProps, SafeUser, SharedProps},
            welcome::WelcomeProps,
        };

        AuthProps::export().map_err(|error| Error::Message(error.to_string()))?;
        DashboardProps::export().map_err(|error| Error::Message(error.to_string()))?;
        FlashProps::export().map_err(|error| Error::Message(error.to_string()))?;
        LoginProps::export().map_err(|error| Error::Message(error.to_string()))?;
        RegisterProps::export().map_err(|error| Error::Message(error.to_string()))?;
        SafeUser::export().map_err(|error| Error::Message(error.to_string()))?;
        SharedProps::export().map_err(|error| Error::Message(error.to_string()))?;
        WelcomeProps::export().map_err(|error| Error::Message(error.to_string()))?;
        Ok(())
    }
}
