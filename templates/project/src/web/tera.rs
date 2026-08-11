use std::sync::Arc;

use axum::response::Html;
use serde::Serialize;

use crate::error::Result;

#[derive(Clone)]
pub struct TeraEngine(Arc<tera::Tera>);

impl TeraEngine {
    pub fn load(glob: &str) -> Result<Self> {
        Ok(Self(Arc::new(tera::Tera::new(glob)?)))
    }

    pub fn render(&self, template: &str, values: &impl Serialize) -> Result<Html<String>> {
        let context = tera::Context::from_serialize(values)?;
        Ok(Html(self.0.render(template, &context)?))
    }

    pub fn render_to_string(&self, template: &str, values: &impl Serialize) -> Result<String> {
        let context = tera::Context::from_serialize(values)?;
        Ok(self.0.render(template, &context)?)
    }

    #[cfg(test)]
    pub(crate) fn from_tera(tera: tera::Tera) -> Self {
        Self(Arc::new(tera))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_keeps_html_escaping_enabled() {
        let mut tera = tera::Tera::default();
        tera.add_raw_template("test.html", "{{ value }}").unwrap();
        let engine = TeraEngine(Arc::new(tera));
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
}
