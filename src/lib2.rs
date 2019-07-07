mod utils;

use bit_vec::BitVec;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}

#[wasm_bindgen]
pub struct Canvas {
    width: u8,
    height: u8,
    cells: Vec<Vec<Cell>>,
    //consist of 256*256 pixels or 32*32 tiles
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

    pub fn cells(&self) -> *const Vec<Cell> {
        self.cells.as_ptr()
    }

    pub fn new() -> Canvas {
        let width = 16;
        let height = 8;

        let _logo: Vec<u8> = vec![
            0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c,
            0x00, 0x0d, 0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6,
            0xdd, 0xdd, 0xd9, 0x99, 0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc,
            0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
        ];

        let tile_vec = Canvas::tile_vec(vec![
            0xFF, 0x00, 0x7E, 0xFF, 0x85, 0x81, 0x89, 0x83, 0x93, 0x85, 0xA5, 0x8B, 0xC9, 0x97,
            0x7E, 0xFF,
        ]);

        Canvas {
            width,
            height,
            cells: tile_vec,
        }
    }

    fn tile_row(first_b: u8, second_b: u8) -> Vec<Cell> {
        let low_bits = BitVec::from_bytes(&[first_b]);
        let high_bits = BitVec::from_bytes(&[second_b]);
        let mut row = Vec::new();

        for idx in 0..8 {
            match (low_bits[idx], high_bits[idx]) {
                (false, false) => row.push(Cell::White),
                (false, true) => row.push(Cell::LightGray),
                (true, false) => row.push(Cell::DarkGray),
                (true, true) => row.push(Cell::Black),
            }
        }
        row
    }

    fn tile_vec(byte_vec: Vec<u8>) -> Vec<Vec<Cell>> {
        if byte_vec.len() % 16 == 0 {
            let mut tile = Vec::new();
            let mut tile_vec = Vec::new();
            let mut idx = 0;
            while idx < byte_vec.len() {
                for i in (idx..idx + 16).step_by(2) {
                    let row = Canvas::tile_row(byte_vec[i], byte_vec[i + 1]);
                    tile.extend(row)
                }
                idx = idx + 16
            }

            tile_vec.push(tile);

            tile_vec
        } else {
            println!("Passed bytes vec cannot represent a full tile");
            panic!("Passed bytes vec cannot represent a full tile");
        }
    }

    // pub fn render(&self) -> String {
    //     self.to_string()
    // }
}

use std::fmt;

// impl fmt::Display for Canvas {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         for line in self.cells.as_slice().chunks(
//             8, // self.width as usize
//         ) {
//             for &cell in line {
//                 let symbol = {
//                     if cell == Cell::White {
//                         '0'
//                     } else if cell == Cell::LightGray {
//                         '1'
//                     } else if cell == Cell::DarkGray {
//                         '2'
//                     } else if cell == Cell::Black {
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
