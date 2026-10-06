use gurthang_http::Route;

pub const REGISTER: Route = Route {
    name: "register",
    method: "GET",
    path: "/register",
};

pub const REGISTER_CREATE: Route = Route {
    name: "register.create",
    method: "POST",
    path: "/register",
};

pub const LOGIN: Route = Route {
    name: "login",
    method: "GET",
    path: "/login",
};

pub const LOGIN_CREATE: Route = Route {
    name: "login.create",
    method: "POST",
    path: "/login",
};

pub const LOGOUT: Route = Route {
    name: "logout",
    method: "DELETE",
    path: "/logout",
};
