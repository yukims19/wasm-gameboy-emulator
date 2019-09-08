use log::info;
use log::Level;

mod utils;

use bit_vec::BitVec;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, OscillatorType};

const MAX_GAMEBOY_VOLUME: u8 = 0xf;
const PIXEL_ZOOM: u32 = 1;
const BACKGROUND_WIDTH: u32 = 255;
const BACKGROUND_HEIGHT: u32 = 255;
const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const BYTES_PER_TILE: usize = 16;
const BYTES_PER_8_PIXEL: usize = 2;

#[macro_use]
extern crate serde_derive;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

enum CycleRegister {
    CpuCycle,
    VramCycle,
}

#[wasm_bindgen]
struct Canvases {
    background_canvas: web_sys::CanvasRenderingContext2d,
    screen_canvas: web_sys::CanvasRenderingContext2d,
    char_map_canvas: web_sys::CanvasRenderingContext2d,
    char_map_debug_canvas: web_sys::CanvasRenderingContext2d,
    update_char_map_canvas_last_data: Vec<u8>,
}

#[wasm_bindgen]
impl Canvases {
    pub fn new() -> Canvases {
        let background_canvas = Canvases::make_canvas(
            "gameboy-background-canvas-rust",
            PIXEL_ZOOM * BACKGROUND_WIDTH,
            PIXEL_ZOOM * BACKGROUND_HEIGHT,
        );
        let screen_canvas = Canvases::make_canvas(
            "gameboy-screen-canvas-rust",
            PIXEL_ZOOM * SCREEN_WIDTH,
            PIXEL_ZOOM * SCREEN_HEIGHT,
        );

        let char_map_canvas = Canvases::make_canvas("char-map-actual-canvas-rust", 8, 1024);

        let char_map_debug_canvas =
            Canvases::make_canvas("char-map-debug-canvas-rust", 8 * 12, 8 * 8);

        let canvases = Canvases {
            background_canvas,
            screen_canvas,
            char_map_canvas,
            char_map_debug_canvas,
            update_char_map_canvas_last_data: Vec::new(),
        };

        canvases
    }

    pub fn render_background_map_1_as_image_data(&mut self, gameboy: &mut Gameboy) {
        // if !gameboy.is_vblank() {
        //     return;
        // }

        let background_map_1 = gameboy.background_map_1();

        //TODO: need to check which charmap is used
        let char_map_vec = gameboy.memory[0x8000..0x9000].to_vec();
        let mut char_map_tiles_bytes = Vec::new();

        //Get Tiles
        for idx in (0..char_map_vec.len()).step_by(BYTES_PER_TILE) {
            let tile_bytes = &char_map_vec[idx..idx + BYTES_PER_TILE];
            let mut image_data_source = Vec::new();
            //Get Tile Pixel rgba data
            for i in (0..tile_bytes.len()).step_by(BYTES_PER_8_PIXEL) {
                let low_bits = BitVec::from_bytes(&[tile_bytes[i]]);
                let high_bits = BitVec::from_bytes(&[tile_bytes[i + 1]]);

                for pixel_index in 0..8 {
                    let [r, g, b, a] = match (low_bits[pixel_index], high_bits[pixel_index]) {
                        (false, false) => [255, 255, 255, 255],
                        (false, true) => [191, 191, 191, 255],
                        (true, false) => [64, 64, 64, 255],
                        (true, true) => [0, 0, 0, 255],
                    };

                    image_data_source.push(r);
                    image_data_source.push(g);
                    image_data_source.push(b);
                    image_data_source.push(a);
                }
            }

            char_map_tiles_bytes.push(image_data_source);
        }

        //Clear context
        self.background_canvas.clear_rect(
            0.0,
            0.0,
            self.background_canvas.canvas().unwrap().width() as f64,
            self.background_canvas.canvas().unwrap().height() as f64,
        );

        let mut x = 0;
        let mut y = 0;

        for ele in background_map_1 {
            // Generate Tile Image data
            let tile_bytes = &mut char_map_tiles_bytes[ele as usize];
            let clamped_image_source = wasm_bindgen::Clamped(&mut tile_bytes[..]);

            let tile_image_data =
                web_sys::ImageData::new_with_u8_clamped_array(clamped_image_source, 8).unwrap();

            self.background_canvas
                .put_image_data(&tile_image_data, x as f64, y as f64)
                .unwrap();

            x = x + 8;
            if x >= 32 * 8 {
                x = 0;
                y = y + 8;
            }
        }

        info!("Rust draw background");

        self.draw_screen(gameboy);
    }

    pub fn update_char_map_canvas(&mut self, gameboy: &mut Gameboy) {
        let mut image_source = gameboy.char_map_to_image_data();
        let clamped_image_source = wasm_bindgen::Clamped(&mut image_source[..]);

        let image_data: web_sys::ImageData =
            web_sys::ImageData::new_with_u8_clamped_array(clamped_image_source, 8).unwrap();

        self.char_map_canvas
            .put_image_data(&image_data, 0.0, 0.0)
            .unwrap();

        let tiles_per_row = 12;
        let width = self.char_map_debug_canvas.canvas().unwrap().width() as f64;
        let height = self.char_map_debug_canvas.canvas().unwrap().height() as f64;

        self.char_map_debug_canvas
            .clear_rect(0.0, 0.0, width, height);

        for tile_idx in 0..96 {
            //Get tile image data
            let y0 = (tile_idx * 8) as f64;
            let image_data = self.char_map_canvas.get_image_data(0.0, y0, 8.0, y0 + 8.0);
            let tile = image_data.unwrap();

            let x = (tile_idx % tiles_per_row) as f64;
            let y = ((tile_idx / tiles_per_row) as f64).floor();
            self.char_map_debug_canvas
                .put_image_data(&tile, x * 8.0, y * 8.0)
                .unwrap();
        }

        self.update_char_map_canvas_last_data = image_source;
    }

    pub fn draw_screen(&self, gameboy_inst: &mut Gameboy) {
        if !gameboy_inst.is_vblank() {
            return;
        }

        //Clear context
        self.screen_canvas.clear_rect(
            0.0,
            0.0,
            self.screen_canvas.canvas().unwrap().width() as f64,
            self.screen_canvas.canvas().unwrap().height() as f64,
        );

        let is_lcd_enable = gameboy_inst.is_lcd_display_enable();
        if !is_lcd_enable {
            return;
        }

        let x = gameboy_inst.get_scroll_x();
        let y = gameboy_inst.get_scroll_y();

        let image_data = self
            .background_canvas
            .get_image_data(
                x as f64,
                y as f64,
                PIXEL_ZOOM as f64 * SCREEN_WIDTH as f64,
                PIXEL_ZOOM as f64 * SCREEN_HEIGHT as f64,
            )
            .unwrap();

        self.screen_canvas
            .put_image_data(&image_data, 0.0, 0.0)
            .unwrap();
    }

    pub fn make_canvas(
        canvas_selector: &str,
        width: u32,
        height: u32,
    ) -> web_sys::CanvasRenderingContext2d {
        let document = web_sys::window().unwrap().document().unwrap();

        let el = document.get_element_by_id(canvas_selector).unwrap();
        let el: web_sys::HtmlCanvasElement = el
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        let ctx = el
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        el.set_width(width);
        el.set_height(height);
        // el.style().width = el.width * zoom + 'px';
        // el.style.height = el.height * zoom + 'px';

        ctx.set_image_smoothing_enabled(false);

        ctx
    }

    fn clear_context(
        &self,
        context: web_sys::CanvasRenderingContext2d,
    ) -> web_sys::CanvasRenderingContext2d {
        context.clear_rect(
            0.0,
            0.0,
            context.canvas().unwrap().width() as f64,
            context.canvas().unwrap().height() as f64,
        );

        context
    }
}

#[wasm_bindgen]
pub struct FmOsc {
    ctx: AudioContext,
    /// The primary oscillator.  This will be the fundamental frequency
    primary: web_sys::OscillatorNode,
    /// Overall gain (volume) control
    gain: web_sys::GainNode,
    /// Amount of frequency modulation
    fm_gain: web_sys::GainNode,
    /// The oscillator that will modulate the primary oscillator's frequency
    fm_osc: web_sys::OscillatorNode,
    /// The ratio between the primary frequency and the fm_osc frequency.
    /// Generally fractional values like 1/2 or 1/4 sound best
    fm_freq_ratio: f32,
    fm_gain_ratio: f32,
}

#[wasm_bindgen]
impl FmOsc {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<FmOsc, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        // Create our web audio objects.
        let primary = ctx.create_oscillator()?;
        let fm_osc = ctx.create_oscillator()?;
        let gain = ctx.create_gain()?;
        let fm_gain = ctx.create_gain()?;

        // Some initial settings:
        primary.set_type(OscillatorType::Square);
        primary.frequency().set_value(0.0);
        gain.gain().set_value(0.0); // starts muted
        fm_gain.gain().set_value(0.0); // no initial frequency modulation
        fm_osc.set_type(OscillatorType::Square);
        fm_osc.frequency().set_value(0.0);

        // Connect the nodes up!

        // The primary oscillator is routed through the gain node, so that
        // it can control the overall output volume.
        primary.connect_with_audio_node(&gain)?;

        // Then connect the gain node to the AudioContext destination (aka
        // your speakers).
        gain.connect_with_audio_node(&ctx.destination())?;

        // The FM oscillator is connected to its own gain node, so it can
        // control the amount of modulation.
        fm_osc.connect_with_audio_node(&fm_gain)?;

        // Connect the FM oscillator to the frequency parameter of the main
        // oscillator, so that the FM node can modulate its frequency.
        fm_gain.connect_with_audio_param(&primary.frequency())?;

        // Start the oscillators!
        primary.start()?;
        fm_osc.start()?;

        Ok(FmOsc {
            ctx,
            primary,
            gain,
            fm_gain,
            fm_osc,
            fm_freq_ratio: 0.0,
            fm_gain_ratio: 0.0,
        })
    }

    pub fn volume(&self) -> f32 {
        let gain = self.gain.gain().value();
        gain
    }

    pub fn frequency(&self) -> f32 {
        let fr = self.primary.frequency().value();
        fr
    }

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    #[wasm_bindgen]
    pub fn set_gain(&self, mut gain: f32) {
        if gain > 1.0 {
            gain = 1.0;
        }
        if gain < 0.0 {
            gain = 0.0;
        }
        self.gain.gain().set_value(gain);
    }

    #[wasm_bindgen]
    pub fn set_primary_frequency(&self, freq: f32) {
        self.primary.frequency().set_value(freq);

        // The frequency of the FM oscillator depends on the frequency of the
        // primary oscillator, so we update the frequency of both in this method.
        self.fm_osc.frequency().set_value(self.fm_freq_ratio * freq);
        self.fm_gain.gain().set_value(self.fm_gain_ratio * freq);
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_fm_amount(&mut self, amt: f32) {
        self.fm_gain_ratio = amt;

        self.fm_gain
            .gain()
            .set_value(self.fm_gain_ratio * self.primary.frequency().value());
    }

    #[wasm_bindgen]
    pub fn set_gain_shift(&mut self, original_volume_float: f32, shift_num: u8, is_increase: bool) {
        let current_time = self.ctx.current_time();
        let one64th = 1.0 / 64.0;
        let shift_length = (one64th) as f64 * shift_num as f64;
        let original_volume = (original_volume_float * 10.0) as u8;

        if is_increase {
            let steps_to_max = MAX_GAMEBOY_VOLUME - (original_volume as u8 * 10);
            for shift_offset in 1..steps_to_max as u8 {
                let at_time = current_time + (shift_offset as f64 * shift_length);
                let volume = (original_volume + (shift_offset)) as f32 / 10.0;

                match self.gain.gain().set_value_at_time(volume, at_time) {
                    Ok(_v) => (),
                    Err(_e) => (),
                }
            }
        } else {
            let steps_to_min = original_volume as u8 + 1;
            for shift_offset in 1..steps_to_min {
                let at_time = current_time + (shift_offset as f64 * shift_length);
                let volume = (original_volume - (shift_offset)) as f32 / 10.0;

                // info!(
                //     "volume={:?} original_volume={:?} shift_offset={:?}",
                //     volume, original_volume, shift_offset
                // );

                match self.gain.gain().set_value_at_time(volume, at_time) {
                    Ok(_v) => (),
                    Err(_e) => (),
                }
            }
        }
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_fm_frequency(&mut self, amt: f32) {
        self.fm_freq_ratio = amt;
        self.fm_osc
            .frequency()
            .set_value(self.fm_freq_ratio * self.primary.frequency().value());
    }
}

#[wasm_bindgen]
pub struct Channel {
    sweep_time: f32,
    is_sweep_increase: bool,
    sweep_shift_num: u8,
    wave_duty_pct: f32,
    sound_length_sec: f32,
    volume: u8,
    is_envelop_increase: bool,
    envelop_shift_num: u8,
    fr: u16,
    frequency: f32,
    is_restart: bool,
    is_use_length: bool,
}

#[wasm_bindgen]
impl Channel {
    pub fn sweep_time(&self) -> f32 {
        self.sweep_time
    }

    pub fn is_sweep_increase(&self) -> bool {
        self.is_sweep_increase
    }

    pub fn sweep_shift_num(&self) -> u8 {
        self.sweep_shift_num
    }

    pub fn wave_duty_pct(&self) -> f32 {
        self.wave_duty_pct
    }

    pub fn sound_length_sec(&self) -> f32 {
        self.sound_length_sec
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn is_envelop_increase(&self) -> bool {
        self.is_envelop_increase
    }

    pub fn envelop_shift_num(&self) -> u8 {
        self.envelop_shift_num
    }

    pub fn fr(&self) -> u16 {
        self.fr
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    pub fn is_restart(&self) -> bool {
        self.is_restart
    }

    pub fn is_use_length(&self) -> bool {
        self.is_use_length
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
struct Flag {
    z: bool, //(0x80) if zero
    n: bool, //(0x40) if subtraction
    h: bool, //(0x20) if the lower half of the byte overflowed past 15
    c: bool, //(0x10) if result over 255 or under 0
    interuption: bool,
}

impl Flag {
    fn set_flag(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.z = z;
        self.n = n;
        self.h = h;
        self.c = c;
    }

    fn set_interuption(&mut self, interupt: bool) {
        self.interuption = interupt
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    f: Flag, // Control last operation result
    sp: u16,
    pc: u16,
}

#[wasm_bindgen]
impl Registers {
    fn execute_instruction(&mut self, opcode: u8, memory: &mut Vec<u8>) {
        let pointer = self.pc as usize;
        let mut flag_z = false;
        let mut flag_n = false;
        let mut flag_h = false;
        let mut flag_c = false;

        match opcode {
            0x031 => {
                //LD SP, $0xFFFE
                let value = self.following_two_bytes(pointer, &memory);
                self.set_sp(value);
                self.inc_pc();
            }
            0x0AF => {
                // XOR A
                // Logical exclusive OR n with register A, result in A?
                // This is to set A to 0, regardless of what's currently in it
                self.set_a(0 as u8);
                flag_z = true;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x021 => {
                //LD HL, *2bytes
                let value = self.following_two_bytes(self.pc as usize, &memory);
                self.set_hl(value);
                self.inc_pc();
            }
            0x077 => {
                //LD (HL), A
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.a;
                self.inc_pc();
            }
            0x011 => {
                //LD DE,*16bit
                let value = self.following_two_bytes(pointer, memory);
                self.set_de(value);
                self.inc_pc();
            }
            0x00E => {
                //LD C, *1byte
                let value = self.following_byte(pointer, memory);
                self.set_c(value);
                self.inc_pc();
            }
            0x03E => {
                //LD A, *1byte
                let value = self.following_byte(pointer, memory);
                self.set_a(value);
                self.inc_pc();
            }
            0x006 => {
                //LD B, *1byte
                let value = self.following_byte(pointer, memory);
                self.set_b(value);
                self.inc_pc();
            }
            0x002e => {
                //LD L, *1byte
                let value = self.following_byte(pointer, memory);
                self.set_l(value);
                self.inc_pc();
            }
            0x001e => {
                //LD E, *1byte
                let value = self.following_byte(pointer, memory);
                self.set_e(value);
                self.inc_pc();
            }
            0x0016 => {
                //LD D, *1byte
                let value = self.following_byte(pointer, memory);
                self.set_d(value);
                self.inc_pc();
                println!("d is 0x020 => d:{:x}", self.d);
            }
            0x07B => {
                //LD A, E
                self.set_a(self.e);
                self.inc_pc();
            }
            0x07C => {
                //LD A, H
                self.set_a(self.h);
                self.inc_pc();
            }
            0x07D => {
                //LD A, L
                self.set_a(self.l);
                self.inc_pc();
            }
            0x078 => {
                //LD A, B
                self.set_a(self.b);
                self.inc_pc();
            }
            0x01A => {
                //LD A, (DE)
                let d_e = self.combine_two_bytes(self.d, self.e);
                let value = memory[d_e as usize];
                self.set_a(value as u8);
                self.inc_pc();
            }
            0x04F => {
                //LD C,A
                self.set_c(self.a);
                self.inc_pc();
            }
            0x067 => {
                //LD H,A
                self.set_h(self.a);
                self.inc_pc();
            }
            0x057 => {
                //LD D,A
                self.set_d(self.a);
                self.inc_pc();
            }
            0x032 => {
                //LD (HL-), A
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.a;
                self.set_hl(h_l.wrapping_sub(1) as u16);
                self.inc_pc();
            }
            0x022 => {
                //LD (HL+), A
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.a;
                self.set_hl(h_l + 1);
                self.inc_pc();
            }
            0x0f0 => {
                //LD A, ($ff00+n)
                let following_byte = self.following_byte(pointer, memory);
                let offset = 0xff00 + following_byte as usize;
                let value = memory[offset];
                // info!(
                //     "LD A, ($ff00+{:x}): ${:x}={:x} ",
                //     following_byte, offset, value
                // );
                self.set_a(value);
                self.inc_pc();
            }
            0x0E2 => {
                //LD ($ff00+C), A
                memory[0xFF00 + self.c as usize] = self.a;
                self.inc_pc();
            }
            0x0E0 => {
                //LD ($ff00+n), A
                let memory_add = 0xFF00 + self.following_byte(pointer, memory) as u16;
                memory[memory_add as usize] = self.a;
                self.inc_pc();
            }
            0x0CB => {
                match self.following_byte(pointer, memory) {
                    0x07c => {
                        if self.h & 0x80 == 0x00 {
                            flag_z = true;
                        }
                        self.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }
                    0x011 => {
                        self.c = self.c.rotate_left(1);
                        if self.c & 0x001 == 0 {
                            flag_c = false
                        } else {
                            flag_c = true
                        }
                        self.c = self.c | self.f.c as u8;
                        self.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }
                    other => println!("Unrecogized opcode (CB: {:x})", other),
                }

                self.inc_pc();
            }
            0x017 => {
                // RLA: Rotate A left through Carry flag.
                // 1. Note current carry-flag value:
                let msb_is_set = 0b10000000 & self.a == 0b10000000;
                self.a = (self.a << 1) | self.f.c as u8;

                flag_c = msb_is_set;

                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x020 => {
                //JR NZ,*one byte
                if !self.f.z {
                    let n_param = self.following_byte(pointer, memory) as i8;
                    self.inc_pc();
                    let destination = self.pc as i16 + n_param as i16;
                    // info!(
                    //     "PC: {:x}, n: {:x}, destination: {:x}",
                    //     self.pc, n_param, destination
                    // );
                    self.set_pc(destination as u16);
                } else {
                    self.inc_pc();
                    self.inc_pc();
                }
            }
            0x028 => {
                //JR Z,*
                if self.f.z {
                    let value = self.following_byte(pointer, memory) as i8;
                    self.inc_pc();
                    let address = self.pc as i16 + value as i16;
                    self.set_pc(address as u16);
                } else {
                    self.inc_pc();
                    self.inc_pc();
                }
            }
            0x018 => {
                //JR n
                let value = self.following_byte(pointer, memory) as i8;
                self.inc_pc();
                let address = self.pc as i16 + value as i16;
                self.set_pc(address as u16);
            }
            0x00C => {
                //INC C
                let value = self.c + 1;
                if value == 0 {
                    flag_z = true;
                };
                if self.check_half_carry(self.c, 1) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.set_c(value);
                self.inc_pc();
            }
            0x004 => {
                //INC B
                let value = self.b + 1;

                if value == 0 {
                    flag_z = true;
                };
                if self.check_half_carry(self.b, 1) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.set_b(value);
                self.inc_pc();
            }
            0x0CD => {
                //CALL
                let next_two_bytes = self.following_two_bytes(pointer, memory);
                let next_instruction_address = self.pc + 1;
                self.push_stack(memory, next_instruction_address);
                self.set_pc(next_two_bytes);
            }
            0x0C9 => {
                //RET
                let address = self.pop_stack(self.sp, memory);
                self.set_pc(address);
            }
            0x0C5 => {
                //PUSH BC
                let bc_value = self.combine_two_bytes(self.b, self.c);
                self.push_stack(memory, bc_value);
                self.inc_pc();
            }
            0x0C1 => {
                //POP nn
                let value = self.pop_stack(self.sp, memory);
                //
                self.set_bc(value);
                self.inc_pc();
            }
            0x005 => {
                //DEC B
                let value = self.b.wrapping_sub(1);
                self.set_b(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.b, 1u8) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x00D => {
                //DEC C
                let value = self.c.wrapping_sub(1);
                self.set_c(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.c, 1u8) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x01D => {
                //DEC E
                let value = self.e.wrapping_sub(1);
                self.set_e(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.e, 1u8) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x03D => {
                //DEC A
                let value = self.a.wrapping_sub(1);
                self.set_a(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, 1u8) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x015 => {
                //DEC D
                let value = self.d.wrapping_sub(1);
                self.set_d(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.d, 1u8) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x013 => {
                //INC DE
                let value = self.combine_two_bytes(self.d, self.e);
                self.set_de(value + 1);
                self.inc_pc();
            }
            0x023 => {
                //INC HL
                let value = self.combine_two_bytes(self.h, self.l) + 1;
                self.set_hl(value);
                self.inc_pc();
            }
            0x024 => {
                //INC H
                let value = self.h + 1;
                self.set_h(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.h, 1u8) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x0FE => {
                //CP #
                let following_byte = self.following_byte(pointer, memory);
                if self.a == following_byte {
                    flag_z = true
                }
                flag_n = true;

                if self.check_half_carry_sub(self.a, following_byte) {
                    //TODO:  Set if no borrow from bit 4.
                    //- why set if no borrow instead of borrow?
                    flag_h = true
                }

                if self.a < following_byte {
                    flag_c = true;
                }

                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x0BE => {
                //CP (HL)
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];

                if self.a == value {
                    flag_z = true
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, value) {
                    flag_h = true
                }
                if self.a < value {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x0EA => {
                // LD (nn),A
                let following_two_bytes = self.following_two_bytes(pointer, memory);
                memory[following_two_bytes as usize] = self.a;
                self.inc_pc();
            }
            0x090 => {
                // SUB B
                let value = self.a - self.b;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.b) {
                    flag_h = true
                }
                if self.a < self.b {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x086 => {
                // ADD (HL)
                let h_l = self.combine_two_bytes(self.h, self.l);
                //info!("HL: {:x}, A:{:?}, $14D:{:?}", h_l, self.a, memory[0x014d]);
                let value = self.a.wrapping_add(memory[h_l as usize]);

                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, value) {
                    flag_h = true
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            //New Opcodes after BootRom
            0x000 => {
                //NOP
                self.inc_pc();
            }

            0x0CE => {
                //ADC A,#
                let following_byte = self.following_byte(pointer, memory);
                let value = self.f.c as u8 + following_byte;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, 1u8) {
                    flag_h = true;
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x066 => {
                //LD H,(hl) - 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_h(value);
                self.inc_pc();
            }

            0x0CC => {
                //CALL Z, nn - 12
                let next_two_bytes = self.following_two_bytes(pointer, memory);
                if self.f.z {
                    let next_instruction_address = self.pc + 1;
                    self.push_stack(memory, next_instruction_address);
                    self.set_pc(next_two_bytes);
                } else {
                    self.inc_pc();
                }
            }

            0x00B => {
                //DEB BC - 8
                let b_c = self.combine_two_bytes(self.b, self.c);
                let value = b_c - 1;
                self.set_bc(value);
                self.inc_pc();
            }

            0x003 => {
                //INC BC - 8
                let b_c = self.combine_two_bytes(self.b, self.c);
                let value = b_c + 1;
                self.set_bc(value);
                self.inc_pc();
            }

            0x073 => {
                //LD (HL),E
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.e;
                self.inc_pc();
            }

            0x083 => {
                //ADD A,E
                let value = self.a.wrapping_add(self.e);
                self.set_a(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, value) {
                    flag_h = true
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x008 => {
                //LD (nn), SP - 20
                let address = self.following_two_bytes(pointer, &memory);
                self.set_two_bytes(memory, address, self.sp);
                self.inc_pc();
                // info!(
                //     "LD (nn), SP - Put Stack Pointer at address n.sp: {:x}, address:{:x}, address value:{:x} & {:x}",
                //     self.sp,address,memory[address as usize], memory[address as usize + 1]
                // )
            }
            0x01F => {
                //RRA
                self.a = self.a.rotate_right(1);
                if self.a & 0x001 == 0 {
                    flag_c = false
                } else {
                    flag_c = true
                }
                self.a = self.a | self.f.c as u8;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x088 => {
                // ADC A,B - 4
                let value = self.f.c as u8 + self.b;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, 1u8) {
                    flag_h = true;
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }
            0x089 => {
                // ADC A,C - 4
                let value = self.f.c as u8 + self.c;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, 1u8) {
                    flag_h = true;
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x06E => {
                //LD L,(hl) - 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_l(value);
                self.inc_pc();
            }

            0x0E6 => {
                //AND #
                let next_byte = self.following_byte(pointer, memory);
                let value = next_byte & self.a;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                };
                if self.check_half_carry(self.c, 1) {
                    flag_h = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x0dd => match self.following_byte(pointer, memory) {
                0x0dd => {
                    info!("dd again, pc: {:x}", self.pc);
                }

                0x0D9 => {
                    //RETI
                    // info!("0x0DD -> 0x0D9, sp:{:x}", self.sp);
                    //TODO: stack set & pop
                    let address = self.pop_stack(self.sp, memory);
                    self.set_pc(address);
                    //Endable interrupts
                    self.f.set_interuption(true);
                    info!("enable interrupts");
                    std::process::exit(1)
                }

                other => {
                    info!("Unknown instruction after 0x0DD: {:x}", other);
                    std::process::exit(1)
                }
            },

            0x0C3 => {
                // JP nn - 12
                let value = self.following_two_bytes(pointer, memory);
                self.set_pc(value)
            }

            0x0f3 => {
                // DI
                //Interrupts are disabled after instruction after DI is executed.
                self.f.set_interuption(false);
                self.inc_pc();
            }

            0x036 => {
                //LD (HL),n -> 12
                let value = self.following_byte(self.pc as usize, &memory);
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = value;
                self.inc_pc();
            }

            0x02a => {
                // LDI A,(HL) -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                self.set_a(memory[h_l as usize]);
                self.set_hl(h_l + 1);
                self.inc_pc();
            }

            0x047 => {
                // LD B,A -> 4
                self.set_b(self.a);
                self.inc_pc();
            }

            0x002 => {
                //LD (BC), A -> 8
                self.set_bc(self.a as u16);
                self.inc_pc();
            }

            //New Round
            0x0fd => {
                //No operation?
                println!("no operation with opcode 0xfd");
                self.inc_pc();
            }

            0x06d => {
                //LD L,L -> 4
                self.set_l(self.l);
                self.inc_pc();
            }

            0x06c => {
                //LD L,H -> 4
                self.set_l(self.h);
                self.inc_pc();
            }

            0x071 => {
                //LD (HL), C -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.c;
                self.inc_pc();
            }

            0x03c => {
                //INC A -> 4
                let value = self.a + 1;
                if value == 0 {
                    flag_z = true;
                };
                if self.check_half_carry(self.a, 1) {
                    flag_h = true;
                }
                flag_c = self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.set_a(value);
                self.inc_pc();
            }

            0x0e1 => {
                //POP HL -> 12
                let value = self.pop_stack(self.sp, memory);
                self.set_hl(value);
                self.inc_pc();
            }

            0x055 => {
                //LD D,L -> 4
                self.set_d(self.l);
                self.inc_pc();
            }

            0x056 => {
                //LD D,(HL) -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_d(value);
                self.inc_pc();
            }

            0x05a => {
                //LD E,D -> 4
                self.set_e(self.d);
                self.inc_pc();
            }

            0x061 => {
                //LD H,C -> 4
                self.set_h(self.c);
                self.inc_pc();
            }

            0x05b => {
                //LD E,E -> 4
                self.set_e(self.e);
                self.inc_pc();
            }

            0x049 => {
                //LD C,C -> 4
                self.set_c(self.c);
                self.inc_pc();
            }

            0x05e => {
                //LD E,(HL) -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_e(value);
                self.inc_pc();
            }

            0x058 => {
                //LD E,B -> 4
                self.set_e(self.b);
                self.inc_pc();
            }

            0x05c => {
                //LD E,H -> 4
                self.set_e(self.h);
                self.inc_pc();
            }

            0x051 => {
                //LD D,C -> 4
                self.set_d(self.c);
                self.inc_pc();
            }

            0x050 => {
                //LD D,B -> 4
                self.set_d(self.b);
                self.inc_pc();
            }

            0x04C => {
                //LD C,H -> 4
                self.set_d(self.b);
                self.inc_pc();
            }

            0x04E => {
                //LD C,(HL) -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_c(value);
                self.inc_pc();
            }

            0x059 => {
                //LD E,C -> 4
                self.set_e(self.d);
                self.inc_pc();
            }

            0x053 => {
                //LD D,E -> 4
                self.set_d(self.e);
                self.inc_pc();
            }

            0x052 => {
                //LD D,D -> 4
                self.set_d(self.d);
                self.inc_pc();
            }

            0x04d => {
                //LD C,L -> 4
                self.set_c(self.l);
                self.inc_pc();
            }

            0x054 => {
                //LD D,H -> 4
                self.set_d(self.h);
                self.inc_pc();
            }

            0x0d3 => {
                //No operation?
                println!("no operation with opcode 0xd3");
                self.inc_pc();
            }

            0x0a6 => {
                //AND (HL)
                let h_l = self.combine_two_bytes(self.h, self.l);
                let address_value = memory[h_l as usize];
                let value = address_value & self.a;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                };
                if self.check_half_carry(self.c, 1) {
                    flag_h = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x05d => {
                //LD E,L
                self.set_e(self.l);
                self.inc_pc();
            }

            0x03a => {
                //LD A, (HL-)
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_a(value);
                self.set_hl(h_l.wrapping_sub(1));
                self.inc_pc();
            }

            //New Round 2
            0x040 => {
                //LD B,B
                self.set_b(self.b);
                self.inc_pc();
            }
            0x043 => {
                //LD B,E
                self.set_b(self.e);
                self.inc_pc();
            }

            0x038 => {
                //JR C,*one byte -> 8
                if !self.f.c {
                    let n_param = self.following_byte(pointer, memory) as i8;
                    self.inc_pc();
                    let destination = self.pc as i16 + n_param as i16;
                    self.set_pc(destination as u16);
                } else {
                    self.inc_pc();
                    self.inc_pc();
                }
            }

            0x0c2 => {
                //JP NZ,nn -> 12
                let value = self.following_two_bytes(pointer, memory);
                if !self.f.z {
                    self.set_pc(value);
                } else {
                    self.inc_pc();
                }
            }

            0x0f4 => {
                //No operation?
                println!("no operation with opcode 0xf4");
                self.inc_pc();
            }

            0x07f => {
                // LD A,A -> 4
                self.set_a(self.a);
                self.inc_pc();
            }

            0x074 => {
                // LD (HL),H -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.h;
                self.inc_pc();
            }

            0x075 => {
                // LD (HL),L -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.l;
                self.inc_pc();
            }

            0x072 => {
                // LD (HL),D -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.d;
                self.inc_pc();
            }

            0x076 => {
                //HALT Power down CPU until interrupt occurs -> 4
                //Implementation escalated to Gameboy. Checking at fn execute_opcodes()
                println!("NEED TO IMPLEMENT HALT FUNCTION FOR 0x076");
                self.inc_pc();
            }

            0x079 => {
                //LD A,C -> 4
                self.set_a(self.c);
                self.inc_pc();
            }

            0x0f1 => {
                //POP AF -> 12
                let value = self.pop_stack(self.sp, memory);
                self.set_af(value);
                self.inc_pc();
            }

            0x085 => {
                //ADD A,L
                let value = self.a.wrapping_add(self.l);
                self.set_a(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, value) {
                    flag_h = true
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x0b1 => {
                //OR C
                let value = self.c | self.a;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.inc_pc();
            }

            0x03f => {
                //CCF -> 4
                flag_z = self.f.z;
                flag_c = !self.f.c;
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x042 => {
                // LD B,D -> 4
                self.set_b(self.d);
                self.inc_pc();
            }

            0x081 => {
                //ADD A,C
                let value = self.a.wrapping_add(self.c);
                self.set_a(value);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.check_half_carry(self.a, value) {
                    flag_h = true
                }
                if self.check_carry(self.a, value) {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x046 => {
                //LD B,(HL)
                let h_l = self.combine_two_bytes(self.h, self.l);
                let value = memory[h_l as usize];
                self.set_b(value);
                self.inc_pc();
            }

            0x0b5 => {
                //OR L
                let value = self.l | self.a;
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.inc_pc();
            }

            0x070 => {
                // LD (HL),B -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                memory[h_l as usize] = self.b;
                self.inc_pc();
            }

            0x048 => {
                // LD C,B -> 4
                self.set_c(self.b);
                self.inc_pc();
            }

            0x0C4 => {
                let next_two_bytes = self.following_two_bytes(pointer, memory);
                if !self.f.z {
                    //CALL NZ, nn -> 24
                    let next_instruction_address = self.pc + 1;
                    self.push_stack(memory, next_instruction_address);
                    self.set_pc(next_two_bytes);
                } else {
                    //CALL NZ, nn -> 12
                    self.inc_pc();
                }
            }

            0x069 => {
                // LD L,C -> 4
                self.set_l(self.c);
                self.inc_pc();
            }

            0x06a => {
                // LD L,D -> 4
                self.set_l(self.d);
                self.inc_pc();
            }

            0x06f => {
                // LD L,A -> 4
                self.set_l(self.a);
                self.inc_pc();
            }

            0x0D1 => {
                //POP DE -> 12
                let value = self.pop_stack(self.sp, memory);
                //
                self.set_de(value);
                self.inc_pc();
            }

            0x05f => {
                // LD E,A -> 4
                self.set_e(self.a);
                self.inc_pc();
            }

            0x092 => {
                // SUB D -> 4
                let value = self.a.wrapping_sub(self.d);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.d) {
                    flag_h = true
                }
                if self.a < self.d {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x097 => {
                // SUB A -> 4
                let value = self.a.wrapping_sub(self.a);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.a) {
                    flag_h = true
                }
                if self.a < self.a {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x091 => {
                // SUB C -> 4
                let value = self.a.wrapping_sub(self.c);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.c) {
                    flag_h = true
                }
                if self.a < self.c {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x093 => {
                // SUB E -> 4
                let value = self.a.wrapping_sub(self.e);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.e) {
                    flag_h = true
                }
                if self.a < self.e {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x094 => {
                // SUB H -> 4
                let value = self.a.wrapping_sub(self.h);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.h) {
                    flag_h = true
                }
                if self.a < self.h {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x095 => {
                // SUB L -> 4
                let value = self.a.wrapping_sub(self.l);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, self.l) {
                    flag_h = true
                }
                if self.a < self.l {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x096 => {
                // SUB (HL) -> 8
                let h_l = self.combine_two_bytes(self.h, self.l);
                let address_value = memory[h_l as usize];
                let value = self.a.wrapping_sub(address_value);
                self.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.check_half_carry_sub(self.a, address_value) {
                    flag_h = true
                }
                if self.a < address_value {
                    flag_c = true;
                }
                self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.inc_pc();
            }

            0x041 => {
                // LD B,C -> 4
                self.set_b(self.c);
                self.inc_pc();
            }

            0x044 => {
                // LD B,H -> 4
                self.set_b(self.h);
                self.inc_pc();
            }

            other => {
                info!("No opcode found for {:x} at {:x}", other, pointer);
                std::process::exit(1)
            }
        }
    }

    fn inc_pc(&mut self) {
        self.pc = self.pc + 1;
    }

    fn check_carry(&self, num_a: u8, num_b: u8) -> bool {
        (num_a & 0x00ff) as u16 + (num_b & 0x00ff) as u16 & 0x100 == 0x100
    }

    fn check_half_carry(&self, num_a: u8, num_b: u8) -> bool {
        (num_a & 0x00f) + (num_b & 0x00f) & 0x010 == 0x010
    }

    fn check_half_carry_sub(&self, num_a: u8, num_b: u8) -> bool {
        (num_a & 0x00f) + (!num_b & 0x00f) & 0x010 == 0x010
    }

    fn combine_two_bytes(&self, first_b: u8, second_b: u8) -> u16 {
        let two_bytes_value = ((first_b as u16) << 8) | second_b as u16;
        two_bytes_value
    }

    fn push_stack(&mut self, memory: &mut Vec<u8>, value: u16) {
        self.sp = self.sp - 2;
        let value_byte_vec = value.to_be_bytes();
        memory[self.sp as usize] = value_byte_vec[0];
        memory[self.sp as usize - 1] = value_byte_vec[1];
    }

    fn pop_stack(&mut self, sp: u16, memory: &mut Vec<u8>) -> u16 {
        // println!("Memory last 10: {:x?}", &memory[0xfff0..0xffff]);
        // println!("SP: {:x}", sp);
        let firt_byte = memory[sp as usize];
        let second_byte = memory[sp as usize - 1];
        self.sp = self.sp + 2;
        let result = self.combine_two_bytes(firt_byte, second_byte);
        result
    }

    fn following_byte(&mut self, address: usize, memory: &Vec<u8>) -> u8 {
        let byte = memory[address + 1];
        self.set_pc(&self.pc + 1);
        byte
    }

    fn following_two_bytes(&mut self, address: usize, memory: &Vec<u8>) -> u16 {
        let byte_vec = &memory[address + 1..address + 3];
        let two_bytes_value = self.combine_two_bytes(byte_vec[1], byte_vec[0]);
        self.set_pc(&self.pc + 2);
        two_bytes_value
    }

    fn set_pc(&mut self, value: u16) {
        self.pc = value
    }

    fn set_a(&mut self, value: u8) {
        self.a = value
    }
    fn set_b(&mut self, value: u8) {
        self.b = value
    }
    fn set_c(&mut self, value: u8) {
        self.c = value
    }
    fn set_d(&mut self, value: u8) {
        self.d = value
    }
    fn set_e(&mut self, value: u8) {
        self.e = value
    }
    fn set_h(&mut self, value: u8) {
        self.h = value
    }
    fn set_l(&mut self, value: u8) {
        self.l = value
    }

    fn set_hl(&mut self, value: u16) {
        let byte_vec = value.to_be_bytes();
        self.h = byte_vec[0];
        self.l = byte_vec[1];
    }

    fn set_de(&mut self, value: u16) {
        let byte_vec = value.to_be_bytes();
        self.d = byte_vec[0];
        self.e = byte_vec[1];
    }

    fn set_bc(&mut self, value: u16) {
        let byte_vec = value.to_be_bytes();
        self.b = byte_vec[0];
        self.c = byte_vec[1];
    }

    fn set_af(&mut self, value: u16) {
        let byte_vec = value.to_be_bytes();
        self.a = byte_vec[0];
        let flag = byte_vec[1];

        let mut flag_z = false; //(0x80)
        let mut flag_n = false; //(0x40)
        let mut flag_h = false; //(0x20)
        let mut flag_c = false; //(0x10)

        if flag & 0x80 == 0x80 {
            flag_z = true;
        };
        if flag & 0x40 == 0x40 {
            flag_n = true;
        };
        if flag & 0x20 == 0x20 {
            flag_h = true;
        };
        if flag & 0x10 == 0x10 {
            flag_c = true;
        };

        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn set_two_bytes(&mut self, memory: &mut Vec<u8>, start_address: u16, value: u16) {
        let byte_vec = value.to_be_bytes();
        memory[start_address as usize] = byte_vec[1];
        memory[start_address as usize + 1] = byte_vec[0];
    }

    fn set_sp(&mut self, value: u16) {
        self.sp = value
    }
}

#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pixel {
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}

pub fn pixel_to_rgba(pixel: Pixel) -> [u8; 4] {
    match pixel {
        Pixel::White => [255, 255, 255, 255],
        Pixel::LightGray => [191, 191, 191, 255],
        Pixel::DarkGray => [64, 64, 64, 255],
        Pixel::Black => [0, 0, 0, 255],
    }
}

#[wasm_bindgen]
pub fn pixels_to_image_data(pixels_as_byte_vec: Vec<u8>) -> Vec<u8> {
    let new_image_data = {
        let len = pixels_as_byte_vec.len();
        let bpp = 4;
        let size = len * bpp;
        let mut image_data: Vec<u8> = Vec::with_capacity(size as usize);

        for idx in (0..pixels_as_byte_vec.len()).step_by(2) {
            // Consume two bytes each iteration, and produce
            // 8 pixels of data.
            let low_bits = BitVec::from_bytes(&[pixels_as_byte_vec[idx]]);
            let high_bits = BitVec::from_bytes(&[pixels_as_byte_vec[idx + 1]]);

            for pixel_index in 0..8 {
                let [r, g, b, a] = match (low_bits[pixel_index], high_bits[pixel_index]) {
                    (false, false) => [255, 255, 255, 255],
                    (false, true) => [191, 191, 191, 255],
                    (true, false) => [64, 64, 64, 255],
                    (true, true) => [0, 0, 0, 255],
                };

                image_data.push(r);
                image_data.push(g);
                image_data.push(b);
                image_data.push(a);
            }
        }

        image_data
    };

    return new_image_data.to_vec();
}

#[wasm_bindgen]
#[derive(Deserialize, Serialize)]
pub struct SerializedGameboy {
    registers: Registers,
    total_cycle_num: usize,
    vram_cycle_num: u16,
    timer: usize,
    cpu_clock: usize,
    break_points: Vec<u16>,
    memory: Vec<u8>,
}

#[wasm_bindgen]
pub struct Gameboy {
    background_width: u32,
    background_height: u32,
    screen_width: u32,
    screen_height: u32,
    image_data: Vec<u8>,
    registers: Registers,
    fm_osc: FmOsc,
    total_cycle_num: usize,
    vram_cycle_num: u16,
    timer: usize,
    cpu_clock: usize,
    is_running: bool,
    break_points: Vec<u16>,
    memory: Vec<u8>,
    cpu_paused: bool,
}

#[wasm_bindgen]
impl Gameboy {
    pub fn to_serializable(&self) -> SerializedGameboy {
        let serializable = SerializedGameboy {
            registers: self.registers.clone(),
            total_cycle_num: self.total_cycle_num,
            vram_cycle_num: self.vram_cycle_num,
            timer: self.timer,
            cpu_clock: self.cpu_clock,
            break_points: self.break_points.clone(),
            memory: self.memory.clone(),
        };

        serializable
    }

    pub fn background_width(&self) -> u32 {
        self.background_width
    }

    pub fn screen_width(&self) -> u32 {
        self.screen_width
    }

    pub fn background_height(&self) -> u32 {
        self.background_height
    }

    pub fn screen_height(&self) -> u32 {
        self.screen_height
    }

    pub fn ly(&self) -> u8 {
        self.memory[0xff44]
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn cpu_paused(&self) -> bool {
        self.cpu_paused
    }

    pub fn is_vblank(&self) -> bool {
        self.memory[0xff44] >= 144
    }

    pub fn toggle_is_running(&mut self) {
        info!("toggle running{:?}", self.is_running);
        self.is_running = !self.is_running;
    }

    pub fn stop_running(&mut self) {
        self.is_running = false
    }

    pub fn start_running(&mut self) {
        info!("start running");
        self.is_running = true
    }

    pub fn pause_cpu(&mut self) {
        self.cpu_paused = true
    }

    pub fn start_cpu(&mut self) {
        self.cpu_paused = false
    }

    pub fn set_break_point(&mut self, point: u16) {
        if !self.break_points.contains(&point) {
            self.break_points.push(point);
        }
        info!("add- break points{:?}", self.break_points);
    }

    pub fn remove_break_point(&mut self, point: u16) {
        self.break_points.retain(|&x| x != point);
        info!("remove- break points{:?}", self.break_points);
    }

    //Intrupts
    // fn do_interupt(&self) -> bool {
    //     let has_request = self.memory[0xff0f] & 0b00010111 != 0;
    //     let interupt_enabled = self.memory[0xffff] > 0;

    //     interupt_enabled && has_request
    // }

    // fn set_timer_interupt_register(&mut self) {
    //     self.memory[0xff0f] = self.memory[0xff0f] | 0b00000100u8;
    //     /*
    //        Bit 0: V-Blank  Interrupt Request (INT 40h)  (1=Request)
    //        Bit 1: LCD STAT Interrupt Request (INT 48h)  (1=Request)
    //        Bit 2: Timer    Interrupt Request (INT 50h)  (1=Request)
    //        Bit 3: Serial   Interrupt Request (INT 58h)  (1=Request)
    //        Bit 4: Joypad   Interrupt Request (INT 60h)  (1=Request)
    //     */
    // }

    // fn execute_interuption(&self) {
    //     let interupt_register = self.memory[0xff0f];
    //     if ((interupt_register & 0b00000001u8) == 0b00000001u8) {
    //         //v_blank
    //         self.memory[0x040]
    //     }
    //     // lcd => self.memory[0x048],
    //     // timer => self.memory[0x50],
    //     // serial => self.memory[0x58],
    //     // joypad => self.memory[0x60],
    // }

    pub fn set_vram_cycle(&mut self, value: u16) {
        self.vram_cycle_num = value
    }

    pub fn request_vblank(&mut self) {
        self.memory[0xff0f] = self.memory[0xff0f] | 0b1000000
    }

    pub fn disable_vblank(&mut self) {
        self.memory[0xff0f] = self.memory[0xff0f] ^ 0b1000000
    }

    //LCDC Y-Coordinate : LY
    pub fn inc_ly(&mut self) {
        let ly_max = 153;
        let vblank_start = 144;

        if self.memory[0xff44] == ly_max {
            self.memory[0xff44] = 0;
            self.disable_vblank()
        } else {
            self.memory[0xff44] = self.memory[0xff44] + 1;
            if self.memory[0xff44] == vblank_start {
                self.request_vblank()
            }
        }
    }

    //Timer
    pub fn total_cycle(&self) -> usize {
        self.total_cycle_num
    }

    pub fn vram_cycle(&self) -> u16 {
        self.vram_cycle_num
    }

    pub fn timer_counter_memory(&self) -> u8 {
        self.memory[0xff05]
    }

    pub fn timer(&mut self) -> usize {
        self.timer = self.total_cycle_num / self.timer_frequency();
        self.timer
    }

    pub fn cpu_clock(&mut self) -> usize {
        self.cpu_clock = self.timer / self.timer_cycle_to_cpu_clock();
        self.cpu_clock
    }

    pub fn timer_cycle_to_cpu_clock(&self) -> usize {
        let cpu_clock_speed = 4194304;
        let frequency = self.timer_frequency();

        cpu_clock_speed / frequency
    }

    pub fn is_timer_enabled(&self) -> bool {
        self.memory[0xff07] & 0b00000100u8 == 0b00000100u8
    }

    pub fn timer_frequency(&self) -> usize {
        let timer_frequency = match self.memory[0xff07] & 0b00000011u8 {
            0 => 4096,
            1 => 262144,
            2 => 65536,
            3 => 16384,
            _ => 0,
        };

        timer_frequency
    }

    fn _add_time_counter(&mut self) {
        if self.memory[0xff05] == 255 {
            self.memory[0xff05] = self.memory[0xff06]
        } else {
            self.memory[0xff05] += 1
        }
    }

    pub fn get_divide_register(&self) -> u8 {
        self.memory[0xff04]
    }

    fn add_cycles(&mut self, instruction: u8, cycle_register: CycleRegister) {
        let cycle = match instruction {
            0x031 => 12,
            0x0AF => 4,
            0x021 => 12,
            0x077 => 8,
            0x011 => 12,
            0x00E => 8,
            0x03E => 8,
            0x006 => 8,
            0x002e => 8,
            0x001e => 8,
            0x0016 => 8,
            0x07B => 4,
            0x07C => 4,
            0x07D => 4,
            0x078 => 4,
            0x01A => 8,
            0x04F => 4,
            0x067 => 4,
            0x057 => 4,
            0x032 => 8,
            0x022 => 8,
            0x0f0 => 12,
            0x0E2 => 8,
            0x0E0 => 12,
            0x0CB => // match self
            //     .registers
            //     .following_byte(self.registers.pc as usize, &self.memory)
            // {
            //     0x07c => 8,
            //     0x011 => 8,
            //     other => {
            //         println!("Unrecogized opcode (CB: {:x})", other);
            //         std::process::exit(1)
            //     }
            // }
                8,
            0x017 => 4,
            0x020 => 8,
            0x028 => 8,
            0x018 => 8,
            0x00C => 4,
            0x004 => 4,
            0x0CD => 12,
            0x0C9 => 8,
            0x0C5 => 16,
            0x0C1 => 12,
            0x005 => 4,
            0x00D => 4,
            0x01D => 4,
            0x03D => 4,
            0x015 => 4,
            0x013 => 8,
            0x023 => 8,
            0x024 => 4,
            0x0FE => 8,
            0x0BE => 8,
            0x0EA => 16,
            0x090 => 4,
            0x086 => 8,
            0x000 => 4,
            0x0CE => 8,
            0x066 => 8,
            0x0CC =>
            { if self.registers.f.z  {24} else {12}  },
            0x00B => 8,
            0x003 => 8,
            0x073 => 8,
            0x083 => 4,
            0x008 => 20,
            0x01F => 4,
            0x088 => 4,
            0x089 => 4,
            0x06E => 8,
            0x0E6 => 8,
            0x0dd => {
                println!("Not sure what's the cycle for 0x0DD");
                std::process::exit(1)
            }
            0x0C3 => 12,
            0x0f3 => 4,
            0x036 =>  12,
            0x02a => 8,
            0x047 => 4,
            0x002 => 8,
                        //New Round
            0x0fd => 0,
            0x06d => 4,
            0x06c => 4,
            0x071 => 8,
            0x03c => 4,
            0x0e1 => 12,
            0x055 => 4,
            0x056 => 8,
            0x05a => 4,
            0x061 => 4,
            0x05b => 4,
            0x049 => 4,
            0x05e => 8,
            0x058 => 4,
            0x05c => 4,
            0x051 => 4,
            0x050 => 4,
            0x04C => 4,
            0x04E => 8,
            0x059 => 4,
            0x053 => 4,
            0x052 => 4,
            0x04d => 4,
            0x054 => 4,
            0x0d3 => 0,
            0x0a6 => 8,
            0x05d => 4,
            0x03a => 8,
                        //New Round 2
            0x040 => 4,
            0x043 => 4,
            0x038 => 8,
            0x0c2 => 12,
            0x0f4 => 0,
            0x07f => 4,
            0x074 => 8,
            0x075 => 8,
            0x072 => 8,
            0x076 => 4,
            0x079 => 4,
            0x0f1 => 12,
            0x085 => 4,
            0x0b1 => 4,
            0x03f => 4,
            0x042 => 4,
            0x081 => 4,
            0x046 => 8,
            0x0b5 => 4,
            0x070 => 8,
            0x048 => 4,
            0x0C4 =>
            { if !self.registers.f.z  {24} else {12}  },
            0x069 => 4,
            0x06a => 4,
            0x06f => 4,
            0x0D1 => 12,
            0x05f => 4,
            0x092 => 4,
            0x097 => 4,
            0x091 => 4,
            0x093 => 4,
            0x094 => 4,
            0x095 => 4,
            0x096 => 8,
            0x041 => 4,
            0x044 => 4,
            other => {
                println!("Cycle calc - No opcode found for {:x}", other);
                std::process::exit(1)
            }
        };

        match cycle_register {
            CycleRegister::VramCycle => self.vram_cycle_num += cycle as u16,
            CycleRegister::CpuCycle => self.total_cycle_num += cycle as usize,
        }
    }

    pub fn square1(&self) -> Channel {
        let sweep_time_raw = self.memory[0xff10] & 0b01110000u8;
        let sweep_time = match sweep_time_raw {
            0b00000000 => 0.0,
            0b00010000 => 7.8,
            0b00100000 => 15.6,
            0b00110000 => 23.4,
            0b01000000 => 31.3,
            0b01010000 => 39.1,
            0b01100000 => 46.9,
            0b01110000 => 54.7,
            _ => panic!("Improper sweep_time. Check memory 0xff10"),
        };

        let is_sweep_increase = self.memory[0xff10] & 0b00001000u8 == 0b00001000u8;
        let sweep_shift_num = self.memory[0xff10] & 0b00000111u8;

        let _wave_duty_raw = self.memory[0xff11] & 0b11000000u8;
        let wave_duty_pct = match sweep_time_raw {
            0b00000000 => 12.5,
            0b01000000 => 25.0,
            0b10000000 => 50.0,
            0b11000000 => 75.0,
            _ => panic!("Improper wave_duty. Check memory 0xff11"),
        };

        let sound_length_raw = self.memory[0xff11] & 0b00111111u8;
        let sound_length_sec = (64.0 - sound_length_raw as f32) * (1.0 / 256.0);
        let volume = (self.memory[0xff12] & 0b11110000u8) >> 4;
        let is_envelop_increase = self.memory[0xff12] & 0b00001000u8 == 0b00001000u8;
        let envelop_shift_num = self.memory[0xff12] & 0b00000111u8;
        let frequency_raw =
            (self.memory[0xff13] as u16) << 3 | (self.memory[0xff14] & 0b00000111u8) as u16;
        let frequency = 131072.0 / (2048.0 - frequency_raw as f32);
        let is_restart = self.memory[0xff14] & 0b10000000u8 == 0b10000000u8;
        let is_use_length = self.memory[0xff14] & 0b01000000u8 == 0b01000000u8;

        Channel {
            sweep_time,
            is_sweep_increase,
            sweep_shift_num,
            wave_duty_pct,
            sound_length_sec,
            volume,
            is_envelop_increase,
            envelop_shift_num,
            fr: frequency_raw,
            frequency,
            is_restart,
            is_use_length,
        }
    }

    pub fn is_sound_all_on(&self) -> bool {
        self.memory[0xff26] & 0b10000000 == 0b10000000
    }
    pub fn is_sound_4_on(&self) -> bool {
        self.memory[0xff26] & 0b00001000 == 0b0000
    }
    pub fn is_sound_3_on(&self) -> bool {
        self.memory[0xff26] & 0b00000100 == 0b00000100
    }
    pub fn is_sound_2_all_on(&self) -> bool {
        self.memory[0xff26] & 0b00000010 == 0b10000010
    }
    pub fn is_sound_1_on(&self) -> bool {
        self.memory[0xff26] & 0b00000001 == 0b10000001
    }

    pub fn get_lcd(&self) -> u8 {
        self.memory[0xff40]
    }

    pub fn is_lcd_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x80 == 0x80
    }

    pub fn window_tile_map(&self) -> *const u8 {
        if self.memory[0xff40] & 0x40 == 0x40 {
            let window_tile_map = self.memory[0x9c00..0xa000].to_vec();
            window_tile_map.as_ptr()
        } else {
            let window_tile_map = self.memory[0x9800..0x9c00].to_vec();
            window_tile_map.as_ptr()
        }
    }

    pub fn is_window_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x20 == 0x20
    }

    pub fn bg_window_tile_data(&self) -> *const u8 {
        if self.memory[0xff40] & 0x10 == 0x10 {
            let bg_window_tile_data = self.memory[0x8000..0x9000].to_vec();
            bg_window_tile_data.as_ptr()
        } else {
            let bg_window_tile_data = self.memory[0x8800..0x9800].to_vec();
            bg_window_tile_data.as_ptr()
        }
    }

    pub fn bg_tile_map(&self) -> *const u8 {
        if self.memory[0xff40] & 0x08 == 0x08 {
            let bg_tile_map = self.memory[0x9c00..0xa000].to_vec();
            bg_tile_map.as_ptr()
        } else {
            let bg_tile_map = self.memory[0x9800..0x9c00].to_vec();
            bg_tile_map.as_ptr()
        }
    }

    pub fn is_sprite_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x02 == 0x02
    }

    pub fn is_bg_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x01 == 0x01
    }

    pub fn get_scroll_y(&self) -> u8 {
        self.memory[0xff42]
    }

    pub fn get_scroll_x(&self) -> u8 {
        self.memory[0xff43]
    }

    pub fn get_window_y(&self) -> u8 {
        self.memory[0xff4a]
    }

    pub fn get_window_x(&self) -> u8 {
        self.memory[0xff4b]
    }

    pub fn get_a(&self) -> u8 {
        self.registers.a
    }

    pub fn get_b(&self) -> u8 {
        self.registers.b
    }

    pub fn get_c(&self) -> u8 {
        self.registers.c
    }

    pub fn get_d(&self) -> u8 {
        self.registers.d
    }

    pub fn get_e(&self) -> u8 {
        self.registers.e
    }

    pub fn get_h(&self) -> u8 {
        self.registers.h
    }

    pub fn get_l(&self) -> u8 {
        self.registers.l
    }

    pub fn get_sp(&self) -> u16 {
        self.registers.sp
    }

    pub fn get_pc(&self) -> u16 {
        self.registers.pc
    }

    pub fn get_flag_z(&self) -> bool {
        self.registers.f.z
    }

    pub fn get_flag_n(&self) -> bool {
        self.registers.f.n
    }

    pub fn get_flag_h(&self) -> bool {
        self.registers.f.h
    }

    pub fn get_flag_c(&self) -> bool {
        self.registers.f.c
    }

    // Setters
    pub fn set_a(&mut self, value: u8) -> u8 {
        self.registers.a = value;
        self.registers.a
    }

    pub fn set_b(&mut self, value: u8) -> u8 {
        self.registers.b = value;
        self.registers.b
    }

    pub fn set_c(&mut self, value: u8) -> u8 {
        self.registers.c = value;
        self.registers.c
    }

    pub fn set_d(&mut self, value: u8) -> u8 {
        self.registers.d = value;
        self.registers.d
    }

    pub fn set_e(&mut self, value: u8) -> u8 {
        self.registers.e = value;
        self.registers.e
    }

    pub fn set_h(&mut self, value: u8) -> u8 {
        self.registers.h = value;
        self.registers.h
    }

    pub fn set_l(&mut self, value: u8) -> u8 {
        self.registers.l = value;
        self.registers.l
    }

    pub fn set_sp(&mut self, value: u16) -> u16 {
        self.registers.sp = value;
        self.registers.sp
    }

    pub fn set_pc(&mut self, value: u16) -> u16 {
        self.registers.pc = value;
        self.registers.pc
    }

    pub fn set_flag_z(&mut self, value: bool) -> bool {
        self.registers.f.z = value;
        self.registers.f.z
    }

    pub fn set_flag_n(&mut self, value: bool) -> bool {
        self.registers.f.n = value;
        self.registers.f.n
    }

    pub fn set_flag_h(&mut self, value: bool) -> bool {
        self.registers.f.h = value;
        self.registers.f.h
    }

    pub fn set_flag_c(&mut self, value: bool) -> bool {
        self.registers.f.c = value;
        self.registers.f.c
    }

    pub fn pixels(&self) -> *const Pixel {
        let pixel_byte_vec = self.memory[0x8000..0x9800].to_vec();
        let pixels = Gameboy::tile(pixel_byte_vec);

        pixels.as_ptr()
    }

    pub fn memory(&self) -> *const u8 {
        self.memory.as_ptr()
    }

    pub fn background_map_1(&self) -> Vec<u8> {
        let background_map_1 = self.memory[0x9800..0x9c00].to_vec().clone();
        background_map_1
    }

    pub fn execute_opcode(&mut self) {
        // if self.cpu_paused {
        //     return;
        // }

        //ff10-ff14 is responsible for sound channel 1
        let pre_ff10 = self.memory[0xff10];
        let pre_ff11 = self.memory[0xff11];
        let pre_ff12 = self.memory[0xff12];
        let pre_ff13 = self.memory[0xff13];
        let pre_ff14 = self.memory[0xff14];

        let instruction = self.memory[self.registers.pc as usize];
        self.registers
            .execute_instruction(instruction, &mut self.memory);
        self.add_cycles(instruction, CycleRegister::CpuCycle);
        self.cycle_based_vram_operation(instruction);

        if self.break_points.contains(&self.registers.pc) {
            self.is_running = false;
        }

        // if instruction == 0x076 {
        //     self.pause_cpu()
        // }

        if self.is_channel1_changed(pre_ff10, pre_ff11, pre_ff12, pre_ff13, pre_ff14) {
            if self.sound_dirty_flag_check_s1() {
                self.reset_fm_osc(self.square1());
            }
        }
    }

    pub fn is_channel1_changed(
        &self,
        pre_ff10: u8,
        pre_ff11: u8,
        pre_ff12: u8,
        pre_ff13: u8,
        pre_ff14: u8,
    ) -> bool {
        let after_ff10 = self.memory[0xff10];
        let after_ff11 = self.memory[0xff11];
        let after_ff12 = self.memory[0xff12];
        let after_ff13 = self.memory[0xff13];
        let after_ff14 = self.memory[0xff14];

        pre_ff10 != after_ff10
            || pre_ff11 != after_ff11
            || pre_ff12 != after_ff12
            || pre_ff13 != after_ff13
            || pre_ff14 != after_ff14
    }

    pub fn cycle_based_vram_operation(&mut self, instruction: u8) {
        let vram_cycle_per_ly_inc = 456;

        if self.is_lcd_display_enable() {
            self.add_cycles(instruction, CycleRegister::VramCycle);
            if self.vram_cycle_num >= vram_cycle_per_ly_inc {
                self.inc_ly();
                //Resetting vram cycle here
                self.set_vram_cycle(self.vram_cycle_num - vram_cycle_per_ly_inc);
            }
        }
    }

    pub fn execute_opcodes(&mut self, count: u8) {
        // if self.cpu_paused {
        //     return;
        // }

        //ff10-ff14 is responsible for sound channel 1
        let pre_ff10 = self.memory[0xff10];
        let pre_ff11 = self.memory[0xff11];
        let pre_ff12 = self.memory[0xff12];
        let pre_ff13 = self.memory[0xff13];
        let pre_ff14 = self.memory[0xff14];

        for _ in 0..count {
            // if self.cpu_paused {
            //     break;
            // }

            let instruction = self.memory[self.registers.pc as usize];
            self.registers
                .execute_instruction(instruction, &mut self.memory);
            self.add_cycles(instruction, CycleRegister::CpuCycle);
            self.cycle_based_vram_operation(instruction);

            if self.break_points.contains(&self.registers.pc) {
                self.is_running = false;
            }

            // if instruction == 0x076 {
            //     //HALT: Pause CPU Until Interrupt
            //     self.pause_cpu()
            // }

            if self.is_channel1_changed(pre_ff10, pre_ff11, pre_ff12, pre_ff13, pre_ff14) {
                if self.sound_dirty_flag_check_s1() {
                    self.reset_fm_osc(self.square1());
                }
            }
        }
    }

    pub fn reset_fm_osc(&mut self, square1: Channel) {
        self.fm_osc.set_primary_frequency(square1.frequency());
        self.fm_osc.set_gain_shift(
            square1.volume() as f32 * 0.1,
            square1.envelop_shift_num(),
            square1.is_envelop_increase(),
        );
    }

    fn sound_dirty_flag_check_s1(&self) -> bool {
        let is_volume_non_zero = self.square1().volume() > 0;
        let is_frequency_non_zero = self.square1().fr() > 0;
        return self.is_sound_all_on() && is_volume_non_zero && is_frequency_non_zero;
    }

    pub fn new() -> Gameboy {
        info!("Starting a new gameboy!");

        let flag = Flag {
            z: false,
            n: false,
            h: false,
            c: false,
            interuption: false,
        };

        let mut registers = Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: flag, //Control last operation result
            h: 0,
            l: 0,
            sp: 0xffff,
            pc: 0,
        };

        let boot_rom_content = include_bytes!("boot-rom.gb");
        let cartridge_content = include_bytes!("mario.gb");

        let _head = boot_rom_content;
        let _body = &cartridge_content[0x100..(cartridge_content.len())];

        let full_memory_capacity = 0x10000;

        let head = boot_rom_content;
        let body = &cartridge_content[0x100..(cartridge_content.len())];

        let mut full_memory: Vec<u8> = Vec::with_capacity(full_memory_capacity);

        full_memory.extend_from_slice(head);
        full_memory.extend_from_slice(body);

        full_memory.resize_with(full_memory_capacity, || 0);

        // Vblank
        // full_memory[0xff44] = 0x90;

        let pixel_byte_vec = full_memory[0x8000..0x8800].to_vec();
        let image_data = pixels_to_image_data(pixel_byte_vec.clone());

        let _pixels = Gameboy::tile(pixel_byte_vec);

        //FmOsc Here

        let fm_osc = match Gameboy::initialize_fm_osc() {
            Ok(something) => something,
            _ => panic!("Failed initialize FmOsc"),
        };

        Gameboy {
            background_width: BACKGROUND_WIDTH,
            background_height: BACKGROUND_HEIGHT,
            screen_width: SCREEN_WIDTH,
            screen_height: SCREEN_HEIGHT,
            registers,
            fm_osc,
            image_data,
            memory: full_memory,
            total_cycle_num: 0,
            vram_cycle_num: 0,
            timer: 0,
            is_running: false,
            break_points: vec![],
            cpu_clock: 0,
            cpu_paused: false,
            should_draw: false,
        }
    }

    fn initialize_fm_osc() -> Result<FmOsc, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        // Create our web audio objects.
        let primary = ctx.create_oscillator()?;
        let fm_osc = ctx.create_oscillator()?;
        let gain = ctx.create_gain()?;
        let fm_gain = ctx.create_gain()?;

        // Some initial settings:
        primary.set_type(OscillatorType::Square);
        primary.frequency().set_value(0.0);
        gain.gain().set_value(0.0); // starts muted
        fm_gain.gain().set_value(0.0); // no initial frequency modulation
        fm_osc.set_type(OscillatorType::Square);
        fm_osc.frequency().set_value(0.0);

        // Connect the nodes up!

        // The primary oscillator is routed through the gain node, so that
        // it can control the overall output volume.
        primary.connect_with_audio_node(&gain)?;

        // Then connect the gain node to the AudioContext destination (aka
        // your speakers).
        gain.connect_with_audio_node(&ctx.destination())?;

        // The FM oscillator is connected to its own gain node, so it can
        // control the amount of modulation.
        fm_osc.connect_with_audio_node(&fm_gain)?;

        // Connect the FM oscillator to the frequency parameter of the main
        // oscillator, so that the FM node can modulate its frequency.
        fm_gain.connect_with_audio_param(&primary.frequency())?;

        // Start the oscillators!
        primary.start()?;
        fm_osc.start()?;

        Ok(FmOsc {
            ctx,
            primary,
            gain,
            fm_gain,
            fm_osc,
            fm_freq_ratio: 0.0,
            fm_gain_ratio: 0.0,
        })
    }

    fn tile_row(first_b: u8, second_b: u8) -> Vec<Pixel> {
        let low_bits = BitVec::from_bytes(&[first_b]);
        let high_bits = BitVec::from_bytes(&[second_b]);
        let mut row = Vec::new();

        for idx in 0..8 {
            match (low_bits[idx], high_bits[idx]) {
                (false, false) => row.push(Pixel::White),
                (false, true) => row.push(Pixel::LightGray),
                (true, false) => row.push(Pixel::DarkGray),
                (true, true) => row.push(Pixel::Black),
            }
        }
        row
    }

    fn tile(byte_vec: Vec<u8>) -> Vec<Pixel> {
        let mut tile = Vec::new();
        let mut tile_vec = Vec::new();
        let mut idx = 0;

        // console_log("Tile: idx={}")

        while idx < byte_vec.len() {
            for i in (idx..idx + 16).step_by(2) {
                let row = Gameboy::tile_row(byte_vec[i], byte_vec[i + 1]);
                tile.extend(row);
            }
            idx = idx + 16;

            tile_vec.append(&mut tile);
        }
        tile_vec
    }

    pub fn char_map_to_image_data(&mut self) -> Vec<u8> {
        let pixels_vec = self.memory[0x8000..0x8800].to_vec();
        let new_image_data = pixels_to_image_data(pixels_vec);

        self.image_data = new_image_data.clone();

        new_image_data
    }
}

pub fn gameboy_from_serializable(serializeable: SerializedGameboy) -> Gameboy {
    let fm_osc = match Gameboy::initialize_fm_osc() {
        Ok(something) => something,
        _ => panic!("Failed initialize FmOsc"),
    };

    let full_memory = serializeable.memory.clone();

    let pixel_byte_vec = full_memory[0x8000..0x8800].to_vec();
    let image_data = pixels_to_image_data(pixel_byte_vec.clone());

    let gameboy = Gameboy {
        // From serialized
        registers: serializeable.registers.clone(),
        total_cycle_num: serializeable.total_cycle_num,
        vram_cycle_num: serializeable.vram_cycle_num,
        timer: serializeable.timer,
        cpu_clock: serializeable.cpu_clock,
        break_points: serializeable.break_points.clone(),
        memory: full_memory,
        // Default, non-serializable values
        background_width: BACKGROUND_WIDTH,
        background_height: BACKGROUND_HEIGHT,
        screen_width: SCREEN_WIDTH,
        screen_height: SCREEN_HEIGHT,
        fm_osc,
        image_data,
        is_running: false,
        cpu_paused: false,
    };

    gameboy
}

impl SerializedGameboy {
    pub fn to_json(&self) -> JsValue {
        JsValue::from_serde(&self).unwrap()
    }

    pub fn from_json(val: &JsValue) -> Gameboy {
        let serialized: SerializedGameboy = val.into_serde().unwrap();
        let gameboy: Gameboy = gameboy_from_serializable(serialized);
        gameboy
    }
}

#[wasm_bindgen]
pub fn to_save_state(gameboy: Gameboy) -> JsValue {
    gameboy.to_serializable().to_json()
}

#[wasm_bindgen]
pub fn load_state(val: &JsValue) -> Gameboy {
    SerializedGameboy::from_json(val)
}

#[wasm_bindgen]
pub fn init() {
    match console_log::init_with_level(Level::Debug) {
        Ok(_value) => info!("WASM Gameboy Emulator initialized"),
        Err(_err) => println!("Failed to initialize console logger"),
    }
}

#[wasm_bindgen]
pub fn opcode_name(opcode: u8) -> String {
    let result = match opcode {
        0x031 => "LD SP, $0xFFFE",
        0x0AF => "XOR A",
        0x021 => "LD HL, *2bytes",
        0x077 => "LD (HL), A",
        0x011 => "LD DE,*16bit",
        0x00E => "LD C, *1byte",
        0x03E => "LD A, *1byte",
        0x006 => "LD B, *1byte",
        0x002e => "LD L, *1byte",
        0x001e => "LD E, *1byte",
        0x0016 => "LD D, *1byte",
        0x07B => "LD A, E",
        0x07C => "LD A, H",
        0x07D => "LD A, L",
        0x078 => "LD A, B",
        0x01A => "LD A, (DE)",
        0x04F => "LD C,A",
        0x067 => "LD H,A",
        0x057 => "LD D,A",
        0x032 => "LD (HL-), A",
        0x022 => "LD (HL+), A",
        0x0f0 => "LD A, ($ff00+n)",
        0x0E2 => "LD ($ff00+C), A",
        0x0E0 => "LD ($ff00+n), A",
        0x0CB => "BIT (7, H)",
        0x017 => "RLA", // Rotate A left through Carry flag
        0x020 => "JR NZ,*one byte",
        0x028 => "JR Z,*",
        0x018 => "JR n",
        0x00C => "INC C",
        0x004 => "INC B",
        0x0CD => "CALL",
        0x0C9 => "RET",
        0x0C5 => "PUSH BC",
        0x0C1 => "POP nn",
        0x005 => "DEC B",
        0x00D => "DEC C",
        0x01D => "DEC E",
        0x03D => "DEC A",
        0x015 => "DEC D",
        0x013 => "INC DE",
        0x023 => "INC HL",
        0x024 => "INC H",
        0x0FE => "CP #",
        0x0BE => "CP (HL)",
        0x0EA => "LD (nn),A",
        0x090 => "SUB B",
        0x086 => "ADD (HL)",
        0x000 => "NOP",
        0x0CE => "ADC A,#",
        0x066 => "LD H,(hl)",
        0x0CC => "CALL Z, nn",
        0x00B => "DEB BC",
        0x003 => "INC BC",
        0x073 => "LD (HL),E",
        0x083 => "ADD A,E",
        0x008 => "LD (nn), SP",
        0x01F => "RRA",
        0x088 => " ADC A,B",
        0x089 => " ADC A,C ",
        0x06E => "LD L,(hl)",
        0x0E6 => "AND #",
        0x0C3 => " JP nn ",
        0x0f3 => "DI -Interrupts are disabled after instruction after DI is executed.",
        0x036 => "LD (HL),n",
        0x02a => " LDI A,(HL)",
        0x047 => " LD B,A",
        0x002 => "LD (BC), A",
        0x0fd => "No operation?",
        0x06d => "LD L,L",
        0x06c => "LD L,H",
        0x071 => "LD (HL), C",
        0x03c => "INC A",
        0x0e1 => "POP HL",
        0x055 => "LD D,L",
        0x056 => "LD D,(HL)",
        0x05a => "LD E,D",
        0x061 => "LD H,C",
        0x05b => "LD E,E",
        0x049 => "LD C,C",
        0x05e => "LD E,(HL)",
        0x058 => "LD E,B",
        0x05c => "LD E,H",
        0x051 => "LD D,C",
        0x050 => "LD D,B",
        0x04C => "LD C,H",
        0x04E => "LD C,(HL)",
        0x059 => "LD E,C",
        0x053 => "LD D,E",
        0x052 => "LD D,D",
        0x04d => "LD C,L",
        0x054 => "LD D,H",
        0x0d3 => "No operation?",
        0x0a6 => "AND (HL)",
        0x05d => "LD E,L",
        0x03a => "LD A, (HL-)",
        0x040 => "LD B,B",
        0x043 => "LD B,E",
        0x038 => "JR C,*one byte",
        0x0c2 => "JP NZ,nn",
        0x0f4 => "No operation?",
        0x07f => " LD A,A",
        0x074 => " LD (HL),H",
        0x075 => " LD (HL),L",
        0x072 => " LD (HL),D",
        0x076 => "HALT Power down CPU until interrupt occurs",
        0x079 => "LD A,C",
        0x0f1 => "POP AF",
        0x085 => "ADD A,L",
        0x0b1 => "OR C",
        0x03f => "CCF",
        0x042 => " LD B,D",
        0x081 => "ADD A,C",
        0x046 => "LD B,(HL)",
        0x0b5 => "OR L",
        0x070 => " LD (HL),B",
        0x048 => " LD C,B",
        0x0C4 => "CALL NZ, nn",
        0x069 => " LD L,C",
        0x06a => " LD L,D",
        0x06f => " LD L,A",
        0x0D1 => "POP DE",
        0x05f => " LD E,A",
        0x092 => " SUB D",
        0x097 => " SUB A",
        0x091 => " SUB C",
        0x093 => " SUB E",
        0x094 => " SUB H",
        0x095 => " SUB L",
        0x096 => " SUB (HL)",
        0x041 => " LD B,C",
        0x044 => "LD B,H",
        _other => "???",
    };

    String::from(result)
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    utils::set_panic_hook();
}
