#[derive(Debug, Clone)]
pub enum WorkerState {
    Running,
    Failed(String),
}