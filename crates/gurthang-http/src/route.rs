use axum::{
    Router,
    handler::Handler,
    routing::{delete, get, patch, post, put},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Route {
    pub name: &'static str,
    pub method: &'static str,
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

pub trait AddRoute<S> {
    fn add_route<H, T>(self, route: Route, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static;
}

impl<S> AddRoute<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn add_route<H, T>(self, route: Route, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static,
    {
        match route.method {
            "GET" => self.route(route.path, get(handler)),
            "POST" => self.route(route.path, post(handler)),
            "PUT" => self.route(route.path, put(handler)),
            "PATCH" => self.route(route.path, patch(handler)),
            "DELETE" => self.route(route.path, delete(handler)),
            method => panic!("unsupported HTTP method {method} for route {}", route.name),
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
            method: "GET",
            path: "/widgets/{id}",
        };
        assert_eq!(route.url_with(7), "/widgets/7");
        assert_eq!(route.as_ref(), "/widgets/{id}");
    }
}
