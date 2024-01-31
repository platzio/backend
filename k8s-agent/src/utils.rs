use tokio::time::Instant;
use tokio_stream::wrappers::IntervalStream;

pub(crate) fn create_interval_stream(duration: std::time::Duration) -> IntervalStream {
    let interval = tokio::time::interval_at(Instant::now() + duration, duration);
    IntervalStream::new(interval)
}
