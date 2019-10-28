pub mod memory;
pub mod lcd;
pub mod sound;
pub mod util;

use memory::GbaMemory;
use pyrite_arm::ArmCpu;
use lcd::GbaLCD;

pub struct Gba {
    pub memory: GbaMemory,
    pub cpu: ArmCpu,
    lcd: GbaLCD,
}

impl Gba {
    pub fn new() -> Gba {
        Gba {
            memory: GbaMemory::new(true),
            cpu: ArmCpu::new(),
            lcd: GbaLCD::new(),
        }
    }

    pub fn reset(&mut self, skip_bios: bool) {
        use pyrite_arm::registers;
        self.memory.init();
        self.cpu.registers.setf_f(); // Disables FIQ interrupts (always high on the GBA)

        if skip_bios {
            self.cpu.set_pc(0x08000000, &mut self.memory);
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            self.cpu.registers.write_mode(registers::CpuMode::System);
            self.cpu.registers.write_with_mode(registers::CpuMode::User, 13, 0x03007F00); // Also System
            self.cpu.registers.write_with_mode(registers::CpuMode::IRQ, 13, 0x03007FA0);
            self.cpu.registers.write_with_mode(registers::CpuMode::Supervisor, 13, 0x03007FE0);
        } else {
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            self.cpu.set_pc(0x00000000, &mut self.memory);
            self.cpu.registers.write_mode(registers::CpuMode::Supervisor);
        }

        self.memory.ioregs.keyinput.inner = 0x3FF;
        // @TODO some more IO registers need to be set here.
    }

    pub fn set_rom(&mut self, rom: Vec<u8>) {
        self.memory.set_gamepak_rom(rom);
    }

    pub fn init(&mut self, video: &mut dyn GbaVideoOutput, _audio: &mut dyn GbaAudioOutput) {
        self.lcd.init(&mut self.cpu, &mut self.memory, video);
    }

    #[inline]
    pub fn step(&mut self, video: &mut dyn GbaVideoOutput, _audio: &mut dyn GbaAudioOutput) {
        let cycles = self.cpu.step(&mut self.memory);
        self.lcd.step(cycles, &mut self.cpu, &mut self.memory, video);
    }

    #[inline]
    pub fn is_frame_ready(&self) -> bool {
        self.lcd.end_of_frame
    }

    pub fn set_key_pressed(&mut self, key: KeypadInput, pressed: bool) {
        // 0 = Pressed, 1 = Released
        if pressed {
            self.memory.ioregs.keyinput.inner &= !key.mask();
        } else {
            self.memory.ioregs.keyinput.inner |= key.mask();
        }
    }

    pub fn is_key_pressed(&mut self, key: KeypadInput) -> bool {
        (self.memory.ioregs.keyinput.inner & (key.mask())) == 0
    }
}

#[derive(Clone, Copy)]
#[repr(u16)]
pub enum KeypadInput {
    ButtonA = 0,
    ButtonB = 1,
    Select  = 2,
    Start   = 3,
    Right   = 4,
    Left    = 5,
    Up      = 6,
    Down    = 7,
    ButtonR = 8,
    ButtonL = 9,
}

impl KeypadInput {
    fn mask(self) -> u16 {
        1 << (self as u16)
    }
}

pub trait GbaVideoOutput {
    /// Called at the beginning of line 0 to signal the start of a new frame.
    fn pre_frame(&mut self);

    /// Called after the last line has been drawn to signal the end of a frame.
    fn post_frame(&mut self);

    /// Called by the LCD every time a line is ready to be committed to the video
    /// output somehow.
    fn display_line(&mut self, line: u32, pixels: &[u16]);
}

pub trait GbaAudioOutput {
    // @TODO Not sure how I want to do this one yet. Instead of having all of the samples
    //       generated by the GBA, I might just send the various states of the channels
    //       instead and have the audio output device (whatever it is) handle generating
    //       the output for each. But that would rely on whatever is on the otherside generating
    //       samples knowing a lot ofthings about the GBA's internals which is what I've been trying to
    //       avoid.
    //
    //       -- Marc C. [25 September, 2019]
    fn play_samples(&mut self);
}

pub struct NoVideoOutput;
pub struct NoAudioOutput;

impl GbaVideoOutput for NoVideoOutput {
    fn pre_frame(&mut self) { /* NOP */ }
    fn post_frame(&mut self) { /* NOP */ }

    fn display_line(&mut self, _line: u32, _pixels: &[u16]) {
        /* NOP */
    }
}

impl GbaAudioOutput for NoAudioOutput {
    fn play_samples(&mut self) {
        /* NOP */
    }
}
