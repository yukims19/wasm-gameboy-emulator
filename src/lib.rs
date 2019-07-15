use log::info;
use log::Level;

mod utils;

use bit_vec::BitVec;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub struct Channel {
    sweep_time: f32,
    is_sweep_increase: bool,
    sweep_shift_num: u8,
    wave_duty_pct: f32,
    sound_length_sec: f32,
    volume: u8,
    is_envelop_increase: bool,
    envelop_shift_num: u8,
    frequency: f32,
    is_restart: bool,
    is_use_length: bool,
}

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
                self.set_hl(h_l - 1 as u16);
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
                let value = memory[0xff00 + following_byte as usize];
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
                self.push_stack(self.sp, memory, next_instruction_address);
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
                self.push_stack(self.sp, memory, bc_value);
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
                //TODO: Confirm wrapping_sub is correct
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
                let value = self.c - 1;
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
                let value = self.e - 1;
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
                let value = self.a - 1;
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
                let value = self.d - 1;
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
                println!(
                    "a:{:x}, hl:{:x}, val:{:x}, sum:{:x}",
                    self.a,
                    h_l,
                    memory[h_l as usize],
                    self.a.wrapping_add(memory[h_l as usize])
                );
                //TODO: Comfirm wrapping add
                let value = self.a.wrapping_add(memory[h_l as usize]);
                // let value = self.a + memory[h_l as usize];
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

            other => {
                println!("No opcode found for {:x} at {:x}", other, pointer);
                // println!("{:?}", self);
                //println!("Cartrage Header---{:x?}", &memory[0x104..0x133]);
                //println!("Cartrage vram---{:x?}", &memory[0x9800..0x9bff]);
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

    fn push_stack(&mut self, sp: u16, memory: &mut Vec<u8>, value: u16) {
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
pub struct Canvas {
    background_width: u8,
    background_height: u8,
    screen_width: u8,
    screen_height: u8,
    image_data: Vec<u8>,
    registers: Registers,
    memory: Vec<u8>, //consist of 256*256 pixels or 32*32 tiles
                     //only 160*144 pixels can be displayed on screen
}

#[wasm_bindgen]
impl Canvas {
    pub fn background_width(&self) -> u8 {
        self.background_width
    }

    pub fn screen_width(&self) -> u8 {
        self.screen_width
    }

    pub fn background_height(&self) -> u8 {
        self.background_height
    }

    pub fn screen_height(&self) -> u8 {
        self.screen_height
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

        let wave_duty_raw = self.memory[0xff11] & 0b11000000u8;
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
        //        let frequency = self.memory[0xff13] + self.memory[0xff14] & 0b00000111u8;
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
            frequency: 440.0,
            is_restart,
            is_use_length,
        }
    }
    // pub fn is_sound_on(&self) -> bool {
    //     self.memory[0x0008] > 0
    // }

    // pub fn can_play_boot_sound_1(&self) -> bool {
    //     self.registers.h == 0x62
    // }

    // pub fn can_play_boot_sound_2(&self) -> bool {
    //     self.registers.h == 0x64
    // }

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
        let pixels = Canvas::tile(pixel_byte_vec);

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
        let instruction = self.memory[self.registers.pc as usize];
        self.registers
            .execute_instruction(instruction, &mut self.memory);
    }

    pub fn execute_opcodes(&mut self, count: u8) {
        for _ in 0..count {
            let instruction = self.memory[self.registers.pc as usize];
            self.registers
                .execute_instruction(instruction, &mut self.memory);
        }
    }

    pub fn new() -> Canvas {
        let background_width = 255;
        let background_height = 255;
        let screen_width = 160;
        let screen_height = 144;

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
            sp: 0xffff,
            pc: 0,
        };

        let boot_rom_content = include_bytes!("boot-rom.gb");
        let cartridge_content = include_bytes!("mario.gb");

        let head = boot_rom_content;
        let body = &cartridge_content[0x100..(cartridge_content.len())];

        let full_memory_capacity = 0xffff;

        let head = boot_rom_content;
        let body = &cartridge_content[0x100..(cartridge_content.len())];

        let mut full_memory: Vec<u8> = Vec::with_capacity(full_memory_capacity);

        full_memory.extend_from_slice(head);
        full_memory.extend_from_slice(body);

        full_memory.resize_with(full_memory_capacity, || 0);

        // //TODO: IMPORTANT! here pretending vertical-blank period
        full_memory[0xff44] = 0x90;
        // //TODO: IMPORTANT! here to pass checksum
        full_memory[0x14D] = -25i8 as u8;

        let pixel_byte_vec = full_memory[0x8000..0x8800].to_vec();
        let image_data = pixels_to_image_data(pixel_byte_vec.clone());

        let pixels = Canvas::tile(pixel_byte_vec);

        Canvas {
            background_width,
            background_height,
            screen_width,
            screen_height,
            registers,
            image_data,
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

        // console_log("Tile: idx={}")

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

    pub fn char_map_to_image_data(&mut self) -> Vec<u8> {
        let pixels_vec = self.memory[0x8000..0x8800].to_vec();
        let new_image_data = pixels_to_image_data(pixels_vec);

        self.image_data = new_image_data.clone();

        new_image_data
    }
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
        _other => "???",
    };

    String::from(result)
}
