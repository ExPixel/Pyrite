use super::obj::{render_objects, ObjectPriority};
use super::palette::GbaPalette;
use super::{
    apply_mosaic, BGControl, BGOffset, LCDLineBuffer, LCDRegisters, Layer, Pixel, WindowInfo,
};
use crate::hardware::{OAM, VRAM};
use crate::util::memory::read_u16_unchecked;

pub fn render_mode0(
    registers: &LCDRegisters,
    vram: &VRAM,
    oam: &OAM,
    pal: &GbaPalette,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    let object_priorities = ObjectPriority::sorted(oam);

    for priority in (0usize..=3).rev() {
        for bg_index in (0usize..=3).rev() {
            if !registers.dispcnt.display_layer(bg_index as u16) {
                continue;
            }
            if registers.bg_cnt[bg_index].priority() == priority as u16 {
                let first_target = registers.effects.is_first_target(bg_index as u16);
                let second_target = registers.effects.is_second_target(bg_index as u16);
                let textbg = TextBG::new(
                    registers.bg_cnt[bg_index],
                    registers.bg_ofs[bg_index],
                    registers.mosaic,
                    first_target,
                    second_target,
                );

                if registers.bg_cnt[bg_index].palette256() {
                    draw_text_bg_8bpp(
                        Layer::from_bg(bg_index as u16),
                        registers.line as u32,
                        &textbg,
                        vram,
                        pal,
                        pixels,
                        window_info,
                    );
                } else {
                    draw_text_bg_4bpp(
                        Layer::from_bg(bg_index as u16),
                        registers.line as u32,
                        &textbg,
                        vram,
                        pal,
                        pixels,
                        window_info,
                    );
                }
            }
        }

        if registers.dispcnt.display_layer(4) {
            render_objects(
                registers,
                object_priorities.objects_with_priority(priority),
                vram,
                oam,
                pal,
                pixels,
                window_info,
            );
        }
    }
}

pub fn render_mode1(
    registers: &LCDRegisters,
    vram: &VRAM,
    oam: &OAM,
    pal: &GbaPalette,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
}

pub fn render_mode2(
    registers: &LCDRegisters,
    vram: &VRAM,
    oam: &OAM,
    pal: &GbaPalette,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
}

fn map_tiles_to_screen(
    tx: usize,
    ty: usize,
    offset: usize,
    width: usize,
    height: usize,
    tiles: &mut [u16; 32],
) {
    // We can have at most two regions mapped into the tile array. They will either be two parts of
    // a single screen that wrapped around horizontally, or a part of two screens that the output
    // spans across.
    todo!();
}

