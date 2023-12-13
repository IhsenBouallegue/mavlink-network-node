use std::sync::{Arc, Mutex};

pub trait Driver<PacketType> {
    fn send(&self, get_packet_to_send: Arc<Mutex<impl Fn() -> Option<PacketType>>>);
    async fn receive(&self, on_receive: Arc<Mutex<impl Fn(PacketType)>>);
    fn create_instance() -> Self;
}
