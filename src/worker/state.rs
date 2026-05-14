use derive_more::Display;

#[derive(Debug, Clone, Display)]
pub enum WorkerState {
    Running,
    #[display(fmt = "Worker failed {}", _0)]
    Failed(String),
}
