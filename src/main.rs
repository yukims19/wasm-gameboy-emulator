mod utils;

use bit_vec::BitVec;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

struct Flag {
    z: bool, //(0x80) if zero
    n: bool, //(0x40) if subtraction
    h: bool, //(0x20) if the lower half of the byte overflowed past 15
    c: bool, //(0x10) if result over 255 or under 0
}

impl Flag {
    fn set_flag(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.z = z;
        self.n = n;
        self.h = h;
        self.c = c;
    }
}

struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    // z_flag: bool,
    // n_flag: bool,
    f: Flag, //Control last operation result
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

impl Registers {
    fn execute_instruction(&mut self, opcode: u8, memory: &mut Vec<u8>) {}
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

#[wasm_bindgen]
pub struct Canvas {
    width: u8,
    height: u8,
    pixels: Vec<Pixel>,
    memory: Vec<u8>, //consist of 256*256 pixels or 32*32 tiles
                     //only 160*144 pixels can be displayed on screen
}

#[wasm_bindgen]
impl Canvas {
    fn get_index(&self, row: u8, column: u8) -> usize {
        (row * self.width + column) as usize
    }

    pub fn width(&self) -> u8 {
        self.width
    }

    pub fn height(&self) -> u8 {
        self.height
    }

    pub fn pixels(&self) -> *const Pixel {
        self.pixels.as_ptr()
    }

    pub fn memory(&self) -> *const u8 {
        self.memory.as_ptr()
    }

    pub fn new() -> Canvas {
        let width = 160;
        let height = 144;

        let flag = Flag {
            z: false,
            n: false,
            h: false,
            c: false,
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
            sp: 0,
            pc: 0,
        };

        let boot_rom_content = include_bytes!("boot-rom.gb");
        let cartrage_header: Vec<u8> = vec![
            0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c,
            0x00, 0x0d, 0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6,
            0xdd, 0xdd, 0xd9, 0x99, 0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc,
            0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
        ];
        let full_memory_capacity = 0xffff;

        let mut full_memory: Vec<u8> = Vec::with_capacity(full_memory_capacity);
        full_memory.extend_from_slice(boot_rom_content);
        full_memory.resize_with(full_memory_capacity, || 0);
        for (idx, cartrage_value) in cartrage_header.iter().enumerate() {
            full_memory[0x104 + idx] = cartrage_value.clone();
        }
        println!("memory:::::::{}", full_memory[0x104]);
        // //TODO: IMPORTANT! here pretending vertical-blank period
        // full_memory[0xff44] = 0x90;
        // //TODO: IMPORTANT! here to pass checksum
        // full_memory[0x14D] = -25i8 as u8;

        // let pixels = Canvas::tile(&full_memory[0x8000..0x8fff]);
        let pixel_byte_vec = &full_memory[0x8000..0x8fff].to_vec();
        let pixels = Canvas::tile(cartrage_header);

        Canvas {
            width,
            height,
            pixels,
            memory: full_memory,
        }
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
        while idx < byte_vec.len() {
            for i in (idx..idx + 16).step_by(2) {
                let row = Canvas::tile_row(byte_vec[i], byte_vec[i + 1]);
                tile.extend(row);
            }
            idx = idx + 16;

            tile_vec.append(&mut tile);
        }
        tile_vec
    }

    // pub fn render(&self) -> String {
    //     self.to_string()
    // }
}

// use std::fmt;

// impl fmt::Display for Canvas {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         for line in self.pixels.as_slice().chunks(
//             8, // self.width as usize
//         ) {
//             for &pixel in line {
//                 let symbol = {
//                     if pixel == Pixel::White {
//                         '0'
//                     } else if pixel == Pixel::LightGray {
//                         '1'
//                     } else if pixel == Pixel::DarkGray {
//                         '2'
//                     } else if pixel == Pixel::Black {
//                         '3'
//                     } else {
//                         '?'
//                     }
//                 };

//                 write!(f, "{}", symbol)?;
//             }
//             write!(f, "\n")?;
//         }
//         Ok(())
//     }
// }