pub fn draw_text_bg_4bpp(
    layer: Layer,
    line: u32,
    bg: &TextBG,
    vram: &VRAM,
    palette: &GbaPalette,
    dest: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    pub const BYTES_PER_TILE: u32 = 32;
    pub const BYTES_PER_LINE: u32 = 4;

    let start_scx = bg.xoffset & (bg.width - 1);
    let scy = if bg.mosaic_y > 0 {
        let original_scy = (bg.yoffset + line) & (bg.height - 1);
        original_scy - (original_scy % bg.mosaic_y)
    } else {
        (bg.yoffset + line) & (bg.height - 1)
    };
    let ty = scy % 8;

    let mut mdx = 0;
    let mut dx = 0;
    let mut pixel_buffer = [0u8; 240];

    while dx < 240 {
        let scx = start_scx + dx - mdx;

        mdx += 1;
        if mdx >= bg.mosaic_x {
            mdx = 0;
        }

        let tile_info_offset = bg.get_tile_info_offset(scx, scy);
        // @TODO I'm not sure if this condition is even possible given how screen areas are
        // addressed. I should check that later.
        if tile_info_offset >= 0x10000 {
            dx += 1;
            continue;
        }
        let tile_info = unsafe { read_u16_unchecked(vram, tile_info_offset as usize) };
        let tile_number = (tile_info & 0x3FF) as u32;
        let tile_palette = ((tile_info >> 12) & 0xF) as u8;
        let hflip = (tile_info & 0x400) != 0;
        let vflip = (tile_info & 0x800) != 0;

        let tx = if hflip { 7 - (scx % 8) } else { scx % 8 };
        let ty = if vflip { 7 - ty } else { ty };

        let tile_data_start = bg.char_base + (BYTES_PER_TILE * tile_number);
        let mut pixel_offset = tile_data_start + (ty * BYTES_PER_LINE) + tx / 2;
        if pixel_offset >= 0x10000 {
            dx += 1;
            continue;
        }

        // try to do 8 pixels at a time if possible:
        if bg.mosaic_x <= 1 && (scx % 8) == 0 && dx <= 232 {
            let pinc = if hflip { -1i32 as u32 } else { 1u32 };
            for _ in 0..4 {
                let palette_entry = vram[pixel_offset as usize];
                let lo_palette_entry = palette_entry & 0xF;
                let hi_palette_entry = palette_entry >> 4;
                pixel_buffer[dx as usize] = (tile_palette * 16) + lo_palette_entry;
                pixel_buffer[dx as usize + 1] = (tile_palette * 16) + hi_palette_entry;
                dx += 2;
                pixel_offset = pixel_offset.wrapping_add(pinc);
            }
        } else {
            let palette_entry = (vram[pixel_offset as usize] >> ((tx % 2) << 2)) & 0xF;
            pixel_buffer[dx as usize] = (tile_palette * 16) + palette_entry;
            dx += 1;
        }
    }

    let first_target_mask = if bg.first_target {
        Pixel::FIRST_TARGET
    } else {
        0
    };

    let second_target_mask = if bg.second_target {
        Pixel::SECOND_TARGET
    } else {
        0
    };

    let pixel_mask = Pixel::layer_mask(layer) | first_target_mask | second_target_mask;

    if !window_info.enabled {
        for x in 0..240 {
            let entry = pixel_buffer[x];
            if (entry & 0xF) == 0 {
                continue;
            }
            dest.push_pixel(x, Pixel(pixel_mask | (entry as u16)));
        }
    } else {
        for x in 0..240 {
            let entry = pixel_buffer[x];
            if (entry & 0xF) == 0 {
                continue;
            }
            if let Some(mask) = window_info.check_pixel(layer, x as u16, line as u16) {
                dest.push_pixel(x, Pixel(pixel_mask | (entry as u16)));
            }
        }
    }
}

