use esp_hal::dma::DmaDescriptor;
use esp_hal::dma::DmaPriority;
use esp_hal::dma::ReadBuffer;
use esp_hal::gpio::AnyPin;
use esp_hal::gpio::DummyPin;
use esp_hal::gpio::ErasedPin;
use esp_hal::parl_io::BitPackOrder;
use esp_hal::parl_io::ClkOutPin;
use esp_hal::parl_io::ParlIoTx;
use esp_hal::parl_io::ParlIoTxOnly;
use esp_hal::parl_io::SampleEdge;
use esp_hal::parl_io::TxSixteenBits;
use esp_hal::peripherals::PARL_IO;

use crate::framebuffer::DmaFrameBuffer;
use crate::HertzU32;
use crate::Hub75Pins;

type Hub75TxSixteenBits<'d> = TxSixteenBits<
    'd,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    AnyPin<'d>,
    DummyPin,
    DummyPin,
    DummyPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
    ErasedPin,
>;

// TODO: make DMA channel a type parameter
use esp_hal::dma::DmaChannel0;
use static_cell::StaticCell;
pub struct Hub75<'d, DM: esp_hal::Mode> {
    parl_io: ParlIoTx<'d, DmaChannel0, DM>,
}

// impl<'d> Hub75<'d, esp_hal::Blocking> {
//     pub fn new(
//         parl_io: PARL_IO,
//         hub75_pins: Hub75Pins,
//         channel: esp_hal::dma::ChannelCreator<0>,
//         tx_descriptors: &'static mut [DmaDescriptor],
//     ) -> Self {
//         static PINS: StaticCell<Hub75TxSixteenBits<'static>> = StaticCell::new();
//         let pins = PINS.init(TxSixteenBits::new(
//             hub75_pins.addr0,
//             hub75_pins.addr1,
//             hub75_pins.addr2,
//             hub75_pins.addr3,
//             hub75_pins.addr4,
//             hub75_pins.latch,
//             AnyPin::new_inverted(hub75_pins.blank),
//             DummyPin::new(),
//             DummyPin::new(),
//             DummyPin::new(),
//             hub75_pins.red1,
//             hub75_pins.grn1,
//             hub75_pins.blu1,
//             hub75_pins.red2,
//             hub75_pins.grn2,
//             hub75_pins.blu2,
//         ));
//         static CLOCK_PIN: StaticCell<ClkOutPin<ErasedPin>> = StaticCell::new();
//         let clock_pin = CLOCK_PIN.init(ClkOutPin::new(hub75_pins.clock));
//         let parl_io: ParlIoTxOnly<DmaChannel0, esp_hal::Blocking> = ParlIoTxOnly::new(
//             parl_io,
//             channel.configure(false, DmaPriority::Priority0),
//             tx_descriptors,
//             15.MHz(),
//         )
//         .unwrap(); // TODO: handle error

//         let parl_io = parl_io
//             .tx
//             .with_config(pins, clock_pin, 0, SampleEdge::Normal, BitPackOrder::Msb)
//             .unwrap(); // TODO: handle error
//         Self { parl_io }
//     }
// }

impl<'d> Hub75<'d, esp_hal::Async> {
    pub fn new_async(
        parl_io: PARL_IO,
        hub75_pins: Hub75Pins, // TODO: how can we make this non-static?
        channel: esp_hal::dma::ChannelCreator<0>,
        tx_descriptors: &'static mut [DmaDescriptor],
        frequency: HertzU32,
    ) -> Self {
        // TODO: how can we make this non-static?
        static PINS: StaticCell<Hub75TxSixteenBits<'static>> = StaticCell::new();
        let pins = PINS.init(TxSixteenBits::new(
            hub75_pins.addr0,
            hub75_pins.addr1,
            hub75_pins.addr2,
            hub75_pins.addr3,
            hub75_pins.addr4,
            hub75_pins.latch,
            AnyPin::new_inverted(hub75_pins.blank),
            DummyPin::new(),
            DummyPin::new(),
            DummyPin::new(),
            hub75_pins.red1,
            hub75_pins.grn1,
            hub75_pins.blu1,
            hub75_pins.red2,
            hub75_pins.grn2,
            hub75_pins.blu2,
        ));
        // TODO: how can we make this non-static?
        static CLOCK_PIN: StaticCell<ClkOutPin<ErasedPin>> = StaticCell::new();
        let clock_pin = CLOCK_PIN.init(ClkOutPin::new(hub75_pins.clock));
        let parl_io = ParlIoTxOnly::new(
            parl_io,
            channel.configure_for_async(false, DmaPriority::Priority0),
            tx_descriptors,
            frequency,
        )
        .unwrap(); // TODO: handle error

        let parl_io = parl_io
            .tx
            .with_config(pins, clock_pin, 0, SampleEdge::Invert, BitPackOrder::Msb)
            .unwrap(); // TODO: handle error
        Self { parl_io }
    }

    pub async fn render_async<
        const ROWS: usize,
        const COLS: usize,
        const BITS: u8,
        const SIZE: usize,
    >(
        &mut self,
        fb: &DmaFrameBuffer<ROWS, COLS, BITS, SIZE>,
    ) {
        // parl_io has a max size limit of 32736 bytes so we need to send the
        // framebuffer in chunks
        let buffer = unsafe {
            let (ptr, len) = fb.read_buffer();
            core::slice::from_raw_parts(ptr, len)
        };
        for chunk in buffer.chunks(32736) {
            self.parl_io
                .write_dma_async(&chunk)
                .await
                .expect("render_async failed");
        }
    }
}
