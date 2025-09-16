// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use tokio::sync::broadcast;
use uuid::Uuid;

use tatuin_core::types::ArcRwLock;

pub struct AsyncJobStorage {
    jobs: HashMap<Uuid, AsyncJob>,
    tx: broadcast::Sender<()>,
}

impl AsyncJobStorage {
    pub fn add(&mut self, j: AsyncJob) {
        self.jobs.insert(j.id, j);
        let _ = self.tx.send(());
    }

    pub fn remove(&mut self, id: &Uuid) {
        self.jobs.remove(id);
        let _ = self.tx.send(());
    }

    pub fn jobs(&self) -> Vec<String> {
        self.jobs.values().map(|p| p.name()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn subscribe_on_changes(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }
}

impl Default for AsyncJobStorage {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self {
            jobs: HashMap::new(),
            tx,
        }
    }
}

#[derive(Clone)]
pub struct AsyncJob {
    id: Uuid,
    name: String,
    jobs: ArcRwLock<AsyncJobStorage>,
}

impl AsyncJob {
    pub async fn new(name: &str, jobs: ArcRwLock<AsyncJobStorage>) -> Self {
        let s = Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            jobs,
        };
        s.jobs.write().await.add(s.clone());
        s
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl Drop for AsyncJob {
    fn drop(&mut self) {
        let id = self.id;
        let jobs = self.jobs.clone();
        tokio::task::spawn(async move { jobs.write().await.remove(&id) });
    }
}
