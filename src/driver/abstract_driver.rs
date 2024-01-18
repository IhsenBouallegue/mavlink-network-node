pub trait Driver<PacketType>: Send + Sync {
    fn create_instance() -> Self
    where
        Self: Sized;

    async fn send(&self, packet_to_send: PacketType);
    async fn receive(&self) -> Option<PacketType>;
}
