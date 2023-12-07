use maybe_sync::{MaybeSend, MaybeSync};

pub trait Messenger: MaybeSend + MaybeSync {
    fn request_redraw(&self);
}
