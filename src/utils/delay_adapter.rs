use std::future::IntoFuture;

pub struct WithDelayNs<T> {
    wrapped: T,
}

impl<T> WithDelayNs<T> {
    pub fn new(wrapped: T) -> Self {
        Self { wrapped }
    }
}

impl<T> lora_phy::DelayNs for WithDelayNs<T>
where
    T: embedded_hal::delay::DelayNs,
{
    #[inline]
    async fn delay_ns(&mut self, ns: u32) {
        async { T::delay_ns(&mut self.wrapped, ns) }
            .into_future()
            .await;
    }
}
