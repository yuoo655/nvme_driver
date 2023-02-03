use core::ptr::{read_volatile, write_volatile};

pub trait IoMapper {
    fn ioremap(start: usize, size: usize) -> usize;
    fn iounmap(start: usize);
}

pub struct IoMem<const SIZE: usize, I: IoMapper> {
    ptr: usize,
}

impl<const SIZE: usize, I: IoMapper> IoMem<SIZE, I> {
    pub fn new(start: usize, size: usize) -> Self {
        let addr = I::ioremap(start, size);
        Self { ptr: addr as usize }
    }

    pub fn readb(&self, offset: usize) -> u8 {
        let val = unsafe { read_volatile((self.ptr + offset) as *mut u8) };
        val
    }

    pub fn readw(&self) -> u16 {
        let val = unsafe { read_volatile(self.ptr as *mut u16) };
        val
    }

    pub fn readl(&self, offset: usize) -> u32 {
        let val = unsafe { read_volatile((self.ptr + offset) as *mut u32) };
        val
    }

    pub fn writeb(&self, val: u8) {
        unsafe {
            write_volatile(self.ptr as *mut u8, val);
        }
    }

    pub fn writew(&self, val: u16) {
        unsafe {
            write_volatile(self.ptr as *mut u16, val);
        }
    }

    pub fn writel(&self, val: u32, offset: usize) {
        unsafe {
            write_volatile((self.ptr + offset) as *mut u32, val);
        }
    }
}

impl<const SIZE: usize, I: IoMapper> Drop for IoMem<SIZE, I> {
    fn drop(&mut self) {
        // SAFETY: By the type invariant, `self.ptr` is a value returned by a previous successful
        // call to `ioremap`.
        unsafe { I::iounmap(self.ptr as _) };
    }
}
