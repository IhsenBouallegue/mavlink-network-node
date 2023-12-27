pub trait Driver<PacketType> {
    async fn send(&self, packet_to_send: PacketType);
    async fn receive(&self) -> Option<PacketType>;
    fn create_instance() -> Self;
}
