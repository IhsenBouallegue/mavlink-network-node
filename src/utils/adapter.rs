use embedded_hal_02::blocking;

pub struct BlockingAsync<T> {
    wrapped: T,
}

impl<T> BlockingAsync<T> {
    /// Create a new instance of a wrapper for a given peripheral.
    pub fn new(wrapped: T) -> Self {
        Self { wrapped }
    }
}
impl<T, E> embedded_hal_async::spi::ErrorType for BlockingAsync<T>
where
    E: embedded_hal::spi::Error,
    T: blocking::spi::Transfer<u8, Error = E> + blocking::spi::Write<u8, Error = E>,
{
    type Error = E;
}

impl<T, E> embedded_hal_async::spi::SpiBus<u8> for BlockingAsync<T>
where
    E: embedded_hal::spi::Error + 'static,
    T: blocking::spi::Transfer<u8, Error = E> + blocking::spi::Write<u8, Error = E>,
{
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.wrapped.write(data)?;
        Ok(())
    }

    async fn read(&mut self, data: &mut [u8]) -> Result<(), Self::Error> {
        self.wrapped.transfer(data)?;
        Ok(())
    }

    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        for i in 0..core::cmp::min(read.len(), write.len()) {
            read[i] = write[i].clone();
        }
        self.wrapped.transfer(read)?;
        Ok(())
    }

    async fn transfer_in_place(&mut self, data: &mut [u8]) -> Result<(), Self::Error> {
        self.wrapped.transfer(data)?;
        Ok(())
    }
}
