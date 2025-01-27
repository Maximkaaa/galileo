/// Messenger used to notify application when the map requires update.
pub trait Messenger: Send + Sync {
    /// Notifies the application that the map requires an update.
    fn request_redraw(&self);
}

/// Empty struct used for generic disambiguation.
pub struct DummyMessenger {}
impl Messenger for DummyMessenger {
    fn request_redraw(&self) {
        // do nothing
    }
}
