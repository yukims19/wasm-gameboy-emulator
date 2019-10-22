use log::debug;
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
const SCREEN_PIXEL_NUM_PER_ROW: usize = 160;
const IMAGE_DATA_LENGTH_PER_PIXEL: usize = 4; //r,g,b,a
const PIXEL_NUM_PER_TILE_COL: usize = 8;
const BACKGROUND_PIXEL_NUM_PER_ROW: usize = 256;
const BYTES_PER_SPRITE: usize = 4;
const SPRITE_PIXEL_NUM_PER_ROW: usize = 8;

#[macro_use]
extern crate serde_derive;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

enum CycleRegister {
    CpuCycle,
    VramCycle,
    TimerCycle,
}

// enum LcdMode {
//     Vblank,
//     Hblank,
//     SearchOAM,
//     DataTransfer,
// }

#[wasm_bindgen]
struct Sprite {
    y: u8,
    x: u8,
    pattern_num: u8,
    attributes: u8,
    priority: bool,
    y_flip: bool,
    x_flip: bool,
    palette_num: bool,
}

#[wasm_bindgen]
pub struct Canvases {
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

    pub fn render_background_map_as_image_data(&mut self, gameboy: &mut Gameboy) {
        // if !gameboy.is_vblank() {
        //     return;
        // }

        let background_map = gameboy.bg_map(); //Tile index
        let char_map_vec = gameboy.bg_window_char_map_bytes(); //Tile data
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

        for ele in background_map {
            // Generate Tile Image data
            let tile_idx: usize = if gameboy.get_tile_data_selection() == 1 {
                ele as usize
            } else {
                ((ele as i8) as i16 + 128) as usize
            };

            let tile_bytes = &mut char_map_tiles_bytes[tile_idx];
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

    pub fn draw_screen_from_memory(&self, gameboy: &mut Gameboy) {
        if !gameboy.is_vblank() {
            return;
        }

        let is_lcd_enable = gameboy.is_lcd_display_enable();
        if !is_lcd_enable {
            return;
        }

        let background_map = gameboy.bg_map();
        let char_map_vec = gameboy.bg_window_char_map_bytes();

        //Generate background bytes from char map
        let mut background_pixels_row_rgba: Vec<Vec<u8>> = Vec::new();
        background_pixels_row_rgba.resize(256, Vec::new());

        let mut idx = 0;
        for ele in background_map {
            let tile_idx: usize = if gameboy.get_tile_data_selection() == 1 {
                ele as usize
            } else {
                ((ele as i8) as i16 + 128) as usize
            };
            let tile_start_idx = tile_idx as usize * BYTES_PER_TILE;
            let tile_end_idx = tile_start_idx + BYTES_PER_TILE;

            let tile_bytes = &char_map_vec[tile_start_idx..tile_end_idx];
            for i in (0..tile_bytes.len()).step_by(BYTES_PER_8_PIXEL) {
                let background_y = (idx / 32) * PIXEL_NUM_PER_TILE_COL + i / BYTES_PER_8_PIXEL;
                let low_bits = BitVec::from_bytes(&[tile_bytes[i]]);
                let high_bits = BitVec::from_bytes(&[tile_bytes[i + 1]]);

                for pixel_index in 0..8 {
                    let [r, g, b, a] = match (low_bits[pixel_index], high_bits[pixel_index]) {
                        (false, false) => [255, 255, 255, 255],
                        (false, true) => [191, 191, 191, 255],
                        (true, false) => [64, 64, 64, 255],
                        (true, true) => [0, 0, 0, 255],
                    };

                    background_pixels_row_rgba[background_y].push(r);
                    background_pixels_row_rgba[background_y].push(g);
                    background_pixels_row_rgba[background_y].push(b);
                    background_pixels_row_rgba[background_y].push(a);
                }
            }
            idx = idx + 1
        }

        let background_pixels_rgba_vec: Vec<u8> = background_pixels_row_rgba.concat();

        //Get screen bytes from background bytes
        let scroll_x = gameboy.get_scroll_x() as usize;
        let scroll_y = gameboy.get_scroll_y() as usize;
        let mut screen_pixels_rgba_vec: Vec<u8> = Vec::new();

        for screen_y in 0..144 {
            //TODO: need to handle x overflow
            let x = scroll_x;
            let y = if scroll_y + screen_y > 255 {
                scroll_y + screen_y - 256
            } else {
                scroll_y + screen_y
            };

            let start = y * BACKGROUND_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL
                + x * IMAGE_DATA_LENGTH_PER_PIXEL;
            let end = start + SCREEN_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;

            let screen_row_bytes = &background_pixels_rgba_vec[start..end];
            screen_pixels_rgba_vec.extend_from_slice(&screen_row_bytes);
        }

        //Drawing screen
        self.screen_canvas.clear_rect(
            0.0,
            0.0,
            self.screen_canvas.canvas().unwrap().width() as f64,
            self.screen_canvas.canvas().unwrap().height() as f64,
        );

        for screen_y in 0..144 {
            let start_row = screen_y * SCREEN_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;
            let end_row = start_row + SCREEN_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;
            let clamped_image_source =
                wasm_bindgen::Clamped(&mut screen_pixels_rgba_vec[start_row..end_row]);

            let pixel_row_image_data =
                web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped_image_source, 160, 1)
                    .unwrap();
            self.screen_canvas
                .put_image_data(&pixel_row_image_data, 0.0, screen_y as f64)
                .unwrap();
        }
    }

    pub fn draw_screen_with_obj(&self, gameboy: &mut Gameboy) {
        // if !gameboy.is_vblank() {
        //     return;
        // }

        // let is_lcd_enable = gameboy.is_lcd_display_enable();
        // if !is_lcd_enable {
        //     return;
        // }

        let background_map = gameboy.bg_map();
        let char_map_vec = gameboy.bg_window_char_map_bytes();

        //Generate background bytes from char map
        let mut background_pixels_row_rgba: Vec<Vec<u8>> = Vec::new();
        background_pixels_row_rgba.resize(256, Vec::new());

        let mut idx = 0;
        for ele in background_map {
            let tile_idx: usize = if gameboy.get_tile_data_selection() == 1 {
                ele as usize
            } else {
                ((ele as i8) as i16 + 128) as usize
            };
            let tile_start_idx = tile_idx as usize * BYTES_PER_TILE;
            let tile_end_idx = tile_start_idx + BYTES_PER_TILE;

            let tile_bytes = &char_map_vec[tile_start_idx..tile_end_idx];
            for i in (0..tile_bytes.len()).step_by(BYTES_PER_8_PIXEL) {
                let background_y = (idx / 32) * PIXEL_NUM_PER_TILE_COL + i / BYTES_PER_8_PIXEL;
                let low_bits = BitVec::from_bytes(&[tile_bytes[i]]);
                let high_bits = BitVec::from_bytes(&[tile_bytes[i + 1]]);

                for pixel_index in 0..8 {
                    let [r, g, b, a] = match (low_bits[pixel_index], high_bits[pixel_index]) {
                        (false, false) => [255, 255, 255, 255],
                        (false, true) => [191, 191, 191, 255],
                        (true, false) => [64, 64, 64, 255],
                        (true, true) => [0, 0, 0, 255],
                    };

                    background_pixels_row_rgba[background_y].push(r);
                    background_pixels_row_rgba[background_y].push(g);
                    background_pixels_row_rgba[background_y].push(b);
                    background_pixels_row_rgba[background_y].push(a);
                }
            }
            idx = idx + 1
        }

        let background_pixels_rgba_vec: Vec<u8> = background_pixels_row_rgba.concat();

        //Get screen bytes from background bytes
        let scroll_x = gameboy.get_scroll_x() as usize;
        let scroll_y = gameboy.get_scroll_y() as usize;
        let mut screen_pixels_rgba_vec: Vec<u8> = Vec::new();

        for screen_y in 0..144 {
            //TODO: need to handle x overflow
            let x = scroll_x;
            let y = if scroll_y + screen_y > 255 {
                scroll_y + screen_y - 256
            } else {
                scroll_y + screen_y
            };

            let start = y * BACKGROUND_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL
                + x * IMAGE_DATA_LENGTH_PER_PIXEL;
            let end = start + SCREEN_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;

            let screen_row_bytes = &background_pixels_rgba_vec[start..end];
            screen_pixels_rgba_vec.extend_from_slice(&screen_row_bytes);
        }

        //Drawing screen
        self.screen_canvas.clear_rect(
            0.0,
            0.0,
            self.screen_canvas.canvas().unwrap().width() as f64,
            self.screen_canvas.canvas().unwrap().height() as f64,
        );

        let blank_screen_with_sprites_rgba = self.get_blank_screen_pixel_with_sprites(gameboy);

        for screen_y in 0..144 {
            let start_row = screen_y * SCREEN_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;
            let end_row = start_row + SCREEN_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;

            let screen_row_rgba = &screen_pixels_rgba_vec[start_row..end_row];

            let blank_screen_with_sprites_rgba_row =
                &blank_screen_with_sprites_rgba[start_row..end_row];

            let mut result: Vec<u8> = Vec::new();

            //Overwrite screen with sprite rgb data
            for pixel_rgba in
                (0..blank_screen_with_sprites_rgba_row.len()).step_by(IMAGE_DATA_LENGTH_PER_PIXEL)
            {
                let r = blank_screen_with_sprites_rgba_row[pixel_rgba];
                let g = blank_screen_with_sprites_rgba_row[pixel_rgba + 1];
                let b = blank_screen_with_sprites_rgba_row[pixel_rgba + 2];
                match (r, g, b) {
                    (255, 255, 255) => {
                        result.extend_from_slice(
                            &screen_row_rgba[pixel_rgba..pixel_rgba + IMAGE_DATA_LENGTH_PER_PIXEL],
                        );
                    }
                    _other => {
                        result.extend_from_slice(
                            &blank_screen_with_sprites_rgba_row
                                [pixel_rgba..pixel_rgba + IMAGE_DATA_LENGTH_PER_PIXEL],
                        );
                    }
                }
            }

            let clamped_image_source = wasm_bindgen::Clamped(&mut result[..]);

            let pixel_row_image_data =
                web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped_image_source, 160, 1)
                    .unwrap();
            self.screen_canvas
                .put_image_data(&pixel_row_image_data, 0.0, screen_y as f64)
                .unwrap();
        }
    }

    fn get_blank_screen_pixel_with_sprites(&self, gameboy: &mut Gameboy) -> Vec<u8> {
        let char_map_vec = gameboy.obj_char_map_bytes(); //Tile data
        let mut tiles_rgba_vec = Vec::new();

        let screen_rbga_vec_length =
            SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * IMAGE_DATA_LENGTH_PER_PIXEL;

        let mut entire_screen_pixels_rgba = Vec::new();
        entire_screen_pixels_rgba.resize_with(screen_rbga_vec_length, || 255);

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

            tiles_rgba_vec.push(image_data_source);
        }

        //Fill in sprites data in blank temp_screen
        for obj in gameboy.all_sprites() {
            let tile_idx = obj.pattern_num as usize;
            let tile_rgba = &tiles_rgba_vec[tile_idx];

            for obj_row in 0..8 {
                let x = obj.x;
                let y = obj.y + obj_row;

                let start: usize =
                    y as usize * SPRITE_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL
                        + x as usize * IMAGE_DATA_LENGTH_PER_PIXEL;
                let end: usize = start + SPRITE_PIXEL_NUM_PER_ROW * IMAGE_DATA_LENGTH_PER_PIXEL;

                //Filling sprite tile row
                // entire_screen_pixels_rgba[start..end] =
                //     tile_rgba[obj_row as usize..obj_row as usize + 8 * IMAGE_DATA_LENGTH_PER_PIXEL];

                entire_screen_pixels_rgba.splice(
                    start..end,
                    tile_rgba[obj_row as usize..obj_row as usize + 8 * IMAGE_DATA_LENGTH_PER_PIXEL]
                        .iter()
                        .cloned(),
                );
            }
        }

        entire_screen_pixels_rgba
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

        ctx.set_image_smoothing_enabled(false);

        ctx
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
    z: bool,   //(0x80) if zero
    n: bool,   //(0x40) if subtraction
    h: bool,   //(0x20) if the lower half of the byte overflowed past 15
    c: bool,   //(0x10) if result over 255 or under 0
    ime: bool, //Interrupt Master Enable Flag
}

impl Flag {
    fn set_flag(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.z = z;
        self.n = n;
        self.h = h;
        self.c = c;
    }

    fn set_ime(&mut self, interupt_enabled: bool) {
        self.ime = interupt_enabled
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
    f: Flag,
    sp: u16,
    pc: u16,
}

#[wasm_bindgen]
impl Registers {
    fn xor_a_n(&mut self, n: u8) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let flag_c = false;

        let result = self.a ^ n;
        if result == 0 {
            flag_z = true;
        }
        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
        self.set_a(result);
    }

    fn and_a_n(&mut self, n: u8) {
        let mut flag_z = false;
        let flag_n = false;

        let flag_c = false;
        let result = self.a & n;
        if result == 0 {
            flag_z = true;
        };
        let flag_h = true;
        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
        self.set_a(result);
    }

