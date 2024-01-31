pub trait Messenger: Send + Sync {
    fn request_redraw(&self);
}

pub struct DummyMessenger {}
impl Messenger for DummyMessenger {
    fn request_redraw(&self) {
        // do nothing
    }
}
