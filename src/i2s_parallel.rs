use esp_hal::dma::DmaChannelFor;
use esp_hal::dma::DmaDescriptor;
use esp_hal::dma::DmaError;
use esp_hal::dma::DmaTxBuf;
use esp_hal::gpio::NoPin;
use esp_hal::i2s::AnyI2s;
use esp_hal::i2s::parallel::I2sParallel;
use esp_hal::i2s::parallel::I2sParallelTransfer;
use esp_hal::i2s::parallel::TxSixteenBits;
use esp_hal::peripheral::Peripheral;
use esp_hal::time::Rate;

use crate::framebuffer::DmaFrameBuffer;
use crate::Hub75Error;
use crate::Hub75Pins;

pub struct Hub75<'d, DM: esp_hal::DriverMode> {
    i2s: I2sParallel<'d, DM>,
    tx_descriptors: &'static mut [DmaDescriptor],
}

impl<'d> Hub75<'d, esp_hal::Blocking> {
    pub fn new<CH>(
        i2s: impl Peripheral<P = AnyI2s> + 'd,
        hub75_pins: Hub75Pins,
        channel: impl Peripheral<P = CH> + 'd,
        tx_descriptors: &'static mut [DmaDescriptor],
        frequency: Rate,
    ) -> Result<Self, Hub75Error>
    where
        CH: DmaChannelFor<AnyI2s>,
    {
        let (_, blank) = hub75_pins.blank.split();
        let pins = TxSixteenBits::new(
            hub75_pins.addr0,
            hub75_pins.addr1,
            hub75_pins.addr2,
            hub75_pins.addr3,
            hub75_pins.addr4,
            hub75_pins.latch,
            NoPin,
            NoPin,
            blank.inverted(),
            hub75_pins.red1,
            hub75_pins.grn1,
            hub75_pins.blu1,
            hub75_pins.red2,
            hub75_pins.grn2,
            hub75_pins.blu2,
            NoPin,
        );

        let i2s = I2sParallel::new(i2s, channel, frequency, pins, hub75_pins.clock);
        Ok(Self {
            i2s,
            tx_descriptors,
        })
    }

    pub fn into_async(self) -> Hub75<'d, esp_hal::Async> {
        Hub75 {
            i2s: self.i2s.into_async(),
            tx_descriptors: self.tx_descriptors,
        }
    }
}

impl<'d, DM: esp_hal::DriverMode> Hub75<'d, DM> {
    pub fn render<
        const ROWS: usize,
        const COLS: usize,
        const NROWS: usize,
        const BITS: u8,
        const FRAME_COUNT: usize,
    >(
        self,
        fb: &mut DmaFrameBuffer<ROWS, COLS, NROWS, BITS, FRAME_COUNT>,
    ) -> Result<Hub75Transfer<'d, DM>, (Hub75Error, Self)> {
        let i2s = self.i2s;
        let tx_descriptors = self.tx_descriptors;
        let tx_buffer = unsafe {
            use esp_hal::dma::ReadBuffer;
            let (ptr, len) = fb.read_buffer();
            // SAFETY: tx_buffer is only used until the tx_buf.split below!
            core::slice::from_raw_parts_mut(ptr as *mut u8, len)
        };
        let tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).expect("DmaTxBuf::new failed");
        let xfer = i2s.send(tx_buf).map_err(|(e, i2s, buf)| {
            let (tx_descriptors, _) = buf.split();
            (
                e.into(),
                Self {
                    i2s,
                    tx_descriptors,
                },
            )
        })?;
        Ok(Hub75Transfer { xfer })
    }
}

pub struct Hub75Transfer<'d, DM: esp_hal::DriverMode> {
    pub xfer: I2sParallelTransfer<'d, DmaTxBuf, DM>,
}

impl<'d, DM: esp_hal::DriverMode> Hub75Transfer<'d, DM> {
    /// Returns true when [Self::wait] will not block.
    pub fn is_done(&self) -> bool {
        self.xfer.is_done()
    }

    /// Waits for the transfer to finish and returns the peripheral and buffer.
    ///
    /// Note: This also clears the transfer interrupt so it can be used in
    /// interrupt handlers to "handle" the interrupt.
    pub fn wait(self) -> (Result<(), DmaError>, Hub75<'d, DM>) {
        let (i2s, tx_buf) = self.xfer.wait();
        let (tx_descriptors, _) = tx_buf.split();
        (
            Ok(()),
            Hub75 {
                i2s,
                tx_descriptors,
            },
        )
    }
}

impl Hub75Transfer<'_, esp_hal::Async> {
    pub async fn wait_for_done(&mut self) -> Result<(), DmaError> {
        self.xfer.wait_for_done().await
    }
}
