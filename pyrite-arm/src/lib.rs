#[macro_use]
extern crate pyrite_common;

pub mod alu;
pub mod arm;
pub mod thumb;
pub mod cpu;
pub mod clock;
pub mod registers;

pub use cpu::ArmCpu;
pub use cpu::ArmMemory;

const FLAT_MEMORY_BLOCK_SIZE: u32 = 1024;

/// Simple implementer of ArmMemory that is flat with a max size.
/// All data and code access are 1 cycles with 0 wait cycles.
pub struct FlatMemory {
    data: Vec<u8>,
    max_size: u32,
}

impl FlatMemory {
    pub fn new(max_size: u32) -> FlatMemory {
        let vsize  = if max_size < FLAT_MEMORY_BLOCK_SIZE { max_size as usize } else { FLAT_MEMORY_BLOCK_SIZE as usize };
        let mut v = Vec::with_capacity(vsize);
        v.resize(vsize, 0);

        FlatMemory {
            data: v,
            max_size: max_size,
        }
    }

    fn try_access(&mut self, addr: u32, data_size: u32, grow: bool) {
        let required_length = addr + data_size;

        if self.data.len() < required_length as usize {
            assert!(required_length <= self.max_size, "flat memory max size reached. ({} > {})", required_length, self.max_size);

            if grow {
                let mut new_size = required_length - (required_length % FLAT_MEMORY_BLOCK_SIZE);
                if new_size < required_length { new_size += FLAT_MEMORY_BLOCK_SIZE; }
                if new_size > self.max_size { new_size = self.max_size; }

                self.data.resize(new_size as usize, 0);
            }
        }
    }

    pub fn set_bytes(&mut self, offset: u32, data: &[u8]) {
        let offset = offset as usize;
        let required_size = offset + data.len();
        assert!(required_size <= self.max_size as usize, "storage required larger than max size ({} > {})", required_size, self.max_size);
        if self.data.len() < required_size { self.data.resize(required_size, 0); }

        self.data[offset..(offset + data.len())].copy_from_slice(data);
    }

    pub fn get_bytes(&mut self, offset: u32, dest: &mut [u8]) {
        let offset = offset as usize;
        // now I could go through all the trouble of not growing the array
        // while copying a segment, or I could just dooooo...this:
        let required_size = offset + dest.len();
        assert!(required_size <= self.max_size as usize, "storage required larger than max size ({} > {})", required_size, self.max_size);
        if self.data.len() < required_size { self.data.resize(required_size, 0); }

        dest.copy_from_slice(&self.data[offset..(offset + dest.len())]);
    }

    pub fn ref_bytes(&mut self, offset: u32, len: u32) -> &[u8] {
        let offset = offset as usize;
        let len = len as usize;

        // now I could go through all the trouble of not growing the array
        // while copying a segment, or I could just dooooo...this:
        let required_size = offset + len;
        assert!(required_size <= self.max_size as usize, "storage required larger than max size ({} > {})", required_size, self.max_size);
        if self.data.len() < required_size { self.data.resize(required_size, 0); }
        
        &self.data[offset..(offset + len)]
    }
}

impl ArmMemory for FlatMemory {
    fn load8(&mut self, addr: u32) -> u8 {
        self.try_access(addr, 1, false);
        self.data[addr as usize]
    }

    fn store8(&mut self, addr: u32, value: u8) {
        self.try_access(addr, 1, true);
        self.data[addr as usize] = value;
    }

    fn load16(&mut self, addr: u32) -> u16 {
        self.try_access(addr, 2, false);
        read16_le(&self.data, addr as usize)
    }

    fn store16(&mut self, addr: u32, value: u16) {
        self.try_access(addr, 2, true);
        write16_le(&mut self.data, addr as usize, value)
    }

    fn load32(&mut self, addr: u32) -> u32 {
        self.try_access(addr, 4, false);
        read32_le(&self.data, addr as usize)
    }

    fn store32(&mut self, addr: u32, value: u32) {
        self.try_access(addr, 4, true);
        write32_le(&mut self.data, addr as usize, value)
    }

    fn code_access_seq8(&self, _addr: u32) -> u32 { 1 }
    fn data_access_seq8(&self, _addr: u32) -> u32 { 1 }

    fn code_access_nonseq8(&self, _addr: u32) -> u32 { 1 }
    fn data_access_nonseq8(&self, _addr: u32) -> u32 { 1 }

    fn code_access_seq16(&self, _addr: u32) -> u32 { 1 }
    fn data_access_seq16(&self, _addr: u32) -> u32 { 1 }

    fn code_access_nonseq16(&self, _addr: u32) -> u32 { 1 }
    fn data_access_nonseq16(&self, _addr: u32) -> u32 { 1 }

    fn code_access_seq32(&self, _addr: u32) -> u32 { 1 }
    fn data_access_seq32(&self, _addr: u32) -> u32 { 1 }

    fn code_access_nonseq32(&self, _addr: u32) -> u32 { 1 }
    fn data_access_nonseq32(&self, _addr: u32) -> u32 { 1 }
}

/// Reads a u16 from a byte array in little endian byte order.
#[inline]
fn read16_le(mem: &[u8], offset: usize) -> u16 {
    assert!(mem.len() > offset + 1, "16bit read out of range (offset: {}, len: {})", offset, mem.len());
    (mem[offset] as u16) | ((mem[offset + 1] as u16) <<  8)
}

/// Reads a u32 from a byte array in little endian byte order.
#[inline]
fn read32_le(mem: &[u8], offset: usize) -> u32 {
    assert!(mem.len() > offset + 3, "32bit read out of range (offset: {}, len: {})", offset, mem.len());
    (mem[offset] as u32) |
        ((mem[offset + 1] as u32) <<  8) |
        ((mem[offset + 2] as u32) << 16) |
        ((mem[offset + 3] as u32) << 24)
}

/// Writes a u16 into a byte array in little endian byte order.
#[inline]
fn write16_le(mem: &mut [u8], offset: usize, value: u16) {
    assert!(mem.len() > offset + 1, "16bit write out of range (offset: {}, len: {})", offset, mem.len());
    mem[offset] = value as u8;
    mem[offset + 1] = (value >> 8) as u8;
}

/// Writes a u32 into a byte array in little endian byte order.
#[inline]
fn write32_le(mem: &mut [u8], offset: usize, value: u32) {
    assert!(mem.len() > offset + 3, "32bit write out of range (offset: {}, len: {})", offset, mem.len());
    mem[offset] = value as u8;
    mem[offset + 1] = (value >>  8) as u8;
    mem[offset + 2] = (value >> 16) as u8;
    mem[offset + 3] = (value >> 24) as u8;
}
