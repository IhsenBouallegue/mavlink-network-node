pub trait Driver<PacketType> {
    fn create_instance() -> Self;
    async fn send(&self, packet_to_send: PacketType);
    async fn receive(&self) -> Option<PacketType>;
}
