// gurthang:generated:start
use crate::{{ snake }}::{Create{{ pascal }}Data, {{ pascal }}};

#[derive(Clone, Debug)]
pub struct {{ pascal }}Factory {
{%- for field in fields %}
    {{ field.name }}: {{ field.ty }},
{%- endfor %}
}

impl Default for {{ pascal }}Factory {
    fn default() -> Self {
        Self {
{%- for field in fields %}
            {{ field.name }}: {{ field.default }},
{%- endfor %}
        }
    }
}

impl {{ pascal }}Factory {
    pub fn new() -> Self {
        Self::default()
    }
{%- for field in fields %}

    pub fn {{ field.name }}(mut self, {{ field.name }}: impl Into<{{ field.ty }}>) -> Self {
        self.{{ field.name }} = {{ field.name }}.into();
        self
    }
{%- endfor %}

    pub fn data(self) -> Create{{ pascal }}Data {
        Create{{ pascal }}Data {
{%- for field in fields %}
            {{ field.name }}: self.{{ field.name }},
{%- endfor %}
        }
    }

    pub async fn create(self, pool: &sqlx::PgPool) -> sqlx::Result<{{ pascal }}> {
        {{ pascal }}::create(pool, self.data()).await
    }
}
// gurthang:generated:end
