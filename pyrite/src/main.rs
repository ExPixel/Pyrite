#[allow(dead_code)] mod platform;

use pyrite_gba::Gba;

fn main() {
    let exit_code = run_emulator();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run_emulator() -> i32 {
    let mut window = platform::glutin::window::Window::new("Pyrite", 480.0, 320.0);
    if let Err(_) = window.set_position_center() {
        log::error!("failed to place the window in the center of the screen");
    }
    let mut video = platform::opengl::PyriteGL::new();
    let mut no_audio = pyrite_gba::NoAudioOutput;

    let mut gba = Gba::new();

    const ROM_FILE: &str = "roms/tonc/m3_demo.gba";
    match load_binary(ROM_FILE) {
        Ok(rom_binary) => {
            gba.set_rom(rom_binary);
        },

        Err(err) => {
            eprintln!("error occurred while loading ROM ({}): {}", ROM_FILE, err);
            return 1;
        }
    }
    gba.reset(true);

    let mut fps_counter = FPSCounter::new();
    let mut title_buffer = "Pyrite (NO FPS)".to_string();
    while !window.close_requested() {
        if let Some(fps) = fps_counter.frame() {
            {
                use std::io::Write;
                title_buffer.clear();
                let mut cursor = std::io::Cursor::new(title_buffer.into_bytes());
                write!(&mut cursor, "Pyrite ({:.02} FPS)", fps).expect("failed to write title");
                title_buffer = unsafe { String::from_utf8_unchecked(cursor.into_inner()) };
            }
            window.set_title(&title_buffer);
        }

        window.handle_events();
        loop {
            gba.step(&mut video, &mut no_audio);
            if gba.is_frame_ready() { break }
        }
        video.render();
        window.flip();
    }

    return 0;
}

fn load_binary<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Vec<u8>> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut f = File::open(path)?;
    let mut binary = Vec::new();
    f.read_to_end(&mut binary)?;
    return Ok(binary);
}

struct FPSCounter {
    last_frame_start: std::time::Instant,
    frame_time_acc: std::time::Duration,
    frames_since_last_check: u32,
}

impl FPSCounter {
    pub fn new() -> FPSCounter {
        FPSCounter {
            last_frame_start: std::time::Instant::now(),
            frame_time_acc: std::time::Duration::from_millis(0),
            frames_since_last_check: 0,
        }
    }

    pub fn frame(&mut self) -> Option<f64> {
        let current_frame_start = std::time::Instant::now();
        let last_frame_time = current_frame_start.duration_since(self.last_frame_start);
        self.frame_time_acc += last_frame_time;
        self.last_frame_start = current_frame_start;

        let mut ret = None;
        if self.frame_time_acc.as_micros() >= 1000000 {
            let seconds = self.frame_time_acc.as_secs_f64() + (self.frame_time_acc.subsec_nanos() as f64 / 1000000000.0);
            let fps = (self.frames_since_last_check as f64) / seconds;
            self.frames_since_last_check = 0;
            self.frame_time_acc = std::time::Duration::from_millis(0);
            ret = Some(fps);
        }

        self.frames_since_last_check += 1;
        
        return ret;
    }
}
