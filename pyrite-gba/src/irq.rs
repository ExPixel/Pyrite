pub struct GbaInterruptControl {
    /// (IME Register) Interrupt master enable bit
    pub(crate) master_enable: bool,

    /// Bits representing enabled interrupts. See `Interrupt`.
    pub(crate) enabled: u16,

    /// (IE Register) Request / Acknowledge interrupt bits.
    request_ack: u16,
}

impl GbaInterruptControl {
    pub fn new() -> GbaInterruptControl {
        GbaInterruptControl {
            master_enable: false,
            enabled: 0,
            request_ack: 0,
        }
    }

    pub(crate) fn read_if(&self) -> u16 {
        self.request_ack
    }

    /// Handles writing to the IF register. Writing a 1 to any bit in the IF register actually
    /// clears it.
    pub(crate) fn write_if(&mut self, value: u16) {
        self.request_ack &= !value;
    }

    /// Requests an interrupt. Returns true if the interrupt was enabled and the request was
    /// successfully made (IRQ request flag set).
    pub(crate) fn request(&mut self, interrupt: Interrupt) -> bool {
        if self.master_enable && self.is_enabled(interrupt) {
            self.request_ack |= interrupt.mask();
            return true;
        } else {
            return false;
        }
    }

    pub(crate) fn is_enabled(&self, interrupt: Interrupt) -> bool {
        (self.enabled & interrupt.mask()) != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Interrupt {
    LCDVBlank = 0,
    LCDHBlank = 1,
    LCDVCounterMatch = 2,
    Timer0Overflow = 3,
    Timer1Overflow = 4,
    Timer2Overflow = 5,
    Timer3Overflow = 6,
    SerialCommunication = 7,
    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Keypad = 12,
    GamePak = 13,
    None = 14,
}

impl Interrupt {
    #[inline]
    pub const fn mask(self) -> u16 {
        self as u16
    }

    pub fn timer(timer_index: usize) -> Interrupt {
        match timer_index {
            0 => Interrupt::Timer0Overflow,
            1 => Interrupt::Timer1Overflow,
            2 => Interrupt::Timer2Overflow,
            3 => Interrupt::Timer3Overflow,
            _ => Interrupt::None,
        }
    }

    pub fn dma(dma_index: usize) -> Interrupt {
        match dma_index {
            0 => Interrupt::DMA0,
            1 => Interrupt::DMA1,
            2 => Interrupt::DMA2,
            3 => Interrupt::DMA3,
            _ => Interrupt::None,
        }
    }
}