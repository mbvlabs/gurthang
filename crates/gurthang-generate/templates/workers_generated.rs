use gurthang::jobs::{PerformJob, Result};
use gurthang::mailer::Email;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Job {
{%- for job in jobs %}
    {{ job.pascal }},
{%- endfor %}
    Mailer(Email),
}

impl PerformJob for Job {
    fn name(&self) -> &'static str {
        match self {
{%- for job in jobs %}
            Self::{{ job.pascal }} => "{{ job.snake }}",
{%- endfor %}
            Self::Mailer(_) => "mailer",
        }
    }

    async fn perform(self, database: &PgPool) -> Result<()> {
        match self {
{%- for job in jobs %}
            Self::{{ job.pascal }} => {
                {{ job.snake }}::{{ job.pascal }}
                    .perform(database)
                    .await
            }
{%- endfor %}
            Self::Mailer(email) => gurthang::mailer::perform_job(email, database).await,
        }
    }
}

