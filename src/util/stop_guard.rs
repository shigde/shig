use tokio_util::sync::CancellationToken;

pub struct StopGuard(pub CancellationToken);

impl Drop for StopGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}