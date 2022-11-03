use core::ptr;

// qemu puts platform-level interrupt controller (PLIC) here.
pub const PLIC_BASE: usize = 0xC00_0000;

pub const PCI_PINA: u32 = 33;

use riscv::register::sie;

pub fn irq_handler() {
    
    // which device interrupted?
    if let Some(irq) = plic_claim() {
        match irq {
            _ => panic!("unsupported IRQ {}", irq),
        }
        // Tell the PLIC we've served the IRQ
        plic_complete(irq);
    }
}




pub fn device_init(){
    plic_init();
    plic_init_hart();
}

pub fn plic_init() {
    write(PLIC_BASE + (PCI_PINA * 4) as usize, 1);
}


pub fn plic_init_hart() {

    let hart_id = hart_id();

    // Set UART's enable bit for this hart's S-mode. 
    write(plic_senable(hart_id), (1 << PCI_PINA));

    // Set this hart's S-mode pirority threshold to 0. 
    write(plic_spriority(hart_id), 0);

    unsafe {
        sie::set_sext();
    }
}

fn plic_senable(hart_id: usize) -> usize {
    PLIC_BASE + 0x2080 + hart_id * 0x100
}

fn plic_spriority(hart_id: usize) -> usize {
    PLIC_BASE + 0x201000 + hart_id * 0x2000
}

fn plic_sclaim(hart_id: usize) -> usize {
    PLIC_BASE + 0x201004 + hart_id * 0x2000
}

/// Ask the PLIC what interrupt we should serve. 
pub fn plic_claim() -> Option<u32> {
    let hart_id = hart_id();
    let interrupt = read(plic_sclaim(hart_id));
    if interrupt == 0 {
        None
    } else {
        Some(interrupt)
    }
}


/// Tell the PLIC we've served the IRQ
pub fn plic_complete(interrupt: u32) {
    let hart_id = hart_id();
    write(plic_sclaim(hart_id), interrupt);
}


fn write(addr: usize, val: u32) {
    unsafe {
        ptr::write(addr as *mut u32, val);
    }
}

fn read(addr: usize) -> u32 {
    unsafe {
        ptr::read(addr as *const u32)
    }
}