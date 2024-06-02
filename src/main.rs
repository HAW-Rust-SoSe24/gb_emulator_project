#[cfg(test)]
pub mod test_helpers;

pub mod cpu;
pub mod memory;
pub mod rendering;

use std::{
    f32::consts::E,
    thread::sleep,
    time::{self, Duration},
};

use macroquad::{prelude::*, ui::root_ui};
use rendering::{
    line_rendering::{draw_pixels, oam_scan, PpuMode},
    tiles::*,
    views::*,
};

#[macro_use]
extern crate simple_log;

use crate::{
    cpu::registers::{Register16Bit, Register8Bit},
    rendering::utils::draw_scaled_text,
};

// Dots are PPU Cycle conters per Frame
const DOTS_PER_CPU_CYCLE: u32 = 4;
const DOTS_PER_LINE: u32 = 456;

#[macroquad::main("GB Emulator")]
async fn main() {
    simple_log::quick!();

    // 60Hz
    // This is the refresh rate of the Gameboy
    let time_per_frame: Duration = Duration::from_secs_f64(1.0 / 60.0);

    const PALETTE: [Color; 4] = [
        Color::new(1.00, 1.00, 1.00, 1.00),
        Color::new(0.18, 0.83, 0.18, 1.00),
        Color::new(0.12, 0.54, 0.12, 1.00),
        Color::new(0.06, 0.15, 0.06, 1.00),
    ];
    const SCALING: f32 = 4.0;

    let mut final_image = Image::gen_image_color(160, 144, GREEN);
    let mut gb_display = GbDisplay {
        offset_x: 5.0,
        offset_y: 5.0,
        scaling: SCALING,
    };
    let gb_display_size = gb_display.size(&final_image);

    let mut background_viewer = BackgroundViewer {
        offset_x: gb_display_size.x + 10.0,
        offset_y: 5.0,
        scaling: SCALING / 2.0,
    };
    let mut background_image = Image::gen_image_color(32 * 8, 32 * 8, PINK);
    let background_viewer_size = background_viewer.size();

    let mut tile_atlas = Image::gen_image_color(8 * 16, 8 * 24, WHITE);
    let mut tile_viewer = TileViewer {
        offset_x: gb_display_size.x + background_viewer_size.x + 15.0,
        offset_y: 5.0,
        scaling: SCALING,
    };
    let tile_viewer_size = tile_viewer.size();

    request_new_screen_size(
        background_viewer_size.x + tile_viewer_size.x + gb_display_size.x + 20.0,
        tile_viewer_size.y + 10.0,
    );

    let mut cpu = cpu::CPU::new(true);

    cpu.load_from_file("./game.gb", 0x0000);

    // Get start time
    let mut last_frame_time = time::Instant::now();
    let mut dump_time = time::Instant::now();
    let mut frame = 0;

    let mut scanline: u8 = 0;
    let mut frame_cycles = 0;
    let mut ppu_mode: PpuMode = PpuMode::OamScan;

    loop {
        let instruction = cpu.prepare_and_decode_next_instruction();
        log::debug!("🔠 Instruction: {:?}", instruction);
        let is_bootrom_enabled = cpu.is_boot_rom_enabled();
        let result = cpu.step();
        log::debug!("➡️ Result: {:?} | Bootrom: {:?}", result, is_bootrom_enabled);

        let pc_following_word = cpu
            .get_memory()
            .read_word(cpu.get_16bit_register(Register16Bit::PC) + 1);
        log::debug!("🔢 Following Word (PC): {:#06X}", pc_following_word);

        cpu.update_key_input();

        let dot = (frame_cycles) * DOTS_PER_CPU_CYCLE;
        cpu.set_lcd_y_coordinate(scanline);

        match ppu_mode {
            PpuMode::OamScan => {
                if dot % DOTS_PER_LINE == 80 - 1 {
                    oam_scan(&cpu);
                    ppu_mode = PpuMode::Drawing
                }
            }
            PpuMode::Drawing => {
                if dot % DOTS_PER_LINE == 172 + 80 - 1 {
                    draw_pixels(&cpu, &mut final_image);
                    ppu_mode = PpuMode::HorizontalBlank;
                }
            }
            PpuMode::HorizontalBlank => {
                if dot % DOTS_PER_LINE == 455 {
                    scanline += 1;
                    ppu_mode = if scanline <= 143 {
                        PpuMode::OamScan
                    } else {
                        PpuMode::VerticalBlank
                    }
                }
            }
            PpuMode::VerticalBlank => {
                if dot % DOTS_PER_LINE == 455 {
                    scanline += 1;
                    if scanline >= 154 {
                        ppu_mode = PpuMode::OamScan;
                        frame_cycles = 0;
                    }
                }
            }
        }

        cpu.set_ppu_mode(ppu_mode as u8);
        frame_cycles += 1;

        // Draw at 60Hz so 60 frames per second
        if dot >= DOTS_PER_LINE * 155 {
            while last_frame_time.elapsed() < time_per_frame {
                // Do nothing
                // TODO: Remove active wait
            }

            // Inform about the time it took to render the frame
            root_ui().label(
                None,
                format!(
                    "Frame time: {:?} | Target: {:?} | Frame: {:?}",
                    last_frame_time.elapsed(),
                    time_per_frame,
                    frame
                )
                .as_str(),
            );
            last_frame_time = time::Instant::now();

            // Update Debugging Views
            update_atlas_from_memory(&cpu, 16 * 24, &mut tile_atlas, &PALETTE);
            update_background_from_memory(&cpu, &mut background_image, &PALETTE, true, false);
            background_viewer.draw(&background_image);
            tile_viewer.draw(&tile_atlas);

            gb_display.draw(&final_image);
            next_frame().await;
            // Set the VBlank interrupt since we are done with the frame
            cpu.set_vblank_interrupt();
            frame += 1;

            // Dump memory every 3 seconds
            if dump_time.elapsed().as_secs() >= 3 {
                dump_time = time::Instant::now();
                cpu.dump_memory();
            }
        }
    }
}
