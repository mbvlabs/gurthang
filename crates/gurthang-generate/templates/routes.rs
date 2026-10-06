use gurthang::Route;
{% for route in routes %}
pub const {{ route.ident }}: Route = Route {
    name: "{{ route.name }}",
    path: "{{ route.path }}",
};
{% endfor %}
