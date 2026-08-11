use std::sync::{Arc, RwLock};

use axum::response::Html;
use serde::Serialize;

use crate::error::{AppError, Result};

#[derive(Clone)]
pub struct TeraEngine(Arc<TeraEngineInner>);

struct TeraEngineInner {
    tera: RwLock<tera::Tera>,
    glob: Option<String>,
}

impl TeraEngine {
    pub fn load(glob: &str) -> Result<Self> {
        Ok(Self(Arc::new(TeraEngineInner {
            tera: RwLock::new(tera::Tera::new(glob)?),
            glob: Some(glob.to_owned()),
        })))
    }

    pub fn render(&self, template: &str, values: &impl Serialize) -> Result<Html<String>> {
        let context = tera::Context::from_serialize(values)?;
        let tera = self.0.tera.read().map_err(|_| AppError::Internal)?;
        Ok(Html(tera.render(template, &context)?))
    }

    pub fn render_to_string(&self, template: &str, values: &impl Serialize) -> Result<String> {
        let context = tera::Context::from_serialize(values)?;
        let tera = self.0.tera.read().map_err(|_| AppError::Internal)?;
        Ok(tera.render(template, &context)?)
    }

    pub fn reload(&self) -> Result<()> {
        let Some(glob) = &self.0.glob else {
            return Ok(());
        };
        let replacement = tera::Tera::new(glob)?;
        *self.0.tera.write().map_err(|_| AppError::Internal)? = replacement;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn from_tera(tera: tera::Tera) -> Self {
        Self(Arc::new(TeraEngineInner {
            tera: RwLock::new(tera),
            glob: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_keeps_html_escaping_enabled() {
        let mut tera = tera::Tera::default();
        tera.add_raw_template("test.html", "{{ value }}").unwrap();
        let engine = TeraEngine::from_tera(tera);
        let Html(output) = engine
            .render(
                "test.html",
                &serde_json::json!({
                    "value": "<script>alert(1)</script>"
                }),
            )
            .unwrap();
        assert_eq!(output, "&lt;script&gt;alert(1)&lt;&#x2F;script&gt;");
    }

    #[test]
    fn reload_keeps_the_last_valid_templates() {
        let directory = tempfile::tempdir().unwrap();
        let template = directory.path().join("page.html");
        std::fs::write(&template, "first").unwrap();
        let glob = format!("{}/*.html", directory.path().display());
        let engine = TeraEngine::load(&glob).unwrap();
        let context = serde_json::json!({});

        assert_eq!(
            engine.render_to_string("page.html", &context).unwrap(),
            "first"
        );

        std::fs::write(&template, "second").unwrap();
        engine.reload().unwrap();
        assert_eq!(
            engine.render_to_string("page.html", &context).unwrap(),
            "second"
        );

        std::fs::write(&template, "{% if %}").unwrap();
        assert!(engine.reload().is_err());
        assert_eq!(
            engine.render_to_string("page.html", &context).unwrap(),
            "second"
        );
    }
}
