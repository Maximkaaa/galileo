pub trait Messenger: Send + Sync {
    fn request_redraw(&self);
}
