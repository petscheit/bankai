use tokio::sync::broadcast;

lazy_static::lazy_static! {
    static ref SEMAPHORE_EVENT: broadcast::Sender<String> = broadcast::channel(10).0;
}

pub fn subscribe_to_semaphore_events() -> broadcast::Receiver<String> {
    SEMAPHORE_EVENT.subscribe()
}
