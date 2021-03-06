use super::{LCDLineBuffer, LCDRegisters, Layer, Pixel};
use crate::hardware::VRAM;
use crate::util::memory::read_u16_unchecked;

pub type Mode4FrameBuffer = [u8; 0x9600];
pub type Mode5FrameBuffer = [u8; 0xA000];

pub fn render_mode3(registers: &LCDRegisters, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    if registers.dispcnt.display_layer(Layer::BG2)
        && (!pixels.windows.enabled || pixels.windows.line_visible(Layer::BG2))
    {
        render_mode3_bitmap(
            registers.line as usize,
            vram,
            registers.effects.is_first_target(Layer::BG2),
            registers.effects.is_second_target(Layer::BG2),
            pixels,
        );
    }
}

fn render_mode3_bitmap(
    line: usize,
    vram: &VRAM,
    first_target: bool,
    second_target: bool,
    pixels: &mut LCDLineBuffer,
) {
    assert!(line < 160);

    let pflags = Pixel::layer_mask(Layer::BG2)
        | (if first_target { Pixel::FIRST_TARGET } else { 0 })
        | (if second_target {
            Pixel::SECOND_TARGET
        } else {
            0
        });

    let line_offset = 480 * line;
    for x in 0..240 {
        let pixel_metadata = if pixels.windows.enabled {
            if let Some(window_effects_mask) = pixels.windows.check_pixel(Layer::BG2, x) {
                Pixel(pflags & window_effects_mask)
            } else {
                continue;
            }
        } else {
            Pixel(pflags)
        };

        // Bounds check at the top of the function ensures that we never go above 75KB (max address
        // is actually 0x12BFE).  The compiler just doesn't seem to be able to optimize the checks
        // away here though.  Doing it this way removes bounds checks and allows auto vectorization :o
        let color = unsafe { read_u16_unchecked(vram, line_offset + x * 2) } | 0x8000;
        pixels.push_bitmap_pixel(x, pixel_metadata, color);
    }
}

pub fn render_mode4(registers: &LCDRegisters, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    const FRAMEBUFFER0_OFFSET: usize = 0x0000;
    const FRAMEBUFFER1_OFFSET: usize = 0xA000;
    const FRAMEBUFFER_SIZE: usize = 0x9600;

    if registers.dispcnt.display_layer(Layer::BG2)
        && (!pixels.windows.enabled || pixels.windows.line_visible(Layer::BG2))
    {
        let framebuffer_start = if registers.dispcnt.frame_select() == 0 {
            FRAMEBUFFER0_OFFSET
        } else {
            FRAMEBUFFER1_OFFSET
        };
        let framebuffer_end = framebuffer_start + FRAMEBUFFER_SIZE;
        assert!(vram.len() >= framebuffer_start && framebuffer_end <= vram.len());

        render_mode4_bitmap(
            registers.line as usize,
            unsafe { std::mem::transmute((&vram[framebuffer_start..framebuffer_end]).as_ptr()) },
            registers.effects.is_first_target(Layer::BG2),
            registers.effects.is_second_target(Layer::BG2),
            pixels,
        );
    }
}

fn render_mode4_bitmap(
    line: usize,
    framebuffer: &Mode4FrameBuffer,
    first_target: bool,
    second_target: bool,
    pixels: &mut LCDLineBuffer,
) {
    assert!(line < 160);

    let pflags = Pixel::layer_mask(Layer::BG2)
        | (if first_target { Pixel::FIRST_TARGET } else { 0 })
        | (if second_target {
            Pixel::SECOND_TARGET
        } else {
            0
        });

    let line_offset = 240 * line;
    for x in 0..240 {
        let pixel_metadata = if pixels.windows.enabled {
            if let Some(window_effects_mask) = pixels.windows.check_pixel(Layer::BG2, x) {
                pflags & window_effects_mask
            } else {
                continue;
            }
        } else {
            pflags
        };

        // Bounds check at the top of the function ensures that we never go above 75KB (max address
        // is actually 0x12BFE).  The compiler just doesn't seem to be able to optimize the checks
        // away here though.  Doing it this way removes bounds checks and allows auto vectorization :o
        let palette_entry = framebuffer[line_offset + x];
        if palette_entry != 0 {
            pixels.push_pixel(x, Pixel(pixel_metadata | palette_entry as u16));
        }
    }
}

pub fn render_mode5(registers: &LCDRegisters, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    const FRAMEBUFFER0_OFFSET: usize = 0x0000;
    const FRAMEBUFFER1_OFFSET: usize = 0xA000;
    const FRAMEBUFFER_SIZE: usize = 0xA000;

    if registers.dispcnt.display_layer(Layer::BG2)
        && (!pixels.windows.enabled || pixels.windows.line_visible(Layer::BG2))
    {
        let framebuffer_start = if registers.dispcnt.frame_select() == 0 {
            FRAMEBUFFER0_OFFSET
        } else {
            FRAMEBUFFER1_OFFSET
        };
        let framebuffer_end = framebuffer_start + FRAMEBUFFER_SIZE;
        assert!(vram.len() >= framebuffer_start && framebuffer_end <= vram.len());

        if registers.line < 128 {
            render_mode5_bitmap(
                registers.line as usize,
                unsafe {
                    std::mem::transmute((&vram[framebuffer_start..framebuffer_end]).as_ptr())
                },
                registers.effects.is_first_target(Layer::BG2),
                registers.effects.is_second_target(Layer::BG2),
                pixels,
            );
        }
    }
}

fn render_mode5_bitmap(
    line: usize,
    framebuffer: &Mode5FrameBuffer,
    first_target: bool,
    second_target: bool,
    pixels: &mut LCDLineBuffer,
) {
    assert!(line < 160);

    let pflags = Pixel::layer_mask(Layer::BG2)
        | (if first_target { Pixel::FIRST_TARGET } else { 0 })
        | (if second_target {
            Pixel::SECOND_TARGET
        } else {
            0
        });

    let line_offset = 480 * line;
    for x in 0..160 {
        let pixel_metadata = if pixels.windows.enabled {
            if let Some(window_effects_mask) = pixels.windows.check_pixel(Layer::BG2, x) {
                Pixel(pflags & window_effects_mask)
            } else {
                continue;
            }
        } else {
            Pixel(pflags)
        };

        // Bounds checks are basically done at the top of the function using the asserts.
        // This ensures that we never go above 75KB (max address is actually 0x12BFE).
        // The compiler just doesn't seem to be able to optimize the checks away here though.
        // Doing it this way removes bounds checks and allows auto vectorization :o
        let color = unsafe { read_u16_unchecked(framebuffer, line_offset + x * 2) } | 0x8000;
        pixels.push_bitmap_pixel(x, pixel_metadata, color);
    }
}