    fn sbc_a_n(&mut self, n: u8) {
        let a = self.a as u16;
        let n = n as u16;
        let cf = self.f.c as u16;
        let r = a.wrapping_sub(n).wrapping_sub(cf);
        let result = r as u8;

        let flag_z = result == 0;
        let flag_h = (a ^ n ^ r) & 0x10 != 0;
        let flag_c = r & 0x100 != 0;
        let flag_n = true;

        self.set_a(result);
        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn adc_a_n(&mut self, n: u8) {
        let a = self.a as u16;
        let n = n as u16;
        let cf = self.f.c as u16;
        let r = a.wrapping_add(n).wrapping_add(cf);
        let result = r as u8;

        let flag_z = result == 0;
        let flag_h = (a ^ n ^ r) & 0x10 != 0;
        let flag_c = r & 0x100 != 0;
        let flag_n = false;

        self.set_a(result);
        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn add_a_n(&mut self, n: u8) {
        let mut flag_z = false;
        let flag_n = false;
        let mut flag_h = false;
        let mut flag_c = false;

        let value = self.a.wrapping_add(n);
        if value == 0 {
            flag_z = true;
        }

        if self.check_half_carry(self.a, n) {
            flag_h = true
        }
        if self.check_carry(self.a, n) {
            flag_c = true;
        }
        self.set_a(value);
        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn bit_b_r(&mut self, bit_idx: u8, register: u8) {
        let mut flag_z = false;
        let flag_n = false;

        let check_bits = match bit_idx {
            0 => 0b00000001,
            1 => 0b00000010,
            2 => 0b00000100,
            3 => 0b00001000,
            4 => 0b00010000,
            5 => 0b00100000,
            6 => 0b01000000,
            7 => 0b10000000,
            _ => {
                println!("Invalid bit index");
                std::process::exit(1)
            }
        };

        let bit = register & check_bits;
        if bit == 0 {
            flag_z = true;
        }
        let flag_h = true;
        let flag_c = self.f.c;
        self.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn add_signed_number(&self, unsigned: u16, signed: i8) -> u16 {
        let is_minus = signed.signum() == -1;
        let value = signed.abs() as u16;
        if is_minus {
            let result = unsigned - (value as u16);
            result
        } else {
            let result = unsigned + value as u16;
            result
        }
    }

    fn inc_pc(&mut self) {
        self.pc = self.pc + 1;
    }

    fn check_half_carry_u16_plus_i8(&self, unsigned: u16, signed: i8, sum_value: u16) -> bool {
        if signed >= 0 {
            ((unsigned & 0xF) + (signed as u16 & 0xF)) > 0xF
        } else {
            (sum_value & 0xF) <= (unsigned & 0xF)
        }
    }

    fn check_carry_u16_plus_i8(&self, unsigned: u16, signed: i8, sum_value: u16) -> bool {
        if signed >= 0 {
            ((unsigned & 0xFF) + signed as u16) > 0xFF
        } else {
            (sum_value & 0xFF) <= (unsigned & 0xFF)
        }
    }

    fn check_carry(&self, num_a: u8, num_b: u8) -> bool {
        (num_a & 0x00ff) as u16 + (num_b & 0x00ff) as u16 & 0x100 == 0x100
    }

    fn check_half_carry(&self, num_a: u8, num_b: u8) -> bool {
        (num_a & 0xf) + (num_b & 0xf) & 0x010 == 0x010
    }

    fn check_half_carry_sub(&self, num_a: u8, num_b: u8) -> bool {
        (num_a & 0xf) < (num_b & 0xf)
    }

    fn check_carry_two_bytes(&self, num_a: u16, num_b: u16) -> bool {
        (num_a & 0xffff) as u32 + (num_b & 0xffff) as u32 & 0x10000 == 0x10000
    }

    fn check_half_carry_two_bytes(&self, num_a: u16, num_b: u16) -> bool {
        (num_a & 0xfff) as u16 + (num_b & 0xfff) as u16 & 0x1000 == 0x1000
    }

    fn combine_two_bytes(&self, first_b: u8, second_b: u8) -> u16 {
        let two_bytes_value = ((first_b as u16) << 8) | second_b as u16;
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

    fn set_sp(&mut self, value: u16) {
        self.sp = value
    }

    fn get_f_as_byte(&self) -> u8 {
        let mut bv = BitVec::from_elem(8, false);
        if self.f.z {
            bv.set(0, true)
        }
        if self.f.n {
            bv.set(1, true)
        }
        if self.f.h {
            bv.set(2, true)
        }
        if self.f.c {
            bv.set(3, true)
        }

        let result = bv.to_bytes()[0];
        result
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
    timer_cycle_num: usize,
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
    timer_cycle_num: usize,
    timer: usize,
    cpu_clock: usize,
    is_running: bool,
    is_halt: bool,
    should_draw: bool,
    break_points: Vec<u16>,
    memory: Vec<u8>,
    cpu_paused: bool,
    cartridge: Vec<u8>,
    mbc: u8,
    rom_bank: u8,
    ram_bank: u8,
    ram_bank_memory: Vec<u8>,
    is_ram_enabled: bool,
    is_rom_banking_enabled: bool,
    joypad_state: u8,
}

#[wasm_bindgen]
impl Gameboy {
    fn execute_instruction(&mut self, opcode: u8) {
        let pointer = self.registers.pc as usize;
        let mut flag_z = false;
        let mut flag_n = false;
        let mut flag_h = false;
        let mut flag_c = false;

        match opcode {
            0x0CB => {
                match self.following_byte(pointer) {
                    0x047 => {
                        //BIT b(0),A -> 8
                        let bit = self.registers.a & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x040 => {
                        //BIT b(0),B -> 8
                        let bit = self.registers.b & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x041 => {
                        //BIT b(0),C -> 8
                        let bit = self.registers.c & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x042 => {
                        //BIT b(0),D -> 8
                        let bit = self.registers.d & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x043 => {
                        //BIT b(0),E -> 8
                        let bit = self.registers.e & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x044 => {
                        //BIT b(0),H -> 8
                        let bit = self.registers.h & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x045 => {
                        //BIT b(0),L -> 8
                        let bit = self.registers.l & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x046 => {
                        //BIT b(0),(HL) -> 16
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        let bit = value & 0b00000001;
                        if bit == 0 {
                            flag_z = true;
                        }
                        flag_h = true;
                        flag_c = self.registers.f.c;
                        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c)
                    }

                    0x048 => {
                        //BIT b(1),B -> 8
                        self.registers.bit_b_r(1, self.registers.b);
                    }
                    0x049 => {
                        //BIT b(1),C -> 8
                        self.registers.bit_b_r(1, self.registers.c);
                    }
                    0x04a => {
                        //BIT b(1),D -> 8
                        self.registers.bit_b_r(1, self.registers.d);
                    }
                    0x04b => {
                        //BIT b(1),E -> 8
                        self.registers.bit_b_r(1, self.registers.e);
                    }
                    0x04c => {
                        //BIT b(1),H -> 8
                        self.registers.bit_b_r(1, self.registers.h);
                    }
                    0x04d => {
                        //BIT b(1),L -> 8
                        self.registers.bit_b_r(1, self.registers.l);
                    }
                    0x04f => {
                        //BIT b(1),A -> 8
                        self.registers.bit_b_r(1, self.registers.a);
                    }
                    0x050 => {
                        //BIT b(2),B -> 8
                        self.registers.bit_b_r(2, self.registers.b);
                    }

                    0x051 => {
                        //BIT b(2),C -> 8
                        self.registers.bit_b_r(2, self.registers.c);
                    }

                    0x052 => {
                        //BIT b(2),D -> 8
                        self.registers.bit_b_r(2, self.registers.d);
                    }

                    0x053 => {
                        //BIT b(2),E -> 8
                        self.registers.bit_b_r(2, self.registers.e);
                    }

                    0x054 => {
                        //BIT b(2),H -> 8
                        self.registers.bit_b_r(2, self.registers.h);
                    }

                    0x055 => {
                        //BIT b(2),L -> 8
                        self.registers.bit_b_r(2, self.registers.l);
                    }
                    0x057 => {
                        //BIT b(2),A -> 8
                        self.registers.bit_b_r(2, self.registers.a);
                    }

                    0x058 => {
                        //BIT b(3),B -> 8
                        self.registers.bit_b_r(3, self.registers.b);
                    }
                    0x059 => {
                        //BIT b(3),C -> 8
                        self.registers.bit_b_r(3, self.registers.c);
                    }
                    0x05a => {
                        //BIT b(3),D -> 8
                        self.registers.bit_b_r(3, self.registers.d);
                    }
                    0x05b => {
                        //BIT b(3),E -> 8
                        self.registers.bit_b_r(3, self.registers.e);
                    }
                    0x05c => {
                        //BIT b(3),H -> 8
                        self.registers.bit_b_r(3, self.registers.h);
                    }
                    0x05d => {
                        //BIT b(3),L -> 8
                        self.registers.bit_b_r(3, self.registers.l);
                    }
                    0x05f => {
                        //BIT b(3),A -> 8
                        self.registers.bit_b_r(3, self.registers.a);
                    }

                    0x060 => {
                        //BIT b(4),B -> 8
                        self.registers.bit_b_r(4, self.registers.b);
                    }

                    0x061 => {
                        //BIT b(4),C -> 8
                        self.registers.bit_b_r(4, self.registers.c);
                    }

                    0x062 => {
                        //BIT b(4),D -> 8
                        self.registers.bit_b_r(4, self.registers.d);
                    }

                    0x063 => {
                        //BIT b(4),E -> 8
                        self.registers.bit_b_r(4, self.registers.e);
                    }

                    0x064 => {
                        //BIT b(4),H -> 8
                        self.registers.bit_b_r(4, self.registers.h);
                    }

                    0x065 => {
                        //BIT b(4),L -> 8
                        self.registers.bit_b_r(4, self.registers.l);
                    }
                    0x067 => {
                        //BIT b(4),A -> 8
                        self.registers.bit_b_r(4, self.registers.a);
                    }

                    0x068 => {
                        //BIT b(5),B -> 8
                        self.registers.bit_b_r(5, self.registers.b);
                    }
                    0x069 => {
                        //BIT b(5),C -> 8
                        self.registers.bit_b_r(5, self.registers.c);
                    }
                    0x06a => {
                        //BIT b(5),D -> 8
                        self.registers.bit_b_r(5, self.registers.d);
                    }
                    0x06b => {
                        //BIT b(5),E -> 8
                        self.registers.bit_b_r(5, self.registers.e);
                    }
                    0x06c => {
                        //BIT b(5),H -> 8
                        self.registers.bit_b_r(5, self.registers.h);
                    }
                    0x06d => {
                        //BIT b(5),L -> 8
                        self.registers.bit_b_r(5, self.registers.l);
                    }
                    0x06f => {
                        //BIT b(5),A -> 8
                        self.registers.bit_b_r(5, self.registers.a);
                    }

                    0x070 => {
                        //BIT b(6),B -> 8
                        self.registers.bit_b_r(6, self.registers.b);
                    }

                    0x071 => {
                        //BIT b(6),C -> 8
                        self.registers.bit_b_r(6, self.registers.c);
                    }

                    0x072 => {
                        //BIT b(6),D -> 8
                        self.registers.bit_b_r(6, self.registers.d);
                    }

                    0x073 => {
                        //BIT b(6),E -> 8
                        self.registers.bit_b_r(6, self.registers.e);
                    }

                    0x074 => {
                        //BIT b(6),H -> 8
                        self.registers.bit_b_r(6, self.registers.h);
                    }

                    0x075 => {
                        //BIT b(6),L -> 8
                        self.registers.bit_b_r(6, self.registers.l);
                    }
                    0x077 => {
                        //BIT b(6),A -> 8
                        self.registers.bit_b_r(6, self.registers.a);
                    }

                    0x078 => {
                        //BIT b(7),B -> 8
                        self.registers.bit_b_r(7, self.registers.b);
                    }
                    0x079 => {
                        //BIT b(7),C -> 8
                        self.registers.bit_b_r(7, self.registers.c);
                    }
                    0x07a => {
                        //BIT b(7),D -> 8
                        self.registers.bit_b_r(7, self.registers.d);
                    }
                    0x07b => {
                        //BIT b(7),E -> 8
                        self.registers.bit_b_r(7, self.registers.e);
                    }
                    0x07c => {
                        //BIT b(7),H -> 8
                        self.registers.bit_b_r(7, self.registers.h);
                    }
                    0x07d => {
                        //BIT b(7),L -> 8
                        self.registers.bit_b_r(7, self.registers.l);
                    }
                    0x07f => {
                        //BIT b(7),A -> 8
                        self.registers.bit_b_r(7, self.registers.a);
                    }

                    0x080 => {
                        //RES b(0),B -> 8
                        self.res_b_r(0, self.registers.b, "b", None);
                    }

                    0x081 => {
                        //RES b(0),C -> 8
                        self.res_b_r(0, self.registers.c, "c", None);
                    }

                    0x082 => {
                        //RES b(0),D -> 8
                        self.res_b_r(0, self.registers.d, "d", None);
                    }

                    0x083 => {
                        //RES b(0),E -> 8
                        self.res_b_r(0, self.registers.e, "e", None);
                    }

                    0x084 => {
                        //RES b(0),H -> 8
                        self.res_b_r(0, self.registers.h, "h", None);
                    }

                    0x085 => {
                        //RES b(0),L -> 8
                        self.res_b_r(0, self.registers.l, "l", None);
                    }
                    0x087 => {
                        //RES b(0),A -> 8
                        self.res_b_r(0, self.registers.a, "a", None);
                    }

                    0x088 => {
                        //RES b(1),B -> 8
                        self.res_b_r(1, self.registers.b, "b", None);
                    }
                    0x089 => {
                        //RES b(1),C -> 8
                        self.res_b_r(1, self.registers.c, "c", None);
                    }
                    0x08a => {
                        //RES b(1),D -> 8
                        self.res_b_r(1, self.registers.d, "d", None);
                    }
                    0x08b => {
                        //RES b(1),E -> 8
                        self.res_b_r(1, self.registers.e, "e", None);
                    }
                    0x08c => {
                        //RES b(1),H -> 8
                        self.res_b_r(1, self.registers.h, "h", None);
                    }
                    0x08d => {
                        //RES b(1),L -> 8
                        self.res_b_r(1, self.registers.l, "l", None);
                    }
                    0x08f => {
                        //RES b(1),A -> 8
                        self.res_b_r(1, self.registers.a, "a", None);
                    }

                    0x090 => {
                        //RES b(2),B -> 8
                        self.res_b_r(2, self.registers.b, "b", None);
                    }

                    0x091 => {
                        //RES b(2),C -> 8
                        self.res_b_r(2, self.registers.c, "c", None);
                    }

                    0x092 => {
                        //RES b(2),D -> 8
                        self.res_b_r(2, self.registers.d, "d", None);
                    }

                    0x093 => {
                        //RES b(2),E -> 8
                        self.res_b_r(2, self.registers.e, "e", None);
                    }

                    0x094 => {
                        //RES b(2),H -> 8
                        self.res_b_r(2, self.registers.h, "h", None);
                    }

                    0x095 => {
                        //RES b(2),L -> 8
                        self.res_b_r(2, self.registers.l, "l", None);
                    }
                    0x097 => {
                        //RES b(2),A -> 8
                        self.res_b_r(2, self.registers.a, "a", None);
                    }

                    0x098 => {
                        //RES b(3),B -> 8
                        self.res_b_r(3, self.registers.b, "b", None);
                    }
                    0x099 => {
                        //RES b(3),C -> 8
                        self.res_b_r(3, self.registers.c, "c", None);
                    }
                    0x09a => {
                        //RES b(3),D -> 8
                        self.res_b_r(3, self.registers.d, "d", None);
                    }
                    0x09b => {
                        //RES b(3),E -> 8
                        self.res_b_r(3, self.registers.e, "e", None);
                    }
                    0x09c => {
                        //RES b(3),H -> 8
                        self.res_b_r(3, self.registers.h, "h", None);
                    }
                    0x09d => {
                        //RES b(3),L -> 8
                        self.res_b_r(3, self.registers.l, "l", None);
                    }
                    0x09f => {
                        //RES b(3),A -> 8
                        self.res_b_r(3, self.registers.a, "a", None);
                    }

                    0x0a0 => {
                        //RES b(4),B -> 8
                        self.res_b_r(4, self.registers.b, "b", None);
                    }

                    0x0a1 => {
                        //RES b(4),C -> 8
                        self.res_b_r(4, self.registers.c, "c", None);
                    }

                    0x0a2 => {
                        //RES b(4),D -> 8
                        self.res_b_r(4, self.registers.d, "d", None);
                    }

                    0x0a3 => {
                        //RES b(4),E -> 8
                        self.res_b_r(4, self.registers.e, "e", None);
                    }

                    0x0a4 => {
                        //RES b(4),H -> 8
                        self.res_b_r(4, self.registers.h, "h", None);
                    }

                    0x0a5 => {
                        //RES b(4),L -> 8
                        self.res_b_r(4, self.registers.l, "l", None);
                    }
                    0x0a7 => {
                        //RES b(4),A -> 8
                        self.res_b_r(4, self.registers.a, "a", None);
                    }

                    0x0a8 => {
                        //RES b(5),B -> 8
                        self.res_b_r(5, self.registers.b, "b", None);
                    }
                    0x0a9 => {
                        //RES b(5),C -> 8
                        self.res_b_r(5, self.registers.c, "c", None);
                    }
                    0x0aa => {
                        //RES b(5),D -> 8
                        self.res_b_r(5, self.registers.d, "d", None);
                    }
                    0x0ab => {
                        //RES b(5),E -> 8
                        self.res_b_r(5, self.registers.e, "e", None);
                    }
                    0x0ac => {
                        //RES b(5),H -> 8
                        self.res_b_r(5, self.registers.h, "h", None);
                    }
                    0x0ad => {
                        //RES b(5),L -> 8
                        self.res_b_r(5, self.registers.l, "l", None);
                    }
                    0x0af => {
                        //RES b(5),A -> 8
                        self.res_b_r(5, self.registers.a, "a", None);
                    }

                    0x0b0 => {
                        //RES b(6),B -> 8
                        self.res_b_r(6, self.registers.b, "b", None);
                    }

                    0x0b1 => {
                        //RES b(6),C -> 8
                        self.res_b_r(6, self.registers.c, "c", None);
                    }

                    0x0b2 => {
                        //RES b(6),D -> 8
                        self.res_b_r(6, self.registers.d, "d", None);
                    }

                    0x0b3 => {
                        //RES b(6),E -> 8
                        self.res_b_r(6, self.registers.e, "e", None);
                    }

                    0x0b4 => {
                        //RES b(6),H -> 8
                        self.res_b_r(6, self.registers.h, "h", None);
                    }

                    0x0b5 => {
                        //RES b(6),L -> 8
                        self.res_b_r(6, self.registers.l, "l", None);
                    }
                    0x0b7 => {
                        //RES b(6),A -> 8
                        self.res_b_r(6, self.registers.a, "a", None);
                    }

                    0x0b8 => {
                        //RES b(7),B -> 8
                        self.res_b_r(7, self.registers.b, "b", None);
                    }
                    0x0b9 => {
                        //RES b(7),C -> 8
                        self.res_b_r(7, self.registers.c, "c", None);
                    }
                    0x0ba => {
                        //RES b(7),D -> 8
                        self.res_b_r(7, self.registers.d, "d", None);
                    }
                    0x0bb => {
                        //RES b(7),E -> 8
                        self.res_b_r(7, self.registers.e, "e", None);
                    }
                    0x0bc => {
                        //RES b(7),H -> 8
                        self.res_b_r(7, self.registers.h, "h", None);
                    }
                    0x0bd => {
                        //RES b(7),L -> 8
                        self.res_b_r(7, self.registers.l, "l", None);
                    }
                    0x0bf => {
                        //RES b(7),A -> 8
                        self.res_b_r(7, self.registers.a, "a", None);
                    }

                    0x0c0 => {
                        //SET b(0),B -> 8
                        self.set_b_r(0, self.registers.b, "b", None);
                    }

                    0x0c1 => {
                        //SET b(0),C -> 8
                        self.set_b_r(0, self.registers.c, "c", None);
                    }

                    0x0c2 => {
                        //SET b(0),D -> 8
                        self.set_b_r(0, self.registers.d, "d", None);
                    }

                    0x0c3 => {
                        //SET b(0),E -> 8
                        self.set_b_r(0, self.registers.e, "e", None);
                    }

                    0x0c4 => {
                        //SET b(0),H -> 8
                        self.set_b_r(0, self.registers.h, "h", None);
                    }

                    0x0c5 => {
                        //SET b(0),L -> 8
                        self.set_b_r(0, self.registers.l, "l", None);
                    }
                    0x0c7 => {
                        //SET b(0),A -> 8
                        self.set_b_r(0, self.registers.a, "a", None);
                    }

                    0x0c8 => {
                        //SET b(1),B -> 8
                        self.set_b_r(1, self.registers.b, "b", None);
                    }
                    0x0c9 => {
                        //SET b(1),C -> 8
                        self.set_b_r(1, self.registers.c, "c", None);
                    }
                    0x0ca => {
                        //SET b(1),D -> 8
                        self.set_b_r(1, self.registers.d, "d", None);
                    }
                    0x0cb => {
                        //SET b(1),E -> 8
                        self.set_b_r(1, self.registers.e, "e", None);
                    }
                    0x0cc => {
                        //SET b(1),H -> 8
                        self.set_b_r(1, self.registers.h, "h", None);
                    }
                    0x0cd => {
                        //SET b(1),L -> 8
                        self.set_b_r(1, self.registers.l, "l", None);
                    }
                    0x0cf => {
                        //SET b(1),A -> 8
                        self.set_b_r(1, self.registers.a, "a", None);
                    }

                    0x0d0 => {
                        //SET b(2),B -> 8
                        self.set_b_r(2, self.registers.b, "b", None);
                    }

                    0x0d1 => {
                        //SET b(2),C -> 8
                        self.set_b_r(2, self.registers.c, "c", None);
                    }

                    0x0d2 => {
                        //SET b(2),D -> 8
                        self.set_b_r(2, self.registers.d, "d", None);
                    }

                    0x0d3 => {
                        //SET b(2),E -> 8
                        self.set_b_r(2, self.registers.e, "e", None);
                    }

                    0x0d4 => {
                        //SET b(2),H -> 8
                        self.set_b_r(2, self.registers.h, "h", None);
                    }

                    0x0d5 => {
                        //SET b(2),L -> 8
                        self.set_b_r(2, self.registers.l, "l", None);
                    }
                    0x0d7 => {
                        //SET b(2),A -> 8
                        self.set_b_r(2, self.registers.a, "a", None);
                    }

                    0x0d8 => {
                        //SET b(3),B -> 8
                        self.set_b_r(3, self.registers.b, "b", None);
                    }
                    0x0d9 => {
                        //SET b(3),C -> 8
                        self.set_b_r(3, self.registers.c, "c", None);
                    }
                    0x0da => {
                        //SET b(3),D -> 8
                        self.set_b_r(3, self.registers.d, "d", None);
                    }
                    0x0db => {
                        //SET b(3),E -> 8
                        self.set_b_r(3, self.registers.e, "e", None);
                    }
                    0x0dc => {
                        //SET b(3),H -> 8
                        self.set_b_r(3, self.registers.h, "h", None);
                    }
                    0x0dd => {
                        //SET b(3),L -> 8
                        self.set_b_r(3, self.registers.l, "l", None);
                    }
                    0x0df => {
                        //SET b(3),A -> 8
                        self.set_b_r(3, self.registers.a, "a", None);
                    }

                    0x0e0 => {
                        //SET b(4),B -> 8
                        self.set_b_r(4, self.registers.b, "b", None);
                    }

                    0x0e1 => {
                        //SET b(4),C -> 8
                        self.set_b_r(4, self.registers.c, "c", None);
                    }

                    0x0e2 => {
                        //SET b(4),D -> 8
                        self.set_b_r(4, self.registers.d, "d", None);
                    }

                    0x0e3 => {
                        //SET b(4),E -> 8
                        self.set_b_r(4, self.registers.e, "e", None);
                    }

                    0x0e4 => {
                        //SET b(4),H -> 8
                        self.set_b_r(4, self.registers.h, "h", None);
                    }

                    0x0e5 => {
                        //SET b(4),L -> 8
                        self.set_b_r(4, self.registers.l, "l", None);
                    }
                    0x0e7 => {
                        //SET b(4),A -> 8
                        self.set_b_r(4, self.registers.a, "a", None);
                    }

                    0x0e8 => {
                        //SET b(5),B -> 8
                        self.set_b_r(5, self.registers.b, "b", None);
                    }
                    0x0e9 => {
                        //SET b(5),C -> 8
                        self.set_b_r(5, self.registers.c, "c", None);
                    }
                    0x0ea => {
                        //SET b(5),D -> 8
                        self.set_b_r(5, self.registers.d, "d", None);
                    }
                    0x0eb => {
                        //SET b(5),E -> 8
                        self.set_b_r(5, self.registers.e, "e", None);
                    }
                    0x0ec => {
                        //SET b(5),H -> 8
                        self.set_b_r(5, self.registers.h, "h", None);
                    }
                    0x0ed => {
                        //SET b(5),L -> 8
                        self.set_b_r(5, self.registers.l, "l", None);
                    }
                    0x0ef => {
                        //SET b(5),A -> 8
                        self.set_b_r(5, self.registers.a, "a", None);
                    }
                    0x0f0 => {
                        //SET b(6),B -> 8
                        self.set_b_r(6, self.registers.b, "b", None);
                    }

                    0x0f1 => {
                        //SET b(6),C -> 8
                        self.set_b_r(6, self.registers.c, "c", None);
                    }

                    0x0f2 => {
                        //SET b(6),D -> 8
                        self.set_b_r(6, self.registers.d, "d", None);
                    }

                    0x0f3 => {
                        //SET b(6),E -> 8
                        self.set_b_r(6, self.registers.e, "e", None);
                    }

                    0x0f4 => {
                        //SET b(6),H -> 8
                        self.set_b_r(6, self.registers.h, "h", None);
                    }

                    0x0f5 => {
                        //SET b(6),L -> 8
                        self.set_b_r(6, self.registers.l, "l", None);
                    }
                    0x0f7 => {
                        //SET b(6),A -> 8
                        self.set_b_r(6, self.registers.a, "a", None);
                    }

                    0x0f8 => {
                        //SET b(7),B -> 8
                        self.set_b_r(7, self.registers.b, "b", None);
                    }
                    0x0f9 => {
                        //SET b(7),C -> 8
                        self.set_b_r(7, self.registers.c, "c", None);
                    }
                    0x0fa => {
                        //SET b(7),D -> 8
                        self.set_b_r(7, self.registers.d, "d", None);
                    }
                    0x0fb => {
                        //SET b(7),E -> 8
                        self.set_b_r(7, self.registers.e, "e", None);
                    }
                    0x0fc => {
                        //SET b(7),H -> 8
                        self.set_b_r(7, self.registers.h, "h", None);
                    }
                    0x0fd => {
                        //SET b(7),L -> 8
                        self.set_b_r(7, self.registers.l, "l", None);
                    }

                    0x0ff => {
                        //SET b(7),A -> 8
                        self.set_b_r(7, self.registers.a, "a", None);
                    }

                    0x04e => {
                        //BIT b(1),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(1, address_value);
                    }

                    0x056 => {
                        //BIT b(2),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(2, address_value);
                    }

                    0x05e => {
                        //BIT b(3),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(3, address_value);
                    }

                    0x066 => {
                        //BIT b(4),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(4, address_value);
                    }

                    0x06e => {
                        //BIT b(5),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(5, address_value);
                    }

                    0x076 => {
                        //BIT b(6),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(6, address_value);
                    }

                    0x07e => {
                        //BIT b(7),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.registers.bit_b_r(7, address_value);
                    }

                    0x086 => {
                        //RES b(0),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(0, address_value, "h_l", Some(h_l));
                    }

                    0x08e => {
                        //RES b(1),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(1, address_value, "h_l", Some(h_l));
                    }

                    0x096 => {
                        //RES b(2),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(2, address_value, "h_l", Some(h_l));
                    }

                    0x09e => {
                        //RES b(3),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(3, address_value, "h_l", Some(h_l));
                    }

                    0x0a6 => {
                        //RES b(4),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(4, address_value, "h_l", Some(h_l));
                    }

                    0x0ae => {
                        //RES b(5),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(5, address_value, "h_l", Some(h_l));
                    }

                    0x0b6 => {
                        //RES b(6),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(6, address_value, "h_l", Some(h_l));
                    }

                    0x0be => {
                        //RES b(7),(HL) -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.res_b_r(7, address_value, "h_l", Some(h_l));
                    }

                    0x0c6 => {
                        //SET b(0),(HL) -> 16
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(0, address_value, "h_l", Some(h_l));
                    }

                    0x0ce => {
                        //SET b(1),(HL) -> 16
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(1, address_value, "h_l", Some(h_l));
                    }

                    0x0d6 => {
                        //SET b(2),(HL) -> 16
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(2, address_value, "h_l", Some(h_l));
                    }

                    0x0de => {
                        //SET b(3),(HL) -> 16
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(3, address_value, "h_l", Some(h_l));
                    }

                    0x0e6 => {
                        //SET b(4),L -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(4, address_value, "h_l", Some(h_l));
                    }

                    0x0ee => {
                        //SET b(5),L -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(5, address_value, "h_l", Some(h_l));
                    }

                    0x0f6 => {
                        //SET b(6),L -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(6, address_value, "h_l", Some(h_l));
                    }

                    0x0fe => {
                        //SET b(7),L -> 8
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let address_value = self.read_memory(h_l);
                        self.set_b_r(7, address_value, "h_l", Some(h_l));
                    }

                    0x007 => {
                        //RLC A
                        self.rlc(self.registers.a, "a", None);
                    }
                    0x000 => {
                        //RLC B
                        self.rlc(self.registers.b, "b", None);
                    }
                    0x001 => {
                        //RLC C
                        self.rlc(self.registers.c, "c", None);
                    }
                    0x002 => {
                        //RLC D
                        self.rlc(self.registers.d, "d", None);
                    }
                    0x003 => {
                        //RLC E
                        self.rlc(self.registers.e, "e", None);
                    }
                    0x004 => {
                        //RLC H
                        self.rlc(self.registers.h, "h", None);
                    }
                    0x005 => {
                        //RLC L
                        self.rlc(self.registers.l, "l", None);
                    }
                    0x006 => {
                        //RLC (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.rlc(value, "h_l", Some(h_l));
                    }

                    0x00f => {
                        //RRC A
                        self.rrc(self.registers.a, "a", None);
                    }
                    0x008 => {
                        //RRC B
                        self.rrc(self.registers.b, "b", None);
                    }
                    0x009 => {
                        //RRC C
                        self.rrc(self.registers.c, "c", None);
                    }
                    0x00a => {
                        //RRC D
                        self.rrc(self.registers.d, "d", None);
                    }
                    0x00b => {
                        //RRC E
                        self.rrc(self.registers.e, "e", None);
                    }
                    0x00c => {
                        //RRC H
                        self.rrc(self.registers.h, "h", None);
                    }
                    0x00d => {
                        //RRC L
                        self.rrc(self.registers.l, "l", None);
                    }
                    0x00e => {
                        //RRC (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.rrc(value, "h_l", Some(h_l));
                    }

                    0x017 => {
                        //RL A
                        self.rl(self.registers.a, "a", None);
                    }
                    0x010 => {
                        //RL B
                        self.rl(self.registers.b, "b", None);
                    }
                    0x011 => {
                        //RL C
                        self.rl(self.registers.c, "c", None);
                    }
                    0x012 => {
                        //RL D
                        self.rl(self.registers.d, "d", None);
                    }
                    0x013 => {
                        //RL E
                        self.rl(self.registers.e, "e", None);
                    }
                    0x014 => {
                        //RL H
                        self.rl(self.registers.h, "h", None);
                    }
                    0x015 => {
                        //RL L
                        self.rl(self.registers.l, "l", None);
                    }
                    0x016 => {
                        //RL (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.rl(value, "h_l", Some(h_l));
                    }

                    0x01f => {
                        //RR A
                        self.rr(self.registers.a, "a", None);
                    }
                    0x018 => {
                        //RR B
                        self.rr(self.registers.b, "b", None);
                    }
                    0x019 => {
                        //RR C
                        self.rr(self.registers.c, "c", None);
                    }
                    0x01a => {
                        //RR D
                        self.rr(self.registers.d, "d", None);
                    }
                    0x01b => {
                        //RR E
                        self.rr(self.registers.e, "e", None);
                    }
                    0x01c => {
                        //RR H
                        self.rr(self.registers.h, "h", None);
                    }
                    0x01d => {
                        //RR L
                        self.rr(self.registers.l, "l", None);
                    }
                    0x01e => {
                        //RR (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.rr(value, "h_l", Some(h_l));
                    }

                    0x027 => {
                        //SLA A
                        self.sla(self.registers.a, "a", None);
                    }
                    0x020 => {
                        //SLA B
                        self.sla(self.registers.b, "b", None);
                    }
                    0x021 => {
                        //SLA C
                        self.sla(self.registers.c, "c", None);
                    }
                    0x022 => {
                        //SLA D
                        self.sla(self.registers.d, "d", None);
                    }
                    0x023 => {
                        //SLA E
                        self.sla(self.registers.e, "e", None);
                    }
                    0x024 => {
                        //SLA H
                        self.sla(self.registers.h, "h", None);
                    }
                    0x025 => {
                        //SLA L
                        self.sla(self.registers.l, "l", None);
                    }
                    0x026 => {
                        //SLA (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.sla(value, "h_l", Some(h_l));
                    }

                    0x02f => {
                        //SRA A
                        self.sra(self.registers.a, "a", None);
                    }
                    0x028 => {
                        //SRA B
                        self.sra(self.registers.b, "b", None);
                    }
                    0x029 => {
                        //SRA C
                        self.sra(self.registers.c, "c", None);
                    }
                    0x02a => {
                        //SRA D
                        self.sra(self.registers.d, "d", None);
                    }
                    0x02b => {
                        //SRA E
                        self.sra(self.registers.e, "e", None);
                    }
                    0x02c => {
                        //SRA H
                        self.sra(self.registers.h, "h", None);
                    }
                    0x02d => {
                        //SRA L
                        self.sra(self.registers.l, "l", None);
                    }
                    0x02e => {
                        //SRA (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.sra(value, "h_l", Some(h_l));
                    }

                    0x037 => {
                        //SWAP A
                        self.swap(self.registers.a, "a", None);
                    }
                    0x030 => {
                        //SWAP B
                        self.swap(self.registers.b, "b", None);
                    }
                    0x031 => {
                        //SWAP C
                        self.swap(self.registers.c, "c", None);
                    }
                    0x032 => {
                        //SWAP D
                        self.swap(self.registers.d, "d", None);
                    }
                    0x033 => {
                        //SWAP E
                        self.swap(self.registers.e, "e", None);
                    }
                    0x034 => {
                        //SWAP H
                        self.swap(self.registers.h, "h", None);
                    }
                    0x035 => {
                        //SWAP L
                        self.swap(self.registers.l, "l", None);
                    }
                    0x036 => {
                        //SWAP (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.swap(value, "h_l", Some(h_l));
                    }
                    0x03f => {
                        //SRL A
                        self.srl(self.registers.a, "a", None);
                    }
                    0x038 => {
                        //SRL B
                        self.srl(self.registers.b, "b", None);
                    }
                    0x039 => {
                        //SRL C
                        self.srl(self.registers.c, "c", None);
                    }
                    0x03a => {
                        //SRL D
                        self.srl(self.registers.d, "d", None);
                    }
                    0x03b => {
                        //SRL E
                        self.srl(self.registers.e, "e", None);
                    }
                    0x03c => {
                        //SRL H
                        self.srl(self.registers.h, "h", None);
                    }
                    0x03d => {
                        //SRL L
                        self.srl(self.registers.l, "l", None);
                    }
                    0x03e => {
                        //SRL (HL)
                        let h_l = self
                            .registers
                            .combine_two_bytes(self.registers.h, self.registers.l);
                        let value = self.read_memory(h_l);
                        self.srl(value, "h_l", Some(h_l));
                    }
                }

                self.registers.inc_pc();
            }

            0x031 => {
                //LD SP, nn
                let value = self.following_two_bytes(pointer);
                self.registers.set_sp(value);
                self.registers.inc_pc();
            }

            0x021 => {
                //LD HL, *2bytes
                let value = self.following_two_bytes(self.registers.pc as usize);
                self.registers.set_hl(value);
                self.registers.inc_pc();
            }
            0x077 => {
                //LD (HL), A
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.a);
                self.registers.inc_pc();
            }
            0x011 => {
                //LD DE,*16bit
                let value = self.following_two_bytes(pointer);
                self.registers.set_de(value);
                self.registers.inc_pc();
            }
            0x00E => {
                //LD C, *1byte
                let value = self.following_byte(pointer);
                self.registers.set_c(value);
                self.registers.inc_pc();
            }
            0x03E => {
                //LD A, *1byte
                let value = self.following_byte(pointer);
                self.registers.set_a(value);
                self.registers.inc_pc();
            }
            0x006 => {
                //LD B, *1byte
                let value = self.following_byte(pointer);
                self.registers.set_b(value);
                self.registers.inc_pc();
            }
            0x002e => {
                //LD L, *1byte
                let value = self.following_byte(pointer);
                self.registers.set_l(value);
                self.registers.inc_pc();
            }
            0x001e => {
                //LD E, *1byte
                let value = self.following_byte(pointer);
                self.registers.set_e(value);
                self.registers.inc_pc();
            }
            0x0016 => {
                //LD D, *1byte
                let value = self.following_byte(pointer);
                self.registers.set_d(value);
                self.registers.inc_pc();
            }
            0x07B => {
                //LD A, E
                self.registers.set_a(self.registers.e);
                self.registers.inc_pc();
            }
            0x07C => {
                //LD A, H
                self.registers.set_a(self.registers.h);
                self.registers.inc_pc();
            }
            0x07D => {
                //LD A, L
                self.registers.set_a(self.registers.l);
                self.registers.inc_pc();
            }
            0x078 => {
                //LD A, B
                self.registers.set_a(self.registers.b);
                self.registers.inc_pc();
            }
            0x01A => {
                //LD A, (DE)
                let d_e = self
                    .registers
                    .combine_two_bytes(self.registers.d, self.registers.e);
                let value = self.read_memory(d_e);

                self.registers.set_a(value as u8);
                self.registers.inc_pc();
            }

            0x032 => {
                //LD (HL-), A
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.a);
                self.registers.set_hl(h_l.wrapping_sub(1) as u16);
                self.registers.inc_pc();
            }
            0x022 => {
                //LD (HL+), A
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.a);
                self.registers.set_hl(h_l + 1);
                self.registers.inc_pc();
            }
            0x0f0 => {
                //LD A, ($ff00+n)

                let following_byte = self.following_byte(pointer);
                if (following_byte == 85) {
                    info!("What's wroning?");
                }
                let offset = 0xff00 + following_byte as u16;
                let value = self.read_memory(offset);
                self.registers.set_a(value);
                self.registers.inc_pc();
            }
            0x0E2 => {
                //LD ($ff00+C), A
                self.write_memory(0xFF00 + self.registers.c as u16, self.registers.a);
                self.registers.inc_pc();
            }
            0x0E0 => {
                //LD ($ff00+n), A
                let memory_add = 0xFF00 + self.following_byte(pointer) as u16;
                self.write_memory(memory_add, self.registers.a);
                self.registers.inc_pc();
            }

            0x017 => {
                // RLA: Rotate A left through Carry flag.
                let shifted_value = self.registers.a << 1;
                let result = shifted_value
                    | match self.registers.f.c {
                        true => 0b00000001,
                        false => 0b00000000,
                    };

                if self.registers.a & 0b10000000 == 0b10000000 {
                    flag_c = true
                } else {
                    flag_c = false
                }
                self.registers.set_a(result);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x020 => {
                //JR NZ,*one byte
                if !self.registers.f.z {
                    let n_param = self.following_byte(pointer);
                    self.registers.inc_pc();
                    let destination = self
                        .registers
                        .add_signed_number(self.registers.pc, n_param as i8);
                    self.registers.set_pc(destination);
                } else {
                    self.registers.inc_pc();
                    self.registers.inc_pc();
                }
            }
            0x028 => {
                //JR Z,*
                if self.registers.f.z {
                    let value = self.following_byte(pointer);
                    self.registers.inc_pc();
                    let address = self
                        .registers
                        .add_signed_number(self.registers.pc, value as i8);
                    self.registers.set_pc(address);
                } else {
                    self.registers.inc_pc();
                    self.registers.inc_pc();
                }
            }
            0x018 => {
                //JR n
                let value = self.following_byte(pointer);
                self.registers.inc_pc();
                let address = self
                    .registers
                    .add_signed_number(self.registers.pc, value as i8);
                self.registers.set_pc(address);
            }
            0x00C => {
                //INC C
                let value = self.registers.c + 1;
                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(self.registers.c, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_c(value);
                self.registers.inc_pc();
            }
            0x004 => {
                //INC B
                let value = self.registers.b + 1;

                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(self.registers.b, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_b(value);
                self.registers.inc_pc();
            }
            0x0CD => {
                //CALL
                let next_two_bytes = self.following_two_bytes(pointer);
                let next_instruction_address = self.registers.pc + 1;
                self.push_stack(next_instruction_address);
                self.registers.set_pc(next_two_bytes);
            }
            0x0C9 => {
                //RET
                let address = self.pop_stack();
                self.registers.set_pc(address);
            }

            0x0C5 => {
                //PUSH BC
                let bc_value = self
                    .registers
                    .combine_two_bytes(self.registers.b, self.registers.c);
                self.push_stack(bc_value);
                self.registers.inc_pc();
            }
            0x0C1 => {
                //POP BC
                let value = self.pop_stack();
                self.registers.set_bc(value);
                self.registers.inc_pc();
            }
            0x005 => {
                //DEC B
                let value = self.registers.b.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.b, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_b(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x00D => {
                //DEC C
                let value = self.registers.c.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.c, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_c(value);

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x01D => {
                //DEC E
                let value = self.registers.e.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.e, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_e(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x03D => {
                //DEC A
                let value = self.registers.a.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.a, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x015 => {
                //DEC D
                let value = self.registers.d.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.d, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_d(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x013 => {
                //INC DE
                let value = self
                    .registers
                    .combine_two_bytes(self.registers.d, self.registers.e);
                self.registers.set_de(value + 1);
                self.registers.inc_pc();
            }
            0x023 => {
                //INC HL
                let value = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l)
                    + 1;
                self.registers.set_hl(value);
                self.registers.inc_pc();
            }
            0x024 => {
                //INC H
                let value = self.registers.h + 1;
                if value == 0 {
                    flag_z = true;
                }
                flag_n = false;
                if self.registers.check_half_carry(self.registers.h, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_h(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0EA => {
                // LD (nn),A
                let following_two_bytes = self.following_two_bytes(pointer);
                self.write_memory(following_two_bytes, self.registers.a);
                self.registers.inc_pc();
            }
            0x090 => {
                // SUB B
                let value = self.registers.a - self.registers.b;

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.b)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.b {
                    flag_c = true;
                }
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            //New Opcodes after BootRom
            0x000 => {
                //NOP
                self.registers.inc_pc();
            }

            0x0CC => {
                //CALL Z, nn - 12
                let next_two_bytes = self.following_two_bytes(pointer);
                if self.registers.f.z {
                    let next_instruction_address = self.registers.pc + 1;
                    self.push_stack(next_instruction_address);
                    self.registers.set_pc(next_two_bytes);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x00B => {
                //DEB BC - 8
                let b_c = self
                    .registers
                    .combine_two_bytes(self.registers.b, self.registers.c);
                let value = b_c - 1;
                self.registers.set_bc(value);
                self.registers.inc_pc();
            }

            0x003 => {
                //INC BC - 8
                let b_c = self
                    .registers
                    .combine_two_bytes(self.registers.b, self.registers.c);
                let value = b_c + 1;
                self.registers.set_bc(value);
                self.registers.inc_pc();
            }

            0x073 => {
                //LD (HL),E
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.e);
                self.registers.inc_pc();
            }

            0x008 => {
                //LD (nn), SP - 20
                let address = self.following_two_bytes(pointer);
                self.set_two_bytes(address);
                self.registers.inc_pc();
            }
            0x01F => {
                //RRA
                let shifted_value = self.registers.a >> 1;
                let result = shifted_value
                    | match self.registers.f.c {
                        true => 0b10000000,
                        false => 0b00000000,
                    };

                if self.registers.a & 0b00000001 == 0b00000001 {
                    flag_c = true
                } else {
                    flag_c = false
                }
                self.registers.set_a(result);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x06E => {
                //LD L,(hl) - 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_l(value);
                self.registers.inc_pc();
            }

            0x0dd => match self.following_byte(pointer) {
                0x0dd => {
                    info!("dd again, pc: {:x}", self.registers.pc);
                }

                0x0D9 => {
                    //RETI

                    let address = self.pop_stack();
                    self.registers.set_pc(address);
                    self.registers.f.set_ime(true);
                }

                other => {
                    info!("Unknown instruction after 0x0DD: {:x}", other);
                    std::process::exit(1)
                }
            },

            0x0C3 => {
                // JP nn - 12
                let value = self.following_two_bytes(pointer);
                self.registers.set_pc(value)
            }

            0x036 => {
                //LD (HL),n -> 12
                let value = self.following_byte(self.registers.pc as usize);
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, value);
                self.registers.inc_pc();
            }

            0x02a => {
                // LDI A,(HL) -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_a(value);
                self.registers.set_hl(h_l + 1);
                self.registers.inc_pc();
            }

            0x002 => {
                //LD (BC), A -> 8
                let b_c = self
                    .registers
                    .combine_two_bytes(self.registers.b, self.registers.c);
                self.write_memory(b_c, self.registers.a);
                self.registers.inc_pc();
            }

            0x06d => {
                //LD L,L -> 4
                self.registers.set_l(self.registers.l);
                self.registers.inc_pc();
            }

            0x071 => {
                //LD (HL), C -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.c);
                self.registers.inc_pc();
            }

            0x03c => {
                //INC A -> 4
                let value = self.registers.a + 1;
                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(self.registers.a, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_a(value);
                self.registers.inc_pc();
            }

            0x0e1 => {
                //POP HL -> 12
                let value = self.pop_stack();
                self.registers.set_hl(value);
                self.registers.inc_pc();
            }

            0x03a => {
                //LD A, (HL-)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_a(value);
                self.registers.set_hl(h_l.wrapping_sub(1));
                self.registers.inc_pc();
            }

            //New Round 2
            0x040 => {
                //LD B,B
                self.registers.set_b(self.registers.b);
                self.registers.inc_pc();
            }
            0x041 => {
                // LD B,C -> 4
                self.registers.set_b(self.registers.c);
                self.registers.inc_pc();
            }
            0x042 => {
                // LD B,D -> 4
                self.registers.set_b(self.registers.d);
                self.registers.inc_pc();
            }
            0x043 => {
                //LD B,E
                self.registers.set_b(self.registers.e);
                self.registers.inc_pc();
            }
            0x044 => {
                // LD B,H -> 4
                self.registers.set_b(self.registers.h);
                self.registers.inc_pc();
            }
            0x045 => {
                //LD B, L -> 4
                self.registers.set_b(self.registers.l);
                self.registers.inc_pc();
            }
            0x046 => {
                //LD B,(HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_b(value);
                self.registers.inc_pc();
            }
            0x047 => {
                // LD B,A -> 4
                self.registers.set_b(self.registers.a);
                self.registers.inc_pc();
            }
            0x048 => {
                // LD C,B -> 4
                self.registers.set_c(self.registers.b);
                self.registers.inc_pc();
            }
            0x049 => {
                //LD C,C -> 4
                self.registers.set_c(self.registers.c);
                self.registers.inc_pc();
            }
            0x04A => {
                //LD C, D -> 4
                self.registers.set_c(self.registers.d);
                self.registers.inc_pc();
            }
            0x04B => {
                //LD C, E -> 4
                self.registers.set_c(self.registers.e);
                self.registers.inc_pc();
            }
            0x04C => {
                //LD C,H -> 4
                self.registers.set_c(self.registers.h);
                self.registers.inc_pc();
            }
            0x04d => {
                //LD C,L -> 4
                self.registers.set_c(self.registers.l);
                self.registers.inc_pc();
            }

            0x04E => {
                //LD C,(HL) -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_c(value);
                self.registers.inc_pc();
            }
            0x04F => {
                //LD C,A
                self.registers.set_c(self.registers.a);
                self.registers.inc_pc();
            }
            0x050 => {
                //LD D,B -> 4
                self.registers.set_d(self.registers.b);
                self.registers.inc_pc();
            }
            0x051 => {
                //LD D,C -> 4
                self.registers.set_d(self.registers.c);
                self.registers.inc_pc();
            }
            0x052 => {
                //LD D,D -> 4
                self.registers.set_d(self.registers.d);
                self.registers.inc_pc();
            }
            0x053 => {
                //LD D,E -> 4
                self.registers.set_d(self.registers.e);
                self.registers.inc_pc();
            }
            0x054 => {
                //LD D,H -> 4
                self.registers.set_d(self.registers.h);
                self.registers.inc_pc();
            }
            0x055 => {
                //LD D,L -> 4
                self.registers.set_d(self.registers.l);
                self.registers.inc_pc();
            }
            0x056 => {
                //LD D,(HL) -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_d(value);
                self.registers.inc_pc();
            }
            0x057 => {
                //LD D,A
                self.registers.set_d(self.registers.a);
                self.registers.inc_pc();
            }
            0x058 => {
                //LD E,B -> 4
                self.registers.set_e(self.registers.b);
                self.registers.inc_pc();
            }
            0x059 => {
                //LD E,C -> 4
                self.registers.set_e(self.registers.c);
                self.registers.inc_pc();
            }
            0x05a => {
                //LD E,D -> 4
                self.registers.set_e(self.registers.d);
                self.registers.inc_pc();
            }
            0x05b => {
                //LD E,E -> 4
                self.registers.set_e(self.registers.e);
                self.registers.inc_pc();
            }
            0x05c => {
                //LD E,H -> 4
                self.registers.set_e(self.registers.h);
                self.registers.inc_pc();
            }
            0x05d => {
                //LD E,L
                self.registers.set_e(self.registers.l);
                self.registers.inc_pc();
            }
            0x05e => {
                //LD E,(HL) -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_e(value);
                self.registers.inc_pc();
            }
            0x05f => {
                // LD E,A -> 4
                self.registers.set_e(self.registers.a);
                self.registers.inc_pc();
            }
            0x060 => {
                //LD H, B -> 4
                self.registers.set_h(self.registers.b);
                self.registers.inc_pc();
            }
            0x061 => {
                //LD H,C -> 4
                self.registers.set_h(self.registers.c);
                self.registers.inc_pc();
            }
            0x062 => {
                //LD H,D -> 4
                self.registers.set_h(self.registers.d);
                self.registers.inc_pc();
            }
            0x063 => {
                //LD H, E -> 4
                self.registers.set_h(self.registers.e);
                self.registers.inc_pc();
            }
            0x064 => {
                //LD H, E -> 4
                self.registers.set_h(self.registers.h);
                self.registers.inc_pc();
            }
            0x065 => {
                //LD H, L -> 4
                self.registers.set_h(self.registers.l);
                self.registers.inc_pc();
            }
            0x066 => {
                //LD H,(hl) - 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_h(value);
                self.registers.inc_pc();
            }
            0x067 => {
                //LD H,A
                self.registers.set_h(self.registers.a);
                self.registers.inc_pc();
            }
            0x068 => {
                //LD L, B -> 4
                self.registers.set_l(self.registers.b);
                self.registers.inc_pc();
            }
            0x069 => {
                // LD L,C -> 4
                self.registers.set_l(self.registers.c);
                self.registers.inc_pc();
            }
            0x06a => {
                // LD L,D -> 4
                self.registers.set_l(self.registers.d);
                self.registers.inc_pc();
            }
            0x06B => {
                //LD L,E -> 4
                self.registers.set_l(self.registers.e);
                self.registers.inc_pc();
            }
            0x06c => {
                //LD L,H -> 4
                self.registers.set_l(self.registers.h);
                self.registers.inc_pc();
            }

            0x038 => {
                //JR C,*one byte -> 8
                if self.registers.f.c {
                    let n_param = self.following_byte(pointer);
                    self.registers.inc_pc();
                    let destination = self
                        .registers
                        .add_signed_number(self.registers.pc, n_param as i8);
                    self.registers.set_pc(destination);
                } else {
                    self.registers.inc_pc();
                    self.registers.inc_pc();
                }
            }

            0x0c2 => {
                //JP NZ,nn -> 12
                let value = self.following_two_bytes(pointer);
                if !self.registers.f.z {
                    self.registers.set_pc(value);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x07f => {
                // LD A,A -> 4
                self.registers.set_a(self.registers.a);
                self.registers.inc_pc();
            }

            0x074 => {
                // LD (HL),H -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.h);
                self.registers.inc_pc();
            }

            0x075 => {
                // LD (HL),L -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.l);
                self.registers.inc_pc();
            }

            0x072 => {
                // LD (HL),D -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.d);
                self.registers.inc_pc();
            }

            0x079 => {
                //LD A,C -> 4
                self.registers.set_a(self.registers.c);
                self.registers.inc_pc();
            }

            0x0f1 => {
                //POP AF -> 12
                let value = self.pop_stack();
                self.registers.set_af(value);
                self.registers.inc_pc();
            }

            0x0b1 => {
                //OR C
                let value = self.registers.c | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x03f => {
                //CCF -> 4
                flag_z = self.registers.f.z;
                flag_c = !self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0b5 => {
                //OR L
                let value = self.registers.l | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x070 => {
                // LD (HL),B -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.write_memory(h_l, self.registers.b);
                self.registers.inc_pc();
            }

            0x0C4 => {
                let next_two_bytes = self.following_two_bytes(pointer);
                if !self.registers.f.z {
                    //CALL NZ, nn -> 24
                    let next_instruction_address = self.registers.pc + 1;
                    self.push_stack(next_instruction_address);
                    self.registers.set_pc(next_two_bytes);
                } else {
                    //CALL NZ, nn -> 12
                    self.registers.inc_pc();
                }
            }

            0x06f => {
                // LD L,A -> 4
                self.registers.set_l(self.registers.a);
                self.registers.inc_pc();
            }

            0x0D1 => {
                //POP DE -> 12
                let value = self.pop_stack();
                self.registers.set_de(value);
                self.registers.inc_pc();
            }

            0x092 => {
                // SUB D -> 4
                let value = self.registers.a.wrapping_sub(self.registers.d);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.d)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.d {
                    flag_c = true;
                }
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x097 => {
                // SUB A -> 4
                let value = self.registers.a.wrapping_sub(self.registers.a);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.a)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.a {
                    flag_c = true;
                }
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x091 => {
                // SUB C -> 4
                let value = self.registers.a.wrapping_sub(self.registers.c);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.c)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.c {
                    flag_c = true;
                }
                self.registers.set_a(value);

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x093 => {
                // SUB E -> 4
                let value = self.registers.a.wrapping_sub(self.registers.e);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.e)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.e {
                    flag_c = true;
                }

                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x094 => {
                // SUB H -> 4
                let value = self.registers.a.wrapping_sub(self.registers.h);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.h)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.h {
                    flag_c = true;
                }

                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x095 => {
                // SUB L -> 4
                let value = self.registers.a.wrapping_sub(self.registers.l);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.l)
                {
                    flag_h = true
                }
                if self.registers.a < self.registers.l {
                    flag_c = true;
                }
                self.registers.set_a(value);

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x096 => {
                // SUB (HL) -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let address_value = self.read_memory(h_l);
                let value = self.registers.a.wrapping_sub(address_value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, address_value)
                {
                    flag_h = true
                }
                if self.registers.a < address_value {
                    flag_c = true;
                }
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0e5 => {
                // PUSH HL -> 16
                let value = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.push_stack(value);
                self.registers.inc_pc();
            }

            0x0f5 => {
                // PUSH AF -> 16
                let value = self
                    .registers
                    .combine_two_bytes(self.registers.a, self.registers.get_f_as_byte());
                self.push_stack(value);
                self.registers.inc_pc();
            }

            0x0D5 => {
                // PUSH DE -> 16
                let value = self
                    .registers
                    .combine_two_bytes(self.registers.d, self.registers.e);
                self.push_stack(value);
                self.registers.inc_pc();
            }

            0x001 => {
                //LD BC, nn -> 12
                let value = self.following_two_bytes(self.registers.pc as usize);
                self.registers.set_bc(value);
                self.registers.inc_pc();
            }

            0x0fa => {
                //LD A, (nn) -> 16
                let address = self.following_two_bytes(self.registers.pc as usize);
                let value = self.read_memory(address);
                self.registers.set_a(value as u8);
                self.registers.inc_pc();
            }

            0x02C => {
                //INC L -> 4
                let value = self.registers.l.wrapping_add(1);
                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(self.registers.l, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_l(value);
                self.registers.inc_pc();
            }

            0x0D6 => {
                // SUB n -> 8
                let following_byte = self.following_byte(pointer);
                let value = self.registers.a - following_byte;

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, following_byte)
                {
                    flag_h = true
                }
                if self.registers.a < following_byte {
                    flag_c = true;
                }
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0b7 => {
                //OR A -> 4
                let value = self.registers.a | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x0b0 => {
                //OR B -> 4
                let value = self.registers.b | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x0b2 => {
                //OR D -> 4
                let value = self.registers.d | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x0b3 => {
                //OR E -> 4
                let value = self.registers.e | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x0b4 => {
                //OR H -> 4
                let value = self.registers.h | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x02D => {
                //DEC L -> 4
                let value = self.registers.l.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.l, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_l(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x025 => {
                //DEC H -> 4
                let value = self.registers.h.wrapping_sub(1);
                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.h, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.set_h(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x026 => {
                //LD H, *1byte -> 8
                let value = self.following_byte(pointer);
                self.registers.set_h(value);
                self.registers.inc_pc();
            }

            0x030 => {
                //JR NC,*one byte -> 8
                if !self.registers.f.c {
                    let n_param = self.following_byte(pointer); // as i8;
                    self.registers.inc_pc();
                    let destination = self
                        .registers
                        .add_signed_number(self.registers.pc, n_param as i8);
                    self.registers.set_pc(destination);
                } else {
                    self.registers.inc_pc();
                    self.registers.inc_pc();
                }
            }

            0x07A => {
                //LD A, D
                self.registers.set_a(self.registers.d);
                self.registers.inc_pc();
            }

            0x0D0 => {
                //RET NC -> 8
                if !self.registers.f.c {
                    let address = self.pop_stack();
                    self.registers.set_pc(address);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0C0 => {
                //RET NZ -> 8
                if !self.registers.f.z {
                    let address = self.pop_stack();
                    self.registers.set_pc(address);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0C8 => {
                //RET Z -> 8
                if self.registers.f.z {
                    let address = self.pop_stack();
                    self.registers.set_pc(address);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0D8 => {
                //RET C -> 8
                if self.registers.f.c {
                    let address = self.pop_stack();
                    self.registers.set_pc(address);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0B6 => {
                //OR (HL) -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let address_value = self.read_memory(h_l);
                let value = self.registers.a | address_value;

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_a(value);

                self.registers.inc_pc();
            }

            0x0F6 => {
                //OR n -> 8
                let following_value = self.following_byte(pointer);
                let value = following_value | self.registers.a;
                self.registers.set_a(value);

                if value == 0 {
                    flag_z = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);

                self.registers.inc_pc();
            }

            0x035 => {
                //DEC (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let address_value = self.read_memory(h_l);
                let value = address_value.wrapping_sub(1);
                self.write_memory(h_l, value);

                if value == 0 {
                    flag_z = true;
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(address_value, 1u8) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x009 => {
                //ADD HL, BC -> 8
                let b_c = self
                    .registers
                    .combine_two_bytes(self.registers.b, self.registers.c);
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = h_l + b_c;

                flag_z = self.registers.f.z;
                flag_n = false;

                if self.registers.check_half_carry_two_bytes(h_l, b_c) {
                    flag_h = true
                }
                if self.registers.check_carry_two_bytes(h_l, b_c) {
                    flag_c = true;
                }
                self.registers.set_hl(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x019 => {
                //ADD HL, DE -> 8
                let d_e = self
                    .registers
                    .combine_two_bytes(self.registers.d, self.registers.e);
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = h_l + d_e;

                flag_z = self.registers.f.z;
                flag_n = false;

                if self.registers.check_half_carry_two_bytes(h_l, d_e) {
                    flag_h = true
                }
                if self.registers.check_carry_two_bytes(h_l, d_e) {
                    flag_c = true;
                }
                self.registers.set_hl(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x029 => {
                //ADD HL, HL -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = h_l + h_l;

                flag_z = self.registers.f.z;
                flag_n = false;

                if self.registers.check_half_carry_two_bytes(h_l, h_l) {
                    flag_h = true
                }
                if self.registers.check_carry_two_bytes(h_l, h_l) {
                    flag_c = true;
                }
                self.registers.set_hl(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0E9 => {
                // JP (HL) -> 4
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.registers.set_pc(h_l)
            }

            0x0F8 => {
                //LDHL SP,n
                let following_byte = self.following_byte(pointer);
                let value = self
                    .registers
                    .add_signed_number(self.registers.sp, following_byte as i8);

                if self.registers.check_half_carry_u16_plus_i8(
                    self.registers.sp,
                    following_byte as i8,
                    value,
                ) {
                    flag_h = true;
                }

                if self.registers.check_carry_u16_plus_i8(
                    self.registers.sp,
                    following_byte as i8,
                    value,
                ) {
                    flag_c = true;
                }
                self.registers.set_hl(value as u16);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x012 => {
                //LD (DE), A
                let d_e = self
                    .registers
                    .combine_two_bytes(self.registers.d, self.registers.e);
                self.write_memory(d_e, self.registers.a);
                self.registers.inc_pc();
            }

            0x01C => {
                //INC E
                let value = self.registers.e + 1;
                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(self.registers.e, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_e(value);
                self.registers.inc_pc();
            }

            0x014 => {
                //INC D
                let value = self.registers.d + 1;
                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(self.registers.d, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_d(value);
                self.registers.inc_pc();
            }

            0x07E => {
                //LD A, (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.set_a(value);
                self.registers.inc_pc();
            }

            0x0f9 => {
                //LD SP, HL
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                self.registers.set_sp(h_l);
                self.registers.inc_pc();
            }

            0x033 => {
                //INC SP -> 8
                let value = self.registers.sp.wrapping_add(1);
                self.registers.set_sp(value);
                self.registers.inc_pc();
            }

            0x03B => {
                //DEC SP -> 8
                let value = self.registers.sp.wrapping_sub(1);
                self.registers.set_sp(value);
                self.registers.inc_pc();
            }

            0x039 => {
                //ADD HL, SP -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = h_l.wrapping_add(self.registers.sp);

                flag_z = self.registers.f.z;
                flag_n = false;

                if self
                    .registers
                    .check_half_carry_two_bytes(h_l, self.registers.sp)
                {
                    flag_h = true
                }
                if self.registers.check_carry_two_bytes(h_l, self.registers.sp) {
                    flag_c = true;
                }
                self.registers.set_hl(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0E8 => {
                //ADD SP, n -> 16
                let following_value = self.following_byte(pointer);
                let value = self
                    .registers
                    .add_signed_number(self.registers.sp, following_value as i8);

                if self.registers.check_half_carry_u16_plus_i8(
                    self.registers.sp,
                    following_value as i8,
                    value,
                ) {
                    flag_h = true
                }
                if self.registers.check_carry_u16_plus_i8(
                    self.registers.sp,
                    following_value as i8,
                    value,
                ) {
                    flag_c = true;
                }
                self.registers.set_sp(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x01B => {
                //DEC DE -> 8
                let d_e = self
                    .registers
                    .combine_two_bytes(self.registers.d, self.registers.e);
                let value = d_e.wrapping_sub(1);
                self.registers.set_de(value);
                self.registers.inc_pc();
            }

            0x02B => {
                //DEC HL -> 8
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = h_l.wrapping_sub(1);
                self.registers.set_hl(value);
                self.registers.inc_pc();
            }

            0x0ca => {
                //JP Z,nn -> 12
                let value = self.following_two_bytes(pointer);
                if self.registers.f.z {
                    self.registers.set_pc(value);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0D2 => {
                //JP NC,nn -> 12
                let value = self.following_two_bytes(pointer);
                if !self.registers.f.c {
                    self.registers.set_pc(value);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0Da => {
                //JP C,nn -> 12
                let value = self.following_two_bytes(pointer);
                if self.registers.f.c {
                    self.registers.set_pc(value);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0D4 => {
                //CALL NC, nn - 12
                let next_two_bytes = self.following_two_bytes(pointer);
                if !self.registers.f.c {
                    let next_instruction_address = self.registers.pc + 1;
                    self.push_stack(next_instruction_address);
                    self.registers.set_pc(next_two_bytes);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0DC => {
                //CALL C, nn - 12
                let next_two_bytes = self.following_two_bytes(pointer);
                if self.registers.f.c {
                    let next_instruction_address = self.registers.pc + 1;
                    self.push_stack(next_instruction_address);
                    self.registers.set_pc(next_two_bytes);
                } else {
                    self.registers.inc_pc();
                }
            }

            0x0D9 => {
                //RETI -> 8
                let address = self.pop_stack();
                self.registers.set_pc(address);
                self.registers.f.set_ime(true);
            }

            0x0f2 => {
                //LD A, ($ff00+C) -> 8
                let offset = 0xff00 + self.registers.c as u16;
                let value = self.read_memory(offset);
                self.registers.set_a(value);
                self.registers.inc_pc();
            }

            0x02f => {
                //CPL: Flip all bits of A -> 4
                let value = !self.registers.a;
                flag_z = self.registers.f.z;
                flag_n = true;
                flag_h = true;
                flag_c = self.registers.f.c;
                self.registers.set_a(value);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x00A => {
                //LD A, (BC)
                let b_c = self
                    .registers
                    .combine_two_bytes(self.registers.b, self.registers.c);
                let value = self.read_memory(b_c);
                self.registers.set_a(value as u8);
                self.registers.inc_pc();
            }

            0x034 => {
                //INC (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let address_value = self.read_memory(h_l);

                let value = address_value.wrapping_add(1);
                if value == 0 {
                    flag_z = true;
                };
                if self.registers.check_half_carry(address_value, 1) {
                    flag_h = true;
                }
                flag_c = self.registers.f.c;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.write_memory(h_l, value);
                self.registers.inc_pc();
            }

            0x027 => {
                //DAA -> 4
                let was_subtract = self.registers.f.n;

                let mut bcd_adjust = 0;

                if self.registers.f.h || (!self.registers.f.n && (self.registers.a & 0xf) > 9) {
                    bcd_adjust |= 0x6;
                }

                if self.registers.f.c || (!self.registers.f.n && self.registers.a > 0x99) {
                    bcd_adjust |= 0x60;
                    flag_c = true
                }

                let result = if was_subtract {
                    self.registers.a.wrapping_sub(bcd_adjust)
                } else {
                    self.registers.a.wrapping_add(bcd_adjust)
                };

                if result == 0 {
                    flag_z = true;
                };

                flag_n = self.registers.f.n;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.set_a(result);
                self.registers.inc_pc();
            }

            0x037 => {
                //SCF -> 4
                flag_z = self.registers.f.z;
                flag_n = false;
                flag_h = false;
                flag_c = true;
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0B8 => {
                //CP B
                if self.registers.a == self.registers.b {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.b)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.b {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0B9 => {
                //CP C
                if self.registers.a == self.registers.c {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.c)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.c {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0BA => {
                //CP D
                if self.registers.a == self.registers.d {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.d)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.d {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0BB => {
                //CP E
                if self.registers.a == self.registers.e {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.e)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.e {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0BC => {
                //CP H
                if self.registers.a == self.registers.h {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.h)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.h {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0BD => {
                //CP L
                if self.registers.a == self.registers.l {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.l)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.l {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x0BE => {
                //CP (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);

                if self.registers.a == value {
                    flag_z = true
                }
                flag_n = true;
                if self.registers.check_half_carry_sub(self.registers.a, value) {
                    flag_h = true
                }
                if self.registers.a < value {
                    flag_c = true;
                }
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0BF => {
                //CP A
                if self.registers.a == self.registers.a {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, self.registers.a)
                {
                    flag_h = true
                }

                if self.registers.a < self.registers.a {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }
            0x0FE => {
                //CP #
                let following_byte = self.following_byte(pointer);
                if self.registers.a == following_byte {
                    flag_z = true
                }
                flag_n = true;

                if self
                    .registers
                    .check_half_carry_sub(self.registers.a, following_byte)
                {
                    flag_h = true
                }

                if self.registers.a < following_byte {
                    flag_c = true;
                }

                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x087 => {
                //ADD A, A
                self.registers.add_a_n(self.registers.a);
                self.registers.inc_pc();
            }
            0x080 => {
                //ADD A, B
                self.registers.add_a_n(self.registers.b);
                self.registers.inc_pc();
            }
            0x081 => {
                //ADD A, C
                self.registers.add_a_n(self.registers.c);
                self.registers.inc_pc();
            }
            0x082 => {
                //ADD A, D
                self.registers.add_a_n(self.registers.d);
                self.registers.inc_pc();
            }
            0x083 => {
                //ADD A, E
                self.registers.add_a_n(self.registers.e);
                self.registers.inc_pc();
            }
            0x084 => {
                //ADD A, H
                self.registers.add_a_n(self.registers.h);
                self.registers.inc_pc();
            }
            0x085 => {
                //ADD A, L
                self.registers.add_a_n(self.registers.l);
                self.registers.inc_pc();
            }
            0x086 => {
                //ADD A, (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.add_a_n(value);
                self.registers.inc_pc();
            }
            0x0C6 => {
                //ADD A, #
                let following_byte = self.following_byte(pointer);
                self.registers.add_a_n(following_byte);
                self.registers.inc_pc();
            }

            0x08f => {
                //ADC A, A
                self.registers.adc_a_n(self.registers.a);
                self.registers.inc_pc();
            }
            0x088 => {
                //ADC A, B
                self.registers.adc_a_n(self.registers.b);
                self.registers.inc_pc();
            }
            0x089 => {
                //ADC A, C
                self.registers.adc_a_n(self.registers.c);
                self.registers.inc_pc();
            }
            0x08A => {
                //ADC A, D
                self.registers.adc_a_n(self.registers.d);
                self.registers.inc_pc();
            }
            0x08B => {
                //ADC A, E
                self.registers.adc_a_n(self.registers.e);
                self.registers.inc_pc();
            }
            0x08C => {
                //ADC A, H
                self.registers.adc_a_n(self.registers.h);
                self.registers.inc_pc();
            }
            0x08D => {
                //ADC A, L
                self.registers.adc_a_n(self.registers.l);
                self.registers.inc_pc();
            }
            0x08E => {
                //ADC A, (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.adc_a_n(value);
                self.registers.inc_pc();
            }
            0x0CE => {
                //ADC A, #
                let following_byte = self.following_byte(pointer);
                self.registers.adc_a_n(following_byte);
                self.registers.inc_pc();
            }

            0x09f => {
                //SBC A, A
                self.registers.sbc_a_n(self.registers.a);
                self.registers.inc_pc();
            }
            0x098 => {
                //SBC A, B
                self.registers.sbc_a_n(self.registers.b);
                self.registers.inc_pc();
            }
            0x099 => {
                //SBC A, C
                self.registers.sbc_a_n(self.registers.c);
                self.registers.inc_pc();
            }
            0x09A => {
                //SBC A, D
                self.registers.sbc_a_n(self.registers.d);
                self.registers.inc_pc();
            }
            0x09B => {
                //SBC A, E
                self.registers.sbc_a_n(self.registers.e);
                self.registers.inc_pc();
            }
            0x09C => {
                //SBC A, H
                self.registers.sbc_a_n(self.registers.h);
                self.registers.inc_pc();
            }
            0x09D => {
                //SBC A, L
                self.registers.sbc_a_n(self.registers.l);
                self.registers.inc_pc();
            }
            0x09E => {
                //SBC A, (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.sbc_a_n(value);
                self.registers.inc_pc();
            }
            0x0de => {
                //SBC A, #
                let following_byte = self.following_byte(pointer);
                self.registers.sbc_a_n(following_byte);
                self.registers.inc_pc();
            }

            0x0A7 => {
                //AND A
                self.registers.and_a_n(self.registers.a);
                self.registers.inc_pc();
            }

            0x0A0 => {
                //AND B
                self.registers.and_a_n(self.registers.b);
                self.registers.inc_pc();
            }
            0x0A1 => {
                //AND C
                self.registers.and_a_n(self.registers.c);
                self.registers.inc_pc();
            }
            0x0A2 => {
                //AND D
                self.registers.and_a_n(self.registers.d);
                self.registers.inc_pc();
            }
            0x0A3 => {
                //AND E
                self.registers.and_a_n(self.registers.e);
                self.registers.inc_pc();
            }
            0x0A4 => {
                //AND H
                self.registers.and_a_n(self.registers.h);
                self.registers.inc_pc();
            }
            0x0A5 => {
                //AND L
                self.registers.and_a_n(self.registers.l);
                self.registers.inc_pc();
            }
            0x0A6 => {
                //AND (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.and_a_n(value);
                self.registers.inc_pc();
            }
            0x0E6 => {
                //AND #
                let following_byte = self.following_byte(pointer);
                self.registers.and_a_n(following_byte);
                self.registers.inc_pc();
            }

            0x0AF => {
                // XOR A
                self.registers.xor_a_n(self.registers.a);
                self.registers.inc_pc();
            }

            0x0A8 => {
                // XOR B
                self.registers.xor_a_n(self.registers.b);
                self.registers.inc_pc();
            }

            0x0A9 => {
                // XOR C
                self.registers.xor_a_n(self.registers.c);
                self.registers.inc_pc();
            }

            0x0AA => {
                // XOR D
                self.registers.xor_a_n(self.registers.d);
                self.registers.inc_pc();
            }

            0x0AB => {
                // XOR E
                self.registers.xor_a_n(self.registers.e);
                self.registers.inc_pc();
            }

            0x0AC => {
                // XOR H
                self.registers.xor_a_n(self.registers.h);
                self.registers.inc_pc();
            }

            0x0AD => {
                // XOR L
                self.registers.xor_a_n(self.registers.l);
                self.registers.inc_pc();
            }

            0x0AE => {
                // XOR (HL)
                let h_l = self
                    .registers
                    .combine_two_bytes(self.registers.h, self.registers.l);
                let value = self.read_memory(h_l);
                self.registers.xor_a_n(value);
                self.registers.inc_pc();
            }

            0x0EE => {
                // XOR n
                let following_byte = self.following_byte(pointer);
                self.registers.xor_a_n(following_byte);
                self.registers.inc_pc();
            }

            0x00F => {
                //RRCA
                let cf = self.registers.a << 7;
                let result = self.registers.a >> 1 | cf;

                if cf & 0b10000000 == 0b10000000 {
                    flag_c = true
                } else {
                    flag_c = false
                }
                self.registers.set_a(result);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x007 => {
                // RLCA:
                let cf = self.registers.a >> 7;
                let result = (self.registers.a << 1) | cf;
                flag_c = cf == 1;
                self.registers.set_a(result);
                self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
                self.registers.inc_pc();
            }

            0x0c7 => {
                //RST $00
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x00);
            }
            0x0cf => {
                //RST $08
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x08);
            }
            0x0d7 => {
                //RST $10
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x10);
            }
            0x0df => {
                //RST $18
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x18);
            }
            0x0e7 => {
                //RST $20
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x20);
            }
            0x0ef => {
                //RST $28
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x28);
            }
            0x0f7 => {
                //RST $30
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x30);
            }
            0x0ff => {
                //RST $38
                self.push_stack(self.registers.pc + 1);
                self.registers.set_pc(0x38);
            }

            0x0f3 => {
                // DI
                //Interrupts are disabled after instruction after DI is executed.
                self.registers.f.set_ime(false);
                self.registers.inc_pc();
            }

            0x0fb => {
                // EI
                self.registers.inc_pc();
                self.registers.f.set_ime(true);
            }

            0x076 => {
                //HALT Power down CPU until interrupt occurs -> 4
                //Implementation escalated to Gameboy. Checking at fn execute_opcodes()
                info!("NEED TO IMPLEMENT HALT FUNCTION FOR 0x076");
                self.registers.inc_pc();
            }

            0x010 => {
                info!("Need to implement STOP");
                self.registers.inc_pc();
                // std::process::exit(1);
                // match self.following_byte(pointer) {
                //     0x00 => self.registers.inc_pc(),
                //     _other => self.registers.inc_pc(),
                // }
            }

            0x0d3 => {
                //No operation
                info!("no operation with opcode 0xd3");
                self.registers.inc_pc();
            }
            0x0fd => {
                //No operation
                info!("no operation with opcode 0xfd");
                self.registers.inc_pc();
            }
            0x0f4 => {
                //No operation
                info!("no operation with opcode 0xf4");
                self.registers.inc_pc();
            }

            other => {
                info!("No opcode found for {:x} at {:x}", other, pointer);
                std::process::exit(1)
            }
        }
    }

    fn push_stack(&mut self, value: u16) {
        let value_byte_vec = value.to_be_bytes();
        self.registers.set_sp(self.registers.sp - 1);
        self.write_memory(self.registers.sp, value_byte_vec[0]);
        self.registers.set_sp(self.registers.sp - 1);
        self.write_memory(self.registers.sp, value_byte_vec[1]);
    }
    fn pop_stack(&mut self) -> u16 {
        let second_byte = self.read_memory(self.registers.sp);
        self.write_memory(self.registers.sp, 0);
        self.registers.set_sp(self.registers.sp + 1);

        let firt_byte = self.read_memory(self.registers.sp);
        self.write_memory(self.registers.sp, 0);
        self.registers.set_sp(self.registers.sp + 1);

        let result = self.registers.combine_two_bytes(firt_byte, second_byte);
        result
    }

    fn following_byte(&mut self, address: usize) -> u8 {
        let byte = self.read_memory(address as u16 + 1);
        self.set_pc(&self.registers.pc + 1);
        byte
    }

    fn following_two_bytes(&mut self, address: usize) -> u16 {
        let byte_one = self.read_memory(address as u16 + 1);
        let byte_two = self.read_memory(address as u16 + 2);
        let two_bytes_value = self.registers.combine_two_bytes(byte_two, byte_one);
        self.set_pc(&self.registers.pc + 2);
        two_bytes_value
    }
    fn set_two_bytes(&mut self, start_address: u16) {
        self.write_memory(start_address, self.registers.sp as u8);
        self.write_memory(
            start_address.wrapping_add(1),
            (self.registers.sp >> 8) as u8,
        );
    }

    fn res_b_r(
        &mut self,
        bit_idx: u8,
        register_value: u8,
        register_name: &str,
        hl_value: Option<u16>,
    ) {
        let check_bits = match bit_idx {
            0 => 0b11111110,
            1 => 0b11111101,
            2 => 0b11111011,
            3 => 0b11110111,
            4 => 0b11101111,
            5 => 0b11011111,
            6 => 0b10111111,
            7 => 0b01111111,
            _ => {
                println!("Invalid bit index");
                std::process::exit(1)
            }
        };

        let value = register_value & check_bits;
        match register_name {
            "a" => self.registers.set_a(value),
            "b" => self.registers.set_b(value),
            "c" => self.registers.set_c(value),
            "d" => self.registers.set_d(value),
            "e" => self.registers.set_e(value),
            "h" => self.registers.set_h(value),
            "l" => self.registers.set_l(value),
            "h_l" => self.write_memory(hl_value.unwrap(), value),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }
    }

    fn set_b_r(
        &mut self,
        bit_idx: u8,
        register_value: u8,
        register_name: &str,
        hl_value: Option<u16>,
    ) {
        let check_bits = match bit_idx {
            0 => 0b00000001,
            1 => 0b00000010,
            2 => 0b00000100,
            3 => 0b00001000,
            4 => 0b00010000,
            5 => 0b00100000,
            6 => 0b01000000,
            7 => 0b10000000,
            _ => {
                println!("Invalid bit index");
                std::process::exit(1)
            }
        };

        let value = register_value | check_bits;
        match register_name {
            "a" => self.registers.set_a(value),
            "b" => self.registers.set_b(value),
            "c" => self.registers.set_c(value),
            "d" => self.registers.set_d(value),
            "e" => self.registers.set_e(value),
            "h" => self.registers.set_h(value),
            "l" => self.registers.set_l(value),
            "h_l" => self.write_memory(hl_value.unwrap(), value),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }
    }

    fn srl(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let mut flag_c = false;

        let result = register_value >> 1;
        if result == 0 {
            flag_z = true
        }

        if register_value & 0b00000001 == 0b00000001 {
            flag_c = true
        }

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn sra(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let mut flag_c = false;

        let result = register_value >> 1 | (register_value & 0x80);
        if result == 0 {
            flag_z = true
        }

        if register_value & 0b00000001 == 0b00000001 {
            flag_c = true
        }
        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn swap(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let flag_c = false;

        let result = register_value << 4 | register_value >> 4;

        if result == 0 {
            flag_z = true;
        }

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn sla(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let mut flag_c = false;

        let result = register_value << 1;

        if result == 0 {
            flag_z = true
        }

        if register_value & 0b10000000 == 0b10000000 {
            flag_c = true
        }

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn rr(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let mut flag_c = false;

        let shifted_value = register_value >> 1;
        let result = shifted_value
            | match self.registers.f.c {
                true => 0b10000000,
                false => 0b00000000,
            };

        if result == 0 {
            flag_z = true
        }

        if register_value & 0b00000001 == 1 {
            flag_c = true
        }

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn rl(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let mut flag_c = false;

        let shifted_value = register_value << 1;
        let result = shifted_value
            | match self.registers.f.c {
                true => 0b00000001,
                false => 0b00000000,
            };

        if result == 0 {
            flag_z = true
        }

        if register_value & 0b10000000 == 0b10000000 {
            flag_c = true
        }

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn rrc(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;
        let mut flag_c = false;

        let cf = register_value << 7;
        let result = register_value >> 1 | cf;

        if result == 0 {
            flag_z = true
        }

        if cf & 0b10000000 == 0b10000000 {
            flag_c = true
        }

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    fn rlc(&mut self, register_value: u8, register_name: &str, hl_value: Option<u16>) {
        let mut flag_z = false;
        let flag_n = false;
        let flag_h = false;

        let cf = register_value >> 7;
        let result = (register_value << 1) | cf;

        if result == 0 {
            flag_z = true
        }

        let flag_c = cf == 1;

        match register_name {
            "a" => self.registers.set_a(result),
            "b" => self.registers.set_b(result),
            "c" => self.registers.set_c(result),
            "d" => self.registers.set_d(result),
            "e" => self.registers.set_e(result),
            "h" => self.registers.set_h(result),
            "l" => self.registers.set_l(result),
            "h_l" => self.write_memory(hl_value.unwrap(), result),
            _ => {
                println!("Invalid register name");
                std::process::exit(1)
            }
        }

        self.registers.f.set_flag(flag_z, flag_n, flag_h, flag_c);
    }

    pub fn to_serializable(&self) -> SerializedGameboy {
        let serializable = SerializedGameboy {
            registers: self.registers.clone(),
            total_cycle_num: self.total_cycle_num,
            vram_cycle_num: self.vram_cycle_num,
            timer_cycle_num: self.timer_cycle_num,
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

    pub fn ime(&self) -> bool {
        self.registers.f.ime
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
    }

    pub fn remove_break_point(&mut self, point: u16) {
        self.break_points.retain(|&x| x != point);
    }

    pub fn get_break_points(&self) -> Vec<u16> {
        self.break_points.clone()
    }

    //MBC

    pub fn get_mbc(&self) -> u8 {
        self.mbc
    }

    pub fn get_rom_bank(&self) -> u8 {
        self.rom_bank
    }

    pub fn get_ram_bank(&self) -> u8 {
        self.ram_bank
    }

    // pub fn get_ram_bank_memory(&self) -> Vec<u8> {
    //     self.ram_bank_memory
    // }

    pub fn get_is_ram_enabled(&self) -> bool {
        self.is_ram_enabled
    }

    pub fn get_is_rom_banking_enabled(&self) -> bool {
        self.is_rom_banking_enabled
    }

    fn get_mbc_from_memory(memory: &Vec<u8>) -> u8 {
        let mbc = match memory[0x0147] {
            0 => 0,
            1 => 1,
            2 => 1,
            3 => 1,
            5 => 2,
            6 => 2,
            0x13 => 1,
            _ => {
                info!("Invalid mbc value: {:x} at $0x147", memory[0x0147]);
                panic!("");
            }
        };
        mbc
    }

    fn enable_ram_bank(&mut self, address: u16, data: u8) {
        if self.mbc == 2 {
            if address & 0b00010000 == 0b00010000 {
                return;
            }
        }

        let lower_nibble = data & 0xF;
        if lower_nibble == 0xA {
            self.is_ram_enabled = true;
        } else if lower_nibble == 0x0 {
            self.is_ram_enabled = false;
        }
    }

    fn change_lo_rom_bank(&mut self, data: u8) {
        if self.mbc == 2 {
            self.rom_bank = data & 0xF;
            if self.rom_bank == 0 {
                self.rom_bank += 1;
            }
            return;
        }

        let lower_5_bits = data & 31;
        self.rom_bank &= 224; // turn off the lower 5
        self.rom_bank |= lower_5_bits;
        if self.rom_bank == 0 {
            self.rom_bank += 1;
        }
    }

    fn change_hi_rom_bank(&mut self, data: u8) {
        // turn off the upper 3 bits of the current rom
        self.rom_bank &= 31;

        // turn off the lower 5 bits of the data
        let new_data = data & 224;
        self.rom_bank = self.rom_bank | new_data;
        if self.rom_bank == 0 {
            self.rom_bank += 1;
        }
    }

    fn change_ram_bank(&mut self, data: u8) {
        self.ram_bank = data & 0x3;
    }

    fn change_rom_ram_mode(&mut self, data: u8) {
        let new_data = data & 0x1;
        self.is_rom_banking_enabled = if new_data == 0 { true } else { false };
        if self.is_rom_banking_enabled {
            self.ram_bank = 0
        }
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        let is_mbc_one_or_two = self.mbc == 1 || self.mbc == 2;
        // enable ram
        if address < 0x2000 {
            if is_mbc_one_or_two {
                // DoRamBankEnable(address,data) ;
                self.enable_ram_bank(address, value);
            }
        }
        // change ROM bank
        else if (address >= 0x200) && (address < 0x4000) {
            if is_mbc_one_or_two {
                // DoChangeLoROMBank(data) ;
                self.change_lo_rom_bank(value);
            }
        }
        // change ROM or RAM bank
        else if (address >= 0x4000) && (address < 0x6000) {
            // there is no rambank in mbc2 so always use rambank 0
            if self.mbc == 1 {
                if self.is_rom_banking_enabled {
                    // DoChangeHiRomBank(data) ;
                    self.change_hi_rom_bank(value);
                } else {
                    // DoRAMBankChange(data) ;
                    self.change_ram_bank(value)
                }
            }
        }
        // this will change whether we are doing ROM banking
        // or RAM banking with the above if statement
        else if (address >= 0x6000) && (address < 0x8000) {
            if self.mbc == 1 {
                // DoChangeROMRAMMode(data) ;
                self.change_rom_ram_mode(value);
            }
        } else if (address >= 0xA000) && (address < 0xC000) {
            if self.is_ram_enabled {
                let new_address = address - 0xA000;
                self.ram_bank_memory[(new_address + (self.ram_bank as u16 * 0x2000)) as usize] =
                    value;
            }
        } else if (address >= 0xFEA0) && (address < 0xFEFF) {
            //Nothing happens
        } else if address == 0xFF44 {
            self.memory[address as usize] = 0;
        } else {
            self.memory[address as usize] = value;
        }
    }

    fn read_memory(&self, address: u16) -> u8 {
        // Read from the rom memory bank
        if (address >= 0x4000) && (address <= 0x7FFF) {
            let new_address = address - 0x4000;
            return self.cartridge[(new_address + (self.rom_bank as u16 * 0x4000)) as usize];
        }
        // Reading from ram memory bank
        else if (address >= 0xA000) && (address <= 0xBFFF) {
            let new_address = address - 0xA000;
            return self.ram_bank_memory[(new_address + (self.ram_bank as u16 * 0x2000)) as usize];
        } else if 0xFF00 == address {
            return self.get_joypad_state();
        }

        // else return memory
        return self.memory[address as usize];
    }

    fn get_joypad_state(&self) -> u8 {
        let p1 = self.memory[0xFF00];

        let result = p1 ^ 0xFF;

        let is_standard_button = result & 0b00010000 == 0;
        let is_directional_button = result & 0b00100000 == 0;

        if is_standard_button {
            let top_joypad = self.joypad_state >> 4;
            return result & (top_joypad | 0xF0);
        } else if is_directional_button {
            let bottom_joypad = self.joypad_state & 0xF;
            return result & (bottom_joypad | 0xF0);
        }

        result
    }

    pub fn joypad_key_pressed(&mut self, key: u8) {
        info!("key pressed: {:x}", key);
        // if setting from 1 to 0 we may have to request an interupt
        let previously_unset = self.joypad_state & key == 0;

        // remember if a keypressed its bit is 0 not 1
        let new_joypad_state = self.joypad_state & (!key);
        self.joypad_state = new_joypad_state;

        let is_standard_button = key > 0b0000100; //or directional button

        let p1 = self.memory[0xFF00];

        let starndard_btn_listening = is_standard_button && (p1 & 0b00100000 == 0b00100000);
        let directional_btn_listening = !is_standard_button && (p1 & 0b00010000 == 0b00010000);

        if (starndard_btn_listening || directional_btn_listening) && !previously_unset {
            self.request_joypad_interrupt();
        } else {
            info!("no request");
        }

        info!(
            "standard:{:?}, directioanal:{:?}, prevous:{:?}, p1:{:b}",
            starndard_btn_listening, directional_btn_listening, previously_unset, p1
        );
    }

    pub fn joypad_key_released(&mut self, key: u8) {
        info!("key released: {:x}", key);
        let new_joypad_state = self.joypad_state | (key);
        self.joypad_state = new_joypad_state;
    }

    //Timer
    fn update_timer(&mut self, instruction: u8) {
        if self.is_timer_enabled() {
            self.add_cycles(instruction, CycleRegister::TimerCycle);
            let clock_count = self.timer_cycle_to_cpu_clock();

            if self.timer_cycle_num >= clock_count {
                self.add_time_counter();
                self.set_timer_cycle(self.timer_cycle_num - clock_count);
            };
        } else {
            self.set_timer_cycle(0);
        }
    }

    fn execute_interuption(&mut self) {
        let do_v_blank = (self.memory[0xff0f] & 0b00000001 == 0b00000001)
            && (self.memory[0xffff] & 0b00000001 == 0b00000001);

        let do_lcd = (self.memory[0xff0f] & 0b00000010 == 0b00000010)
            && (self.memory[0xffff] & 0b00000010 == 0b00000010);

        let do_timer = (self.memory[0xff0f] & 0b00000100 == 0b00000100)
            && (self.memory[0xffff] & 0b00000100 == 0b00000100);

        let do_serial = (self.memory[0xff0f] & 0b00001000 == 0b00001000)
            && (self.memory[0xffff] & 0b00001000 == 0b00001000);

        let do_joypad = (self.memory[0xff0f] & 0b00010000 == 0b00010000)
            && (self.memory[0xffff] & 0b00010000 == 0b00010000);

        let any_interrupt = do_v_blank || do_lcd || do_timer || do_serial || do_joypad;

        if self.registers.f.ime {
            if do_v_blank {
                info!("execute vblank interrupt");
                self.is_halt = false;
                self.registers.f.set_ime(false);

                self.memory[0xff0f] = self.memory[0xff0f] ^ 0b00000001;
                self.push_stack(self.registers.pc);
                self.registers.set_pc(0x40);
            }

            if do_lcd {
                info!("execute lcd interrupt");
                self.is_halt = false;
                self.registers.f.set_ime(false);

                self.memory[0xff0f] = self.memory[0xff0f] ^ 0b00000010;
                self.push_stack(self.registers.pc);
                self.registers.set_pc(0x48);
            }

            if (self.memory[0xff0f] & 0b00000010 == 0b00000010) {
                info!("has lcd request")
            }
            if do_timer {
                self.is_halt = false;
                self.registers.f.set_ime(false);

                self.memory[0xff0f] = self.memory[0xff0f] ^ 0b00000100;
                self.push_stack(self.registers.pc);
                self.registers.set_pc(0x50);
            }

            if do_serial {
                self.is_halt = false;
                self.registers.f.set_ime(false);

                self.memory[0xff0f] = self.memory[0xff0f] ^ 0b00001000;
                self.push_stack(self.registers.pc);
                self.registers.set_pc(0x58);
            }

            if do_joypad {
                self.is_halt = false;
                self.registers.f.set_ime(false);

                self.memory[0xff0f] = self.memory[0xff0f] ^ 0b00010000;
                self.push_stack(self.registers.pc);
                self.registers.set_pc(0x60);
            }
        }

        if any_interrupt {
            //info!("an_interrupt, halt is false");
            self.is_halt = false
        }
    }

    pub fn set_vram_cycle(&mut self, value: u16) {
        self.vram_cycle_num = value
    }

    pub fn set_timer_cycle(&mut self, value: usize) {
        self.timer_cycle_num = value
    }

    pub fn request_vblank(&mut self) {
        self.should_draw = true;
        self.memory[0xff0f] = self.memory[0xff0f] | 0b000000001;
    }

    pub fn request_lcd_interrupt(&mut self) {
        self.should_draw = true;
        self.memory[0xff0f] = self.memory[0xff0f] | 0b000000010;
    }

    pub fn request_timer_interrupt(&mut self) {
        self.memory[0xff0f] = self.memory[0xff0f] | 0b000000100;
    }

    pub fn request_joypad_interrupt(&mut self) {
        info!("requested");
        self.memory[0xff0f] = self.memory[0xff0f] | 0b00010000;
    }

    pub fn inc_ly(&mut self) {
        let ly_max = 153;
        let vblank_start = 144;

        if self.memory[0xff44] == ly_max {
            self.memory[0xff44] = 0;
        } else {
            self.memory[0xff44] = self.memory[0xff44] + 1;
            if self.memory[0xff44] == vblank_start {
                self.request_vblank()
            }
        }
    }

    pub fn total_cycle(&self) -> usize {
        self.total_cycle_num
    }

    pub fn vram_cycle(&self) -> u16 {
        self.vram_cycle_num
    }

    pub fn timer_cycle(&self) -> usize {
        self.timer_cycle_num
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

    fn add_time_counter(&mut self) {
        if self.memory[0xff05] == 255 {
            self.request_timer_interrupt();
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
            0x0CB => match self.read_memory(self.registers.pc + 1) {
                0x006 => 16,
                0x01E => 16,
                0x02E => 16,
                0x03E => 16,
                0x04E => 16,
                0x05E => 16,
                0x06E => 16,
                0x07E => 16,
                0x08E => 16,
                0x09E => 16,
                0x0aE => 16,
                0x0bE => 16,
                0x0cE => 16,
                0x0dE => 16,
                0x0eE => 16,
                0x0fE => 16,
                0x016 => 16,
                0x026 => 16,
                0x036 => 16,
                0x046 => 16,
                0x056 => 16,
                0x066 => 16,
                0x076 => 16,
                0x086 => 16,
                0x096 => 16,
                0x0a6 => 16,
                0x0b6 => 16,
                0x0c6 => 16,
                0x0d6 => 16,
                0x0e6 => 16,
                0x0f6 => 16,
                _other => 8,
            },
            0x017 => 4,
            0x007 => 4,
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
            0x0CC => {
                if self.registers.f.z {
                    24
                } else {
                    12
                }
            }
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
                info!("Not sure what's the cycle for 0x0DD");
                std::process::exit(1)
            }
            0x0C3 => 12,
            0x0f3 => 4,
            0x036 => 12,
            0x02a => 8,
            0x047 => 4,
            0x002 => 8,
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
            0x0C4 => {
                if !self.registers.f.z {
                    24
                } else {
                    12
                }
            }
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
            0x0e5 => 16,
            0x0f5 => 16,
            0x0D5 => 16,
            0x001 => 12,
            0x0fa => 16,
            0x02C => 4,
            0x0A8 => 4,
            0x0A9 => 4,
            0x0AA => 4,
            0x0AB => 4,
            0x0AC => 4,
            0x0AD => 4,
            0x0C6 => 8,
            0x0D6 => 8,
            0x0b7 => 4,
            0x0b0 => 4,
            0x0b2 => 4,
            0x0b3 => 4,
            0x0b4 => 4,
            0x02D => 4,
            0x025 => 4,
            0x0AE => 8,
            0x0EE => 8,
            0x026 => 8,
            0x030 => 8,
            0x07A => 4,
            0x0D0 => 8,
            0x0C0 => 8,
            0x0C8 => 8,
            0x0D8 => 8,
            0x0B6 => 8,
            0x0F6 => 8,
            0x035 => 12,
            0x009 => 8,
            0x019 => 8,
            0x029 => 8,
            0x0E9 => 4,
            0x0F8 => 12,
            0x062 => 4,
            0x06B => 4,
            0x012 => 8,
            0x01C => 4,
            0x014 => 4,
            0x07E => 8,
            0x0f9 => 8,
            0x033 => 8,
            0x03B => 8,
            0x039 => 8,
            0x0E8 => 16,
            0x0de => 8,
            0x0BB => 4,
            0x01B => 8,
            0x02B => 8,
            0x045 => 4,
            0x04A => 4,
            0x04B => 4,
            0x060 => 4,
            0x063 => 4,
            0x064 => 4,
            0x065 => 4,
            0x068 => 4,
            0x0ca => 12,
            0x0D2 => 12,
            0x0Da => 12,
            0x0D4 => 12,
            0x0DC => 12,
            0x0D9 => 8,
            0x0C7 => 32,
            0x0CF => 32,
            0x0f2 => 8,
            0x02f => 4,
            0x00A => 8,
            0x08E => 8,
            0x09e => 8,
            0x034 => 12,
            0x027 => 4,
            0x037 => 4,
            0x0B8 => 4,
            0x0B9 => 4,
            0x0BA => 4,
            0x0BC => 4,
            0x0BD => 4,
            0x0BF => 4,
            0x087 => 4,
            0x080 => 4,
            0x082 => 4,
            0x084 => 4,
            0x08f => 4,
            0x08A => 4,
            0x08B => 4,
            0x08C => 4,
            0x08D => 4,
            0x09f => 4,
            0x098 => 4,
            0x099 => 4,
            0x09A => 4,
            0x09B => 4,
            0x09C => 4,
            0x09D => 4,
            0x0A7 => 4,
            0x0A0 => 4,
            0x0A1 => 4,
            0x0A2 => 4,
            0x0A3 => 4,
            0x0A4 => 4,
            0x0A5 => 4,
            0x00F => 4,
            0x0d7 => 32,
            0x0df => 32,
            0x0e7 => 32,
            0x0ef => 32,
            0x0f7 => 32,
            0x0ff => 32,
            0x0fb => 4,
            0x010 => 0,
            other => {
                info!("Cycle calc - No opcode found for {:x}", other);
                std::process::exit(1)
            }
        };

        match cycle_register {
            CycleRegister::VramCycle => self.vram_cycle_num += cycle as u16,
            CycleRegister::TimerCycle => self.timer_cycle_num += cycle as usize,
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

    // Sprites

    fn obj_char_map_bytes(&self) -> Vec<u8> {
        self.memory[0x8800..0x9800].to_vec()
    }

    fn get_oam(&self) -> Vec<u8> {
        self.memory[0xfe00..0xfea0].to_vec()
    }

    fn all_sprites(&self) -> Vec<Sprite> {
        let oam_vec = self.get_oam();
        let mut all_sprites = Vec::new();

        for idx in (0..oam_vec.len()).step_by(BYTES_PER_SPRITE) {
            let y = oam_vec[idx];
            let x = oam_vec[idx + 1];
            let pattern_num = oam_vec[idx + 2];
            let attributes = oam_vec[idx + 3];
            let priority = attributes & 0b10000000 == 0b10000000;
            let y_flip = attributes & 0b01000000 == 0b01000000;
            let x_flip = attributes & 0b00100000 == 0b00100000;
            let palette_num = attributes & 0b00010000 == 0b00010000;

            let sprite = Sprite {
                y,
                x,
                pattern_num,
                attributes,
                priority,
                y_flip,
                x_flip,
                palette_num,
            };

            all_sprites.push(sprite);
        }

        all_sprites
    }

    //##LCD Control Register $0xff40
    pub fn get_lcd(&self) -> u8 {
        self.memory[0xff40]
    }

    pub fn is_lcd_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x80 == 0x80
    }

    pub fn get_window_tile_map_selection(&self) -> u8 {
        if self.memory[0xff40] & 0b01000000 == 0b01000000 {
            1
        } else {
            0
        }
    }

    pub fn is_window_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x20 == 0x20
    }

    pub fn get_tile_data_selection(&self) -> u8 {
        if self.memory[0xff40] & 0b00010000 == 0b00010000 {
            1
        } else {
            0
        }
    }

    pub fn get_bg_tile_map_selection(&self) -> u8 {
        if self.memory[0xff40] & 0x08 == 0x08 {
            1
        } else {
            0
        }
    }

    pub fn get_obj_size_selection(&self) -> u8 {
        if self.memory[0xff40] & 0b00000100 == 0b00000100 {
            1
        } else {
            0
        }
    }

    pub fn is_obj_display_enable(&self) -> bool {
        self.memory[0xff40] & 0x00000010 == 0x00000010
    }

    pub fn is_bg_display(&self) -> bool {
        self.memory[0xff40] & 0x00000001 == 0x00000001
    }

    pub fn window_map(&self) -> Vec<u8> {
        if self.memory[0xff40] & 0x40 == 0x40 {
            return self.memory[0x9c00..0xa000].to_vec();
        } else {
            return self.memory[0x9800..0x9c00].to_vec();
        }
    }

    pub fn bg_window_char_map_bytes(&self) -> Vec<u8> {
        if self.memory[0xff40] & 0x10 == 0x10 {
            return self.memory[0x8000..0x9000].to_vec();
        } else {
            return self.memory[0x8800..0x9800].to_vec();
        }
    }

    pub fn bg_map(&self) -> Vec<u8> {
        if self.memory[0xff40] & 0x08 == 0x08 {
            return self.memory[0x9c00..0xa000].to_vec();
        } else {
            return self.memory[0x9800..0x9c00].to_vec();
        }
    }

    //##LCD Status - STAT $0xff41
    pub fn ly_conincidence_interrupt_enabled(&self) -> bool {
        self.memory[0xff41] & 0b01000000 == 0b01000000
    }

    pub fn oam_interrupt_enabled(&self) -> bool {
        self.memory[0xff41] & 0b00100000 == 0b00100000
    }

    pub fn vblank_interrupt_enabled(&self) -> bool {
        self.memory[0xff41] & 0b00010000 == 0b00010000
    }

    pub fn hblank_interrupt_enabled(&self) -> bool {
        self.memory[0xff41] & 0b00001000 == 0b00001000
    }

    pub fn is_conincidence_flag_on(&self) -> bool {
        self.memory[0xff41] & 0b00000100 == 0b00000100
    }

    // fn lcd_mode(&self) -> LcdMode {
    //     let mode_flag = self.memory[0xff41] & 0b00000011;
    //     match mode_flag {
    //         0 => LcdMode::Hblank,
    //         1 => LcdMode::Vblank,
    //         2 => LcdMode::SearchOAM,
    //         3 => LcdMode::DataTransfer,
    //         _ => {
    //             info!("Invalide lcd mode value");
    //             std::process::exit(1)
    //         }
    //     }
    // }

    fn set_lcd_mode_with_gpu_cycle(&mut self, gpu_cycle: u16) {
        // //##This function is GPU emulation. Mode Flag is read only for gameboy
        let lcdc = self.memory[0xff41].clone();
        let mut bv = BitVec::from_bytes(&[lcdc]);
        if gpu_cycle < 80 {
            bv.set(7, false);
            bv.set(6, true);
        } else if gpu_cycle >= 80 && gpu_cycle < 172 {
            bv.set(7, true);
            bv.set(6, true);
        } else if gpu_cycle >= 172 {
            bv.set(7, false);
            bv.set(6, false);
        }

        self.memory[0xff41] = bv.to_bytes()[0];
        // info!("LCD Status: {:b}", self.memory[0xff41]);
    }

    fn set_lcd_status(&mut self) {
        let mut status = self.read_memory(0xFF41);
        if (false == self.is_lcd_display_enable()) {
            // set the mode to 1 during lcd disabled and reset scanline
            self.vram_cycle_num = 456;
            self.memory[0xFF44] = 0;
            status = status & 252;
            status = status | 0b00000001;
            self.write_memory(0xFF41, status);
            return;
        }

        let currentline = self.read_memory(0xFF44);
        let currentmode = status & 0x3;

        let mut mode = 0;
        let mut reqInt = false;

        // in vblank so set mode to 1
        if currentline >= 144 {
            mode = 1;
            status = status | 0b00000001;
            status = status & 0b11111101;
            reqInt = status & 0b00010000 == 0b00010000;
        } else {
            let mode2bounds = 80;
            let mode3bounds = mode2bounds + 172;

            // mode 2
            if (self.vram_cycle_num >= mode2bounds) {
                mode = 2;
                status = status | 0b00000010;
                status = status & 0b11111110;
                reqInt = status & 0b00100000 == 0b00100000;
            }
            // mode 3
            else if (self.vram_cycle_num >= mode3bounds) {
                mode = 3;
                status = status | 0b00000011;
            }
            // mode 0
            else {
                mode = 0;
                status = status & 0b11111100;
                reqInt = status & 0b00001000 == 0b00001000;
            }
        }

        // just entered a new mode so request interupt
        if (reqInt && (mode != currentmode)) {
            self.request_lcd_interrupt();
        }

        // check the conincidence flag
        if (self.ly() == self.read_memory(0xFF45)) {
            status = status | 0b000000100;
            if (status & 0b01000000 == 0b01000000) {
                self.request_lcd_interrupt();
            }
        } else {
            status = status & 0b11111011;
        }
        self.write_memory(0xFF41, status);
    }

    fn set_lcd_mode_to_vblank(&mut self) {
        //##This function is GPU emulation. Mode Flag is read only for gameboy
        let lcdc = self.memory[0xff41].clone();
        let mut bv = BitVec::from_bytes(&[lcdc]);
        bv.set(7, true);
        bv.set(6, false);

        self.memory[0xff41] = bv.to_bytes()[0];
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

    pub fn get_flag_ime(&self) -> bool {
        self.registers.f.ime
    }

    pub fn get_interrupt_enabled_vblank(&self) -> bool {
        self.memory[0xffff] & 0b00000001 == 0b00000001
    }

    pub fn get_interrupt_enabled_lcd(&self) -> bool {
        self.memory[0xffff] & 0b00000010 == 0b00000010
    }

    pub fn get_interrupt_enabled_timer(&self) -> bool {
        self.memory[0xffff] & 0b00000100 == 0b00000100
    }

    pub fn get_interrupt_enabled_serial(&self) -> bool {
        self.memory[0xffff] & 0b0001000 == 0b0001000
    }

    pub fn get_interrupt_enabled_joypad(&self) -> bool {
        self.memory[0xffff] & 0b00010000 == 0b00010000
    }

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

    pub fn memory(&self) -> *const u8 {
        self.memory.as_ptr()
    }

    pub fn background_map_1(&self) -> Vec<u8> {
        let background_map_1 = self.memory[0x9800..0x9c00].to_vec().clone();
        background_map_1
    }

    pub fn execute_opcode(&mut self) {
        //ff10-ff14 is responsible for sound channel 1
        let pre_ff10 = self.memory[0xff10];
        let pre_ff11 = self.memory[0xff11];
        let pre_ff12 = self.memory[0xff12];
        let pre_ff13 = self.memory[0xff13];
        let pre_ff14 = self.memory[0xff14];

        let instruction = self.memory[self.registers.pc as usize];
        self.add_cycles(instruction, CycleRegister::CpuCycle);
        self.execute_instruction(instruction);
        self.cycle_based_gpu_operation(instruction);

        if self.break_points.contains(&self.registers.pc) {
            self.is_running = false;
        }

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

    pub fn cycle_based_gpu_operation(&mut self, instruction: u8) {
        let vram_cycle_per_ly_inc = 456;

        if self.is_lcd_display_enable() {
            self.add_cycles(instruction, CycleRegister::VramCycle);
            self.set_lcd_status();
            // self.set_lcd_mode_with_gpu_cycle(self.vram_cycle_num);
            if self.vram_cycle_num >= vram_cycle_per_ly_inc {
                self.inc_ly();
                //Resetting vram cycle here
                self.set_vram_cycle(self.vram_cycle_num - vram_cycle_per_ly_inc);
            }
        } else {
            self.set_lcd_mode_to_vblank();
        }
    }

    pub fn execute_opcodes(&mut self, count: u8) {
        let mut canvases = Canvases::new();

        //ff10-ff14 is responsible for sound channel 1
        let pre_ff10 = self.memory[0xff10];
        let pre_ff11 = self.memory[0xff11];
        let pre_ff12 = self.memory[0xff12];
        let pre_ff13 = self.memory[0xff13];
        let pre_ff14 = self.memory[0xff14];

        for _ in 0..count {
            let instruction = self.memory[self.registers.pc as usize];
            self.add_cycles(instruction, CycleRegister::CpuCycle);
            self.cycle_based_gpu_operation(instruction);
            self.execute_instruction(instruction);

            if self.is_lcd_display_enable() && self.should_draw {
                canvases.update_char_map_canvas(self);
                canvases.render_background_map_as_image_data(self);
                canvases.draw_screen_from_memory(self);
            }

            if self.break_points.contains(&self.registers.pc) {
                self.is_running = false;
            }

            if self.is_channel1_changed(pre_ff10, pre_ff11, pre_ff12, pre_ff13, pre_ff14) {
                if self.sound_dirty_flag_check_s1() {
                    self.reset_fm_osc(self.square1());
                }
            }
        }
    }

    pub fn debug_serial_value(&mut self) {
        let character = (self.memory[0xff01] as char).to_string();

        let document = web_sys::window().unwrap().document().unwrap();
        let serial_debug_id = "serial-debug";

        let el = document.get_element_by_id(serial_debug_id).unwrap();

        let current_html = el.inner_html().clone();
        let new_html = format!("{}{}", current_html, character);
        info!("{}", new_html);
        el.set_inner_html(&new_html);
    }

    pub fn execute_opcodes_no_stop(&mut self, count: u32) {
        info!("execute_opcodes_no_stop");
        if self.cpu_paused || !self.is_running {
            return;
        }

        let canvases = Canvases::new();

        //#ff10-ff14 is responsible for sound channel 1
        // let pre_ff10 = self.memory[0xff10];
        // let pre_ff11 = self.memory[0xff11];
        // let pre_ff12 = self.memory[0xff12];
        // let pre_ff13 = self.memory[0xff13];
        // let pre_ff14 = self.memory[0xff14];

        let window = web_sys::window().expect("should have a window in this context");
        let performance = window
            .performance()
            .expect("performance should be available");

        let start_cycle_count = self.total_cycle();
        let mut last_cycle_count = 0;
        let cycle_log_target = 50_000;
        let time_last_draw = performance.now();
        let mut previsou_halt = false;

        loop {
            if self.cpu_paused || !self.is_running {
                debug!(
                    "Exiting loop, paused={:?}, is_running={:?}",
                    self.cpu_paused, self.is_running
                );
                break;
            }

            let instruction = self.read_memory(self.registers.pc);

            // FIXME: Only do this on first time through when the bootrom unmaps itself
            if self.registers.pc == 0xfe {
                info!("PC: 0xfe, instruction: {:x}", instruction);
                if instruction == 0x00e0 {
                    info!("PC: 0xfe, instruction: e0, reg a: {:?}", self.registers.a);
                    // && self.registers.a == 1
                    {
                        info!("Unmapping bootrom...");
                        for idx in 0x00..0xff {
                            // info!("\t{:?} -> {:?}", idx, self.cartridge[idx]);
                            self.memory[idx] = self.cartridge[idx];
                        }
                    }
                }
            }

            if self.total_cycle() - last_cycle_count > cycle_log_target {
                last_cycle_count = self.total_cycle();
            }

            if self.is_halt {
                self.add_cycles(0x00, CycleRegister::TimerCycle);
                self.cycle_based_gpu_operation(instruction);
                self.execute_interuption();
            } else {
                self.cycle_based_gpu_operation(instruction);
                self.execute_instruction(instruction);
            }

            self.add_cycles(instruction, CycleRegister::CpuCycle);
            self.update_timer(instruction);
            self.execute_interuption();

            if instruction == 0x076 {
                //HALT: Pause CPU Until Interrupt
                self.is_halt = true;
                if !self.ime() {
                    let next_instruction = self.read_memory(self.registers.pc);
                    self.registers.set_pc(self.registers.pc - 1);
                    self.execute_instruction(next_instruction);
                } else {
                    // self.execute_instruction(0x00);
                }
                info!(
                    "Update halt to true, pc:{:x}, instruction: {:x}",
                    self.registers.pc,
                    self.read_memory(self.registers.pc)
                );

                // break;
            }

            //quick find me
            if self.break_points.contains(&self.registers.pc)
            // || self.registers.pc == 0x040
            // && self.total_cycle() > 1_000_000
            {
                self.is_running = false;
            }

            let executed_cycles = self.total_cycle() - start_cycle_count;
            if executed_cycles as u32 > count {
                break;
            }

            // TODO: Move this to a handle-serial-bus function
            if self.memory[0xff02] == 0x81 {
                self.debug_serial_value();
                info!("PC: {:x}", self.registers.pc);
                self.memory[0xff02] = 0x0;
            }

            if self.is_lcd_display_enable() && self.should_draw {
                canvases.draw_screen_with_obj(self);
                // canvases.draw_screen_from_memory(self);
                self.should_draw = false;
                // let now = performance.now();
                // let elapsed = now - time_last_draw;
                // let _time_to_sleep = 16.66 - elapsed;
                // if time_to_sleep > 0.0 {
                //     info!("Prepping timeout...!");
                //     Timeout::new(time_to_sleep as u32, || {
                //         info!("Timeout success!");
                //         self.execute_opcodes_no_stop()
                //     })
                //     .forget();

                //     // Timeout::new(time_to_sleep, || self.execute_opcodes_no_stop()).forget();
                // }
                break;
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
            ime: true,
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
            pc: 0x000,
        };

        let boot_rom_content = include_bytes!("boot-rom.gb");
        // let boot_rom_content = include_bytes!("test_rom.gb");
        // let cartridge_content = include_bytes!("cpu_instrs.gb");
        let cartridge_content = include_bytes!("mario.gb");
        // let cartridge_content = include_bytes!("pokered.gbc");
        // let cartridge_content = include_bytes!("tetris.gb");
        // let cartridge_content = include_bytes!("02-interrupts.gb"); //Passed
        // let cartridge_content = include_bytes!("01-special.gb"); //Passed
        // let cartridge_content = include_bytes!("11-op a,(hl).gb"); //Passed
        // let cartridge_content = include_bytes!("07-jr,jp,call,ret,rst.gb"); //Passed!
        // let cartridge_content = include_bytes!("08-misc instrs.gb");//Passed!
        // let cartridge_content = include_bytes!("03-op sp,hl.gb"); //Passed!
        // let cartridge_content = include_bytes!("04-op r,imm.gb"); //Passed!
        // let cartridge_content = include_bytes!("05-op rp.gb"); //Passed!
        // let cartridge_content = include_bytes!("06-ld r,r.gb");//Passed!
        // let cartridge_content = include_bytes!("09-op r,r.gb"); //Passed!
        // let cartridge_content = include_bytes!("10-bit ops.gb"); //Passed!

        let full_memory_capacity = 0x10000;

        let head = boot_rom_content;
        let body = &cartridge_content[0x100..0x8000// (cartridge_content.len())
        ];

        // let body = cartridge_content;

        let mut full_memory: Vec<u8> = Vec::new();

        full_memory.extend_from_slice(head);
        full_memory.extend_from_slice(body);

        full_memory.resize_with(full_memory_capacity, || 0);
        info!("memory size: {:x}", full_memory.len());

        // Vblank
        // full_memory[0xff44] = 0x90;

        let pixel_byte_vec = full_memory[0x8000..0x8800].to_vec();
        let image_data = pixels_to_image_data(pixel_byte_vec.clone());

        //FmOsc Here

        let fm_osc = match Gameboy::initialize_fm_osc() {
            Ok(something) => something,
            _ => panic!("Failed initialize FmOsc"),
        };

        let mut ram_bank_memory = Vec::new();
        ram_bank_memory.resize(0x8000, 0);

        Gameboy {
            background_width: BACKGROUND_WIDTH,
            background_height: BACKGROUND_HEIGHT,
            screen_width: SCREEN_WIDTH,
            screen_height: SCREEN_HEIGHT,
            registers,
            fm_osc,
            image_data,
            total_cycle_num: 0,
            vram_cycle_num: 0,
            timer_cycle_num: 0,
            timer: 0,
            is_running: false,
            is_halt: false,
            break_points: vec![],
            cpu_clock: 0,
            cpu_paused: false,
            should_draw: false,
            cartridge: cartridge_content.to_vec(),
            mbc: Gameboy::get_mbc_from_memory(&full_memory),
            rom_bank: 1,
            ram_bank: 0,
            ram_bank_memory,
            is_ram_enabled: false,
            is_rom_banking_enabled: false,
            memory: full_memory,
            joypad_state: 0xff,
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

    pub fn char_map_to_image_data(&mut self) -> Vec<u8> {
        let pixels_vec = self.bg_window_char_map_bytes();
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
    let cartridge_content = include_bytes!("cpu_instrs.gb");
    let cartridge = &cartridge_content[0x0..(cartridge_content.len())];

    let mut ram_bank_memory = Vec::new();
    ram_bank_memory.resize(0x8000, 0);

    let gameboy = Gameboy {
        // From serialized
        registers: serializeable.registers.clone(),
        total_cycle_num: serializeable.total_cycle_num,
        vram_cycle_num: serializeable.vram_cycle_num,
        timer_cycle_num: serializeable.timer_cycle_num,
        timer: serializeable.timer,
        cpu_clock: serializeable.cpu_clock,
        break_points: serializeable.break_points.clone(),
        // Default, non-serializable values
        background_width: BACKGROUND_WIDTH,
        background_height: BACKGROUND_HEIGHT,
        screen_width: SCREEN_WIDTH,
        screen_height: SCREEN_HEIGHT,
        fm_osc,
        image_data,
        should_draw: false,
        is_running: false,
        is_halt: false,
        cpu_paused: false,
        cartridge: cartridge.to_vec(),
        mbc: Gameboy::get_mbc_from_memory(&full_memory),
        rom_bank: 1,
        ram_bank: 0,
        ram_bank_memory,
        is_ram_enabled: false,
        is_rom_banking_enabled: false,
        memory: full_memory,
        joypad_state: 0xff,
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
        0x0CB => "CB function",
        0x017 => "RLA",
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
        0x000 => "NOP",
        0x0CE => "ADC A,#",
        0x066 => "LD H,(hl)",
        0x0CC => "CALL Z, nn",
        0x00B => "DEB BC",
        0x003 => "INC BC",
        0x073 => "LD (HL),E",
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
        0x0b1 => "OR C",
        0x03f => "CCF",
        0x042 => " LD B,D",
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
        0x0e5 => "PUSH HL",
        0x0f5 => "PUSH AF",
        0x0D5 => " PUSH DE",
        0x001 => "LD BC, nn",
        0x0fa => "LD A, (nn)",
        0x02C => "INC L",
        0x0A8 => " XOR B",
        0x0A9 => " XOR C",
        0x0AA => " XOR D",
        0x0AB => " XOR E",
        0x0AC => " XOR H",
        0x0AD => " XOR L",
        0x0D6 => " SUB n",
        0x0b7 => "OR A",
        0x0b0 => "OR B",
        0x0b2 => "OR D",
        0x0b3 => "OR E",
        0x0b4 => "OR H",
        0x02D => "DEC L",
        0x025 => "DEC H",
        0x0AE => " XOR (HL)",
        0x0EE => " XOR n",
        0x026 => "LD H, *1byte",
        0x030 => "JR NC,*one byte",
        0x07A => "LD A, D",
        0x0D0 => "RET NC",
        0x0C0 => "RET NZ",
        0x0C8 => "RET Z",
        0x0D8 => "RET C",
        0x0B6 => "OR (HL)",
        0x0F6 => "OR n",
        0x035 => "DEC (HL)",
        0x009 => "ADD HL, BC",
        0x019 => "ADD HL, DE",
        0x029 => "ADD HL, HL",
        0x0E9 => " JP (HL)",
        0x0F8 => "LDHL SP,n",
        0x062 => "LD H,D",
        0x06B => "LD L,E",
        0x012 => "LD (DE), A",
        0x01C => "INC E",
        0x014 => "INC D",
        0x07E => "LD A, (HL)",
        0x0f9 => "LD SP, HL",
        0x033 => "INC SP",
        0x03B => "DEC SP",
        0x039 => "ADD HL, SP",
        0x0E8 => "ADD SP, n",
        0x0de => "SCB A,n",
        0x0BB => "CP E",
        0x01B => "DEC DE",
        0x02B => "DEC HL",
        0x045 => "LD B, L",
        0x04a => "LD C, D",
        0x04b => "LD C, E",
        0x037 => "SCF",
        0x0B8 => "CP B",
        0x0B9 => "CP C",
        0x0BA => "CP D",
        0x0BC => "CP H",
        0x0BD => "CP L",
        0x0BF => "CP A",
        0x087 => "ADD A, A",
        0x080 => "ADD A, B",
        0x081 => "ADD A, C",
        0x082 => "ADD A, D",
        0x083 => "ADD A, E",
        0x084 => "ADD A, H",
        0x085 => "ADD A, L",
        0x086 => "ADD A, (HL)",
        0x0C6 => "ADD A, #",
        0x08f => "ADC A, A",
        0x08A => "ADC A, D",
        0x08B => "ADC A, E",
        0x08C => "ADC A, H",
        0x08D => "ADC A, L",
        0x08E => "ADC A, (HL)",
        0x09f => "SBC A, A",
        0x098 => "SBC A, B",
        0x099 => "SBC A, C",
        0x09A => "SBC A, D",
        0x09B => "SBC A, E",
        0x09C => "SBC A, H",
        0x09D => "SBC A, L",
        0x09E => "SBC A, (HL)",
        0x0A7 => "AND A",
        0x0A0 => "AND B",
        0x0A1 => "AND C",
        0x0A2 => "AND D",
        0x0A3 => "AND E",
        0x0A4 => "AND H",
        0x0A5 => "AND L",
        0x00F => "RRCA",
        0x0d7 => "RST $10",
        0x0df => "RST $18",
        0x0e7 => "RST $20",
        0x0ef => "RST $28",
        0x0f7 => "RST $30",
        0x0ff => "RST $38",
        0x0fb => "EI",
        _other => "???",
    };

    String::from(result)
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    utils::set_panic_hook();
}
