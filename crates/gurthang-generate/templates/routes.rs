use gurthang_http::Route;
{% for route in routes %}
pub const {{ route.ident }}: Route = Route {
    name: "{{ route.name }}",
    method: "{{ route.method }}",
    path: "{{ route.path }}",
};
{% endfor %}
