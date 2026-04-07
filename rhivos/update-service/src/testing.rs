//! Shared test infrastructure: MockPodmanExecutor.

use crate::podman::{PodmanError, PodmanExecutor};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Records of calls made to the mock executor.
#[derive(Debug, Default, Clone)]
pub struct MockCalls {
    pub pull: Vec<String>,
    pub inspect_digest: Vec<String>,
    pub run: Vec<(String, String)>,
    pub stop: Vec<String>,
    pub rm: Vec<String>,
    pub rmi: Vec<String>,
    pub wait: Vec<String>,
}

#[derive(Debug, Clone)]
struct MockConfig {
    pull_result: Result<(), PodmanError>,
    inspect_result: Result<String, PodmanError>,
    run_result: Result<(), PodmanError>,
    stop_result: Result<(), PodmanError>,
    rm_result: Result<(), PodmanError>,
    rmi_result: Result<(), PodmanError>,
    wait_result: Result<i32, PodmanError>,
    /// Per-adapter overrides for stop results.
    stop_overrides: HashMap<String, Result<(), PodmanError>>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            pull_result: Ok(()),
            inspect_result: Ok("sha256:default".to_string()),
            run_result: Ok(()),
            stop_result: Ok(()),
            rm_result: Ok(()),
            rmi_result: Ok(()),
            wait_result: Ok(0),
            stop_overrides: HashMap::new(),
        }
    }
}

/// A mock podman executor that records calls and returns configurable results.
#[derive(Debug, Clone)]
pub struct MockPodmanExecutor {
    config: Arc<Mutex<MockConfig>>,
    calls: Arc<Mutex<MockCalls>>,
}

impl MockPodmanExecutor {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(MockConfig::default())),
            calls: Arc::new(Mutex::new(MockCalls::default())),
        }
    }

    // --- Configuration setters ---

    pub fn set_pull_result(&self, result: Result<(), PodmanError>) {
        self.config.lock().unwrap().pull_result = result;
    }

    pub fn set_inspect_result(&self, result: Result<String, PodmanError>) {
        self.config.lock().unwrap().inspect_result = result;
    }

    pub fn set_run_result(&self, result: Result<(), PodmanError>) {
        self.config.lock().unwrap().run_result = result;
    }

    pub fn set_stop_result(&self, result: Result<(), PodmanError>) {
        self.config.lock().unwrap().stop_result = result;
    }

    pub fn set_stop_result_for(&self, adapter_id: &str, result: Result<(), PodmanError>) {
        self.config
            .lock()
            .unwrap()
            .stop_overrides
            .insert(adapter_id.to_string(), result);
    }

    pub fn set_rm_result(&self, result: Result<(), PodmanError>) {
        self.config.lock().unwrap().rm_result = result;
    }

    pub fn set_rmi_result(&self, result: Result<(), PodmanError>) {
        self.config.lock().unwrap().rmi_result = result;
    }

    pub fn set_wait_result(&self, result: Result<i32, PodmanError>) {
        self.config.lock().unwrap().wait_result = result;
    }

    // --- Call recording accessors ---

    pub fn calls(&self) -> MockCalls {
        self.calls.lock().unwrap().clone()
    }

    pub fn pull_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().pull.clone()
    }

    pub fn inspect_digest_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().inspect_digest.clone()
    }

    pub fn run_calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().run.clone()
    }

    pub fn stop_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().stop.clone()
    }

    pub fn rm_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().rm.clone()
    }

    pub fn rmi_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().rmi.clone()
    }

    pub fn wait_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().wait.clone()
    }
}

#[async_trait]
impl PodmanExecutor for MockPodmanExecutor {
    async fn pull(&self, image_ref: &str) -> Result<(), PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .pull
            .push(image_ref.to_string());
        self.config.lock().unwrap().pull_result.clone()
    }

    async fn inspect_digest(&self, image_ref: &str) -> Result<String, PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .inspect_digest
            .push(image_ref.to_string());
        self.config.lock().unwrap().inspect_result.clone()
    }

    async fn run(&self, adapter_id: &str, image_ref: &str) -> Result<(), PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .run
            .push((adapter_id.to_string(), image_ref.to_string()));
        self.config.lock().unwrap().run_result.clone()
    }

    async fn stop(&self, adapter_id: &str) -> Result<(), PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .stop
            .push(adapter_id.to_string());
        let cfg = self.config.lock().unwrap();
        if let Some(result) = cfg.stop_overrides.get(adapter_id) {
            result.clone()
        } else {
            cfg.stop_result.clone()
        }
    }

    async fn rm(&self, adapter_id: &str) -> Result<(), PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .rm
            .push(adapter_id.to_string());
        self.config.lock().unwrap().rm_result.clone()
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .rmi
            .push(image_ref.to_string());
        self.config.lock().unwrap().rmi_result.clone()
    }

    async fn wait(&self, adapter_id: &str) -> Result<i32, PodmanError> {
        self.calls
            .lock()
            .unwrap()
            .wait
            .push(adapter_id.to_string());
        self.config.lock().unwrap().wait_result.clone()
    }
}

impl Default for MockPodmanExecutor {
    fn default() -> Self {
        Self::new()
    }
}