pub fn draw_text_bg_8bpp(
    layer: Layer,
    line: u32,
    bg: &TextBG,
    vram: &VRAM,
    palette: &GbaPalette,
    dest: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    pub const BYTES_PER_TILE: u32 = 64;
    pub const BYTES_PER_LINE: u32 = 8;

    let start_scx = bg.xoffset & (bg.width - 1);
    let scy = if bg.mosaic_y > 0 {
        let original_scy = (bg.yoffset + line) & (bg.height - 1);
        original_scy - (original_scy % bg.mosaic_y)
    } else {
        (bg.yoffset + line) & (bg.height - 1)
    };
    let ty = scy % 8;

    let mut mdx = 0;
    let mut dx = 0;

    let mut pixel_buffer = [0u8; 240];

    while dx < 240 {
        let scx = start_scx + dx - mdx;

        mdx += 1;
        if mdx >= bg.mosaic_x {
            mdx = 0;
        }

        let tile_info_offset = bg.get_tile_info_offset(scx, scy);
        if tile_info_offset >= 0x10000 {
            dx += 1;
            continue;
        }
        let tile_info = unsafe { read_u16_unchecked(vram, tile_info_offset as usize) };
        let tile_number = (tile_info & 0x3FF) as u32;
        let tile_palette = ((tile_info >> 12) & 0xF) as u8;
        let hflip = (tile_info & 0x400) != 0;
        let vflip = (tile_info & 0x800) != 0;

        let tx = scx % 8; // not yet accounting for hflip
        let ty = if vflip { 7 - ty } else { ty };

        let tile_data_start = bg.char_base + (BYTES_PER_TILE * tile_number);
        let mut pixel_offset = tile_data_start + (ty * BYTES_PER_LINE); // without X offset
        if pixel_offset >= 0x10000 {
            dx += 1;
            continue;
        }

        // try to do 8 pixels at a time if possible:
        if bg.mosaic_x <= 1 && (scx % 8) == 0 && dx <= 232 {
            pixel_buffer[dx as usize..(dx as usize + 8)]
                .copy_from_slice(&vram[pixel_offset as usize..(pixel_offset as usize + 8)]);
            if hflip {
                pixel_buffer[dx as usize..(dx as usize + 8)].reverse();
            }
            dx += 8;
        } else {
            if hflip {
                pixel_offset += (7 - (scx % 8));
            } else {
                pixel_offset += scx % 8;
            }

            let palette_entry = vram[pixel_offset as usize];
            pixel_buffer[dx as usize] = palette_entry;
            dx += 1;
        }
    }

    let first_target_mask = if bg.first_target {
        Pixel::FIRST_TARGET
    } else {
        0
    };

    let second_target_mask = if bg.second_target {
        Pixel::SECOND_TARGET
    } else {
        0
    };

    let pixel_mask = Pixel::layer_mask(layer) | first_target_mask | second_target_mask;

    if !window_info.enabled {
        for x in 0..240 {
            let entry = pixel_buffer[x];
            if entry == 0 {
                continue;
            }
            dest.push_pixel(x, Pixel(pixel_mask | (entry as u16)));
        }
    } else {
        for x in 0..240 {
            let entry = pixel_buffer[x];
            if entry == 0 {
                continue;
            }
            if let Some(mask) = window_info.check_pixel(layer, x as u16, line as u16) {
                dest.push_pixel(x, Pixel(pixel_mask | (entry as u16)));
            }
        }
    }
}

pub struct TextBG {
    /// Base address of characters.
    char_base: u32,
    /// Base address for screens.
    screen_base: u32,

    xoffset: u32,
    yoffset: u32,
    width: u32,
    height: u32,

    mosaic_x: u32,
    mosaic_y: u32,

    first_target: bool,
    second_target: bool,
}

impl TextBG {
    const SIZES: [(u32, u32); 4] = [(256, 256), (512, 256), (256, 512), (512, 512)];

    pub fn new(
        control: BGControl,
        offset: BGOffset,
        reg_mosaic: super::Mosaic,
        first_target: bool,
        second_target: bool,
    ) -> TextBG {
        let (width, height) = TextBG::SIZES[control.screen_size() as usize];
        let mosaic = if control.mosaic() {
            reg_mosaic.bg
        } else {
            (0, 0)
        };

        TextBG {
            char_base: control.char_base_block() as u32 * 16 * 1024,
            screen_base: control.screen_base_block() as u32 * 2 * 1024,
            xoffset: offset.x as u32,
            yoffset: offset.y as u32,
            width: width,
            height: height,
            mosaic_x: mosaic.0 as u32,
            mosaic_y: mosaic.1 as u32,
            first_target: first_target,
            second_target: second_target,
        }
    }

    #[inline]
    fn get_tile_info_offset(&self, scx: u32, scy: u32) -> u32 {
        let area_y = scy % 256;
        let area_ty = area_y / 8;
        let scx = scx & (self.width - 1); // @NOTE: this relies on bg.width being a power of 2
        let area_idx = (scy / 256) * (self.width / 256) + (scx / 256);
        let area_x = scx % 256;
        let area_tx = area_x / 8;
        return self.screen_base + (area_idx * 2048) + ((area_ty * 32) + area_tx) * 2;
    }
}
