use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{
    app::Context,
    error::{Error, Result},
};

pub struct TaskInfo {
    pub name: String,
    pub detail: String,
}

#[async_trait]
pub trait Task: Send + Sync {
    fn info(&self) -> TaskInfo;
    async fn run(&self, ctx: &Context, vars: &BTreeMap<String, String>) -> Result<()>;
}

#[derive(Default)]
pub struct Tasks {
    tasks: BTreeMap<String, Box<dyn Task>>,
}

impl Tasks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, task: impl Task + 'static) {
        let info = task.info();
        self.tasks.insert(info.name, Box::new(task));
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn names(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    pub fn list_lines(&self) -> Vec<String> {
        self.tasks
            .values()
            .map(|task| {
                let info = task.info();
                if info.detail.is_empty() {
                    info.name
                } else {
                    format!("{:<24} {}", info.name, info.detail)
                }
            })
            .collect()
    }

    pub async fn run(
        &self,
        name: &str,
        ctx: &Context,
        vars: &BTreeMap<String, String>,
    ) -> Result<()> {
        let task = self.tasks.get(name).ok_or_else(|| {
            Error::Message(format!(
                "unknown task {name:?}; run with no name to list tasks"
            ))
        })?;
        task.run(ctx, vars).await
    }
}
