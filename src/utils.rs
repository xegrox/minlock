use std::sync::Arc;
use wayland_client::backend::ObjectData;

pub struct DummyObjectData;
impl ObjectData for DummyObjectData {
    fn event(
        self: Arc<Self>,
        _backend: &wayland_client::backend::Backend,
        _msg: wayland_client::backend::protocol::Message<wayland_client::backend::ObjectId, std::os::fd::OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        // Do nothing
        None
    }

    fn destroyed(&self, _object_id: wayland_client::backend::ObjectId) {
        // Do nothing
    }
}