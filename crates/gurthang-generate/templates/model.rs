#[derive(Clone, Debug)]
pub struct {{ pascal }} {
{%- for field in struct_fields %}
    pub {{ field.name }}: {{ field.ty }},
{%- endfor %}
}

#[derive(Clone, Debug)]
pub struct Create{{ pascal }}Data {
{%- for field in data_fields %}
    pub {{ field.name }}: {{ field.ty }},
{%- endfor %}
}

#[derive(Clone, Debug)]
pub struct Update{{ pascal }}Data {
{%- for field in data_fields %}
    pub {{ field.name }}: {{ field.ty }},
{%- endfor %}
}

impl {{ pascal }} {
{%- if has_pk %}
    pub async fn find(pool: &sqlx::PgPool, id: {{ pk_type }}) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM {{ table }} WHERE {{ pk_name }} = $1",
            id
        )
        .fetch_optional(pool)
        .await
    }

{%- endif %}
    pub async fn list(pool: &sqlx::PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM {{ table }} ORDER BY 1"
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(pool: &sqlx::PgPool, data: Create{{ pascal }}Data) -> sqlx::Result<Self> {
{%- if generate_uuid %}
        let id = uuid::Uuid::new_v4();
{%- endif %}
        let now = chrono::Utc::now();
        sqlx::query_as!(
            Self,
            "INSERT INTO {{ table }} ({{ insert_columns }}) VALUES ({{ insert_placeholders }}) RETURNING *",
{%- for bind in insert_binds %}
            {{ bind }},
{%- endfor %}
        )
        .fetch_one(pool)
        .await
    }
{%- if has_pk %}

    pub async fn update(pool: &sqlx::PgPool, id: {{ pk_type }}, data: Update{{ pascal }}Data) -> sqlx::Result<Self> {
        let now = chrono::Utc::now();
        sqlx::query_as!(
            Self,
            "UPDATE {{ table }} SET {{ update_sets }} WHERE {{ pk_name }} = ${{ update_pk_index }} RETURNING *",
{%- for bind in update_binds %}
            {{ bind }},
{%- endfor %}
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: {{ pk_type }}) -> sqlx::Result<u64> {
        Ok(sqlx::query!(
            "DELETE FROM {{ table }} WHERE {{ pk_name }} = $1",
            id
        )
        .execute(pool)
        .await?
        .rows_affected())
    }
{%- endif %}
}
