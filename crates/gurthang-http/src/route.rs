#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Route {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
}
