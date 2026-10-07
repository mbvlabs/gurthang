use std::{future::Future, pin::Pin};

use axum::{
    Router,
    extract::{FromRequest, FromRequestParts, Request},
    handler::Handler,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Route {
    pub name: &'static str,
    pub path: &'static str,
}

impl Route {
    pub fn url(&self) -> &'static str {
        self.path
    }

    pub fn url_with(&self, id: impl ToString) -> String {
        self.path.replace("{id}", &id.to_string())
    }
}

impl AsRef<str> for Route {
    fn as_ref(&self) -> &str {
        self.path
    }
}

#[derive(Clone)]
pub struct ControllerMethod<C, F> {
    controller: C,
    method: F,
}

pub fn on<C, F>(controller: C, method: F) -> ControllerMethod<C, F> {
    ControllerMethod { controller, method }
}

impl<C, F, Fut, Res, S> Handler<((), C), S> for ControllerMethod<C, F>
where
    C: Clone + Send + Sync + 'static,
    F: FnOnce(C) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
    S: Send + Sync + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, _req: Request, _state: S) -> Self::Future {
        Box::pin(async move { (self.method)(self.controller).await.into_response() })
    }
}

#[allow(non_snake_case, unused_mut)]
mod method_impls {
    use super::*;

    macro_rules! impl_controller_method {
        ([$($ty:ident),*], $last:ident) => {
            impl<C, F, Fut, S, Res, M, $($ty,)* $last> Handler<(M, C, $($ty,)* $last,), S>
                for ControllerMethod<C, F>
            where
                C: Clone + Send + Sync + 'static,
                F: FnOnce(C, $($ty,)* $last) -> Fut + Clone + Send + Sync + 'static,
                Fut: Future<Output = Res> + Send,
                S: Send + Sync + 'static,
                Res: IntoResponse,
                $($ty: FromRequestParts<S> + Send,)*
                $last: FromRequest<S, M> + Send,
            {
                type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

                fn call(self, req: Request, state: S) -> Self::Future {
                    Box::pin(async move {
                        let (mut parts, body) = req.into_parts();
                        $(
                            let $ty = match $ty::from_request_parts(&mut parts, &state).await {
                                Ok(value) => value,
                                Err(rejection) => return rejection.into_response(),
                            };
                        )*
                        let req = Request::from_parts(parts, body);
                        let $last = match $last::from_request(req, &state).await {
                            Ok(value) => value,
                            Err(rejection) => return rejection.into_response(),
                        };
                        (self.method)(self.controller, $($ty,)* $last)
                            .await
                            .into_response()
                    })
                }
            }
        };
    }

    impl_controller_method!([], T1);
    impl_controller_method!([T1], T2);
    impl_controller_method!([T1, T2], T3);
    impl_controller_method!([T1, T2, T3], T4);
    impl_controller_method!([T1, T2, T3, T4], T5);
    impl_controller_method!([T1, T2, T3, T4, T5], T6);
    impl_controller_method!([T1, T2, T3, T4, T5, T6], T7);
    impl_controller_method!([T1, T2, T3, T4, T5, T6, T7], T8);
}

/// A catalog [`Route`] bound to a verb and controller method.
pub struct BoundRoute<S> {
    apply: Box<dyn FnOnce(Router<S>) -> Router<S> + Send>,
}

/// A set of [`BoundRoute`]s that can take middleware once (one sub-router wrap).
pub struct RouteGroup<S> {
    router: Router<S>,
}

impl<S> RouteGroup<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    pub fn from_router(router: Router<S>) -> Self {
        Self { router }
    }

    pub fn add(mut self, bound: BoundRoute<S>) -> Self {
        self.router = (bound.apply)(self.router);
        self
    }

    pub fn into_router(self) -> Router<S> {
        self.router
    }
}

impl<S> Default for RouteGroup<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! bind_verb {
    ($name:ident) => {
        pub fn $name<C, F, T, S>(self, controller: C, handler: F) -> BoundRoute<S>
        where
            C: Clone + Send + Sync + 'static,
            F: Clone + Send + Sync + 'static,
            ControllerMethod<C, F>: Handler<T, S>,
            T: 'static,
            S: Clone + Send + Sync + 'static,
        {
            let path = self.path;
            BoundRoute {
                apply: Box::new(move |router: Router<S>| {
                    router.route(path, axum::routing::$name(on(controller, handler)))
                }),
            }
        }
    };
}

impl Route {
    bind_verb!(get);
    bind_verb!(post);
    bind_verb!(put);
    bind_verb!(patch);
    bind_verb!(delete);
}

#[macro_export]
macro_rules! mount {
    ($app:ident, { $($route:expr => $verb:ident($ctrl:expr, $handler:expr)),+ $(,)? }) => {{
        let mut __router = ::axum::Router::new();
        $(
            __router = __router.route(
                $route.path,
                ::axum::routing::$verb($crate::on($ctrl.clone(), $handler)),
            );
        )+
        __router
    }};
}

pub trait AddRoute<S> {
    fn add_route<H, T>(self, route: Route, method: &'static str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static;
}

impl<S> AddRoute<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn add_route<H, T>(self, route: Route, method: &'static str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static,
    {
        match method {
            "GET" => self.route(route.path, get(handler)),
            "POST" => self.route(route.path, post(handler)),
            "PUT" => self.route(route.path, put(handler)),
            "PATCH" => self.route(route.path, patch(handler)),
            "DELETE" => self.route(route.path, delete(handler)),
            other => panic!("unsupported HTTP method {other} for route {}", route.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_with_fills_axum_id() {
        let route = Route {
            name: "widgets.show",
            path: "/widgets/{id}",
        };
        assert_eq!(route.url_with(7), "/widgets/7");
        assert_eq!(route.as_ref(), "/widgets/{id}");
    }

    #[test]
    fn mount_binds_controller_method() {
        #[derive(Clone)]
        struct Hello;

        impl Hello {
            async fn show(self) -> &'static str {
                "ok"
            }
        }

        let app = Hello;
        let _router: Router = crate::mount!(app, {
            Route {
                name: "root",
                path: "/",
            } => get(app, Hello::show),
        });
    }

    #[test]
    fn route_group_binds_controller_method() {
        #[derive(Clone)]
        struct Hello;

        impl Hello {
            async fn show(self) -> &'static str {
                "ok"
            }
        }

        let app = Hello;
        let _router: Router = RouteGroup::new()
            .add(
                Route {
                    name: "root",
                    path: "/",
                }
                .get(app, Hello::show),
            )
            .into_router();
    }
}
