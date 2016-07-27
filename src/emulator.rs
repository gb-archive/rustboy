use opengl_graphics::*;
use piston::input::*;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::fmt;
use std::path::Path;

use cpu::Cpu;
use mmu::Memory;

pub struct Emulator {
	pub cpu: Cpu,
	pub mem: Memory,
	pub gl: GlGraphics,			// OpenGL drawing backend
	pub rom_loaded: Vec<u8>,	// Rom in heap
	pub rom_header: CartridgeHeader,
}

impl Emulator {

	// Render screen
	pub fn render(&mut self, args: &RenderArgs) {
		use graphics::*;
	}

	// Update state
	pub fn update(&mut self, args: &UpdateArgs) {

	}

	pub fn read_header(&mut self) {
		self.rom_header = read_header_impl(&self);
	}
}

fn open_rom<P: AsRef<Path>>(rom_path: P) -> io::Result< Vec<u8> > {

	// try! to open the file
	let mut rom_file = try!(File::open(rom_path));

	// Create the buffer
	let mut rom_buffer: Vec<u8> = Vec::new();

	// Read the data
	try!(rom_file.read_to_end(&mut rom_buffer));

	// no panic! issued so we're good
	return Ok(rom_buffer);
}

// Wrapper for open_rom
pub fn try_open_rom<P: AsRef<Path>>(rom_path: P) -> Vec<u8> {

	// Create a Path and a Display to the desired file
	let rom_display = rom_path.as_ref().display();

	// Call open_rom and handle Result
	match open_rom(&rom_path) {
        Err(why) => 
        	panic!("Couldn't open rom {}: {}", rom_display,
                                                   why.description()),
		Ok(data) => {
			println!("Read {} bytes from ROM: {}.", data.len(), rom_display);
			return data
		},
	};
}

fn read_header_impl(emu: &Emulator) -> CartridgeHeader {

	use std::mem;
	use std::slice;
	use std::io::Read;

	const HEADER_SIZE: usize = 0x50;
	const HEADER_OFFSET: usize = 0x100;

	let mut buffer: [u8; HEADER_SIZE] = [0u8; HEADER_SIZE];

	for i in 0..HEADER_SIZE {
		buffer[i] = emu.rom_loaded[i + HEADER_OFFSET];
	}

	let mut buffer_slice: &[u8] = &buffer;

    let mut header: CartridgeHeader = Default::default();

    unsafe {
        let header_slice = slice::from_raw_parts_mut(
            &mut header as *mut _ as *mut u8,
            HEADER_SIZE
        );
        
    	// `read_exact()` comes from `Read` impl for `&[u8]`
    	buffer_slice.read_exact(header_slice).unwrap();
	}

	println!("Read header: {:#?}", header);
	header
}

#[derive(Default)]
#[repr(C, packed)]
pub struct CartridgeHeader {
	// Usually a NOP and a JP to 0x0150
	entry_point: [u16; 2],

	// Bitmap of the Nintendo logo
	// Use u16 so that we can use the default Default trait
	// TODO: Don't be lazy and implement our own Default trait
	nintendo_logo: [u16; 24],

	// Game title in upper case ASCII
	game_title: [u8; 12],
	manufacturer_code: [u8; 4],
		
	//80h - Game supports CGB functions, but works on old gameboys also.
	//C0h - Game works on CGB only (physically the same as 80h).
	//cgb_flag: u8,

	// Used by newer games
	new_licence_code: [u8; 2],

	// Specifies whether the game supports SGB functions, common values:
	// 00h = No SGB functions (Normal Gameboy or CGB only game)
	// 03h = Game supports SGB functions
	// The SGB disables its SGB functions if this byte
	// is set to another value than 03h.
	sgb_flag: u8,

	// Specifies which Memory Bank Controller (if any) is used in the
	// cartridge, and if further external hardware exists in the cartridge.
	cartridge_type: u8,

	// Typically calculated as "32KB << N"
	rom_size: u8,

	// Specifies the size of the external RAM in the cartridge (if any).
	// 00h - None
	// 01h - 2 KBytes
	// 02h - 8 Kbytes
	// 03h - 32 KBytes (4 banks of 8KBytes each)
	ram_size: u8,

	// 0 = Japanese, 1 = Non-Japanese
	dest_code: u8,

	// If 0x33 new_licence_code is used instead
	old_licence_code: u8,

	// Usually 0
	rom_version_number: u8,
	header_checksum: u8,
	global_checksum: u16,
}

impl fmt::Debug for CartridgeHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	use std::str;

        write!(f, "CartridgeHeader {{
        	entry_point: {:X} {:X}
        	game_title: {:?}
        	sgb_flag: {}
        	cartridge_type: {}
        	rom_size: {}
        	dest_code: {}
        	header_checksum: {:X}
        	global_checksum: {:X}
        }}",
        	self.entry_point[0], self.entry_point[1],
        	str::from_utf8(&self.game_title).unwrap(),
        	self.sgb_flag,
        	self.cartridge_type,
        	self.rom_size,
        	self.dest_code,
        	self.header_checksum,
        	self.global_checksum
        	)
    }
}

#[cfg(test)]
mod emu_tests {
	use super::*;

	#[test]
	fn header_size() {
		use std::mem;
		assert_eq!(0x0150-0x0100, mem::size_of::<CartridgeHeader>());
	}
}