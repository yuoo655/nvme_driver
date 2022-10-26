#![allow(unused_variables)]
#![allow(dead_code)]

extern crate alloc;
extern crate pci;
extern crate log;

pub use log::*;
use alloc::{format, vec::Vec};
use pci::{PCIDevice, Location, scan_bus, BAR};
use pci::{PortOps, CSpaceAccessMethod};
use core::ptr::{read_volatile, write_volatile};

pub fn pci_scan() -> Option<u32> {
    let mut dev_list = Vec::new();
    let pci_iter = unsafe { scan_bus(&PortOpsImpl, PCI_ACCESS) };
    println!("--------- PCI bus:device:function ---------");
    for dev in pci_iter {
        println!(
            "PCI: {}:{}:{} {:04x}:{:04x} ({} {}) irq: {}:{:?}",
            dev.loc.bus,
            dev.loc.device,
            dev.loc.function,
            dev.id.vendor_id,
            dev.id.device_id,
            dev.id.class,
            dev.id.subclass,
            dev.pic_interrupt_line,
            dev.interrupt_pin,
        );
        init_driver(&dev);
        dev_list.push(dev.loc);
    }
    println!("---------");

    let pci_num = dev_list.len();

    println!("Found PCI number is {}", pci_num);
    Some(pci_num as u32)
}

pub fn init_driver(dev: &PCIDevice) {
    let name = format!("enp{}s{}f{}", dev.loc.bus, dev.loc.device, dev.loc.function);
    match (dev.id.vendor_id, dev.id.device_id) {
        (0x1b36, 0x10) => {
            if let Some(BAR::Memory(addr, _len, _, _)) = dev.bars[0] {
                println!("Found NVME {:?} dev {:?} BAR0 {:#x?}", name, dev, addr);
                let addr = if addr == 0 { E1000_BASE as u64 } else { addr };
                let _irq = unsafe { enable(dev.loc, addr) };
            }
        }
        _ => {}
    }
}

/// Enable the pci device and its interrupt
/// Return assigned MSI interrupt number when applicable
unsafe fn enable(loc: Location, paddr: u64) -> Option<usize> {
    let ops = &PortOpsImpl;
    let am = PCI_ACCESS;

    if paddr != 0 {
        // reveal PCI regs by setting paddr
        let bar0_raw = am.read32(ops, loc, BAR0);
        am.write32(ops, loc, BAR0, (paddr & !0xfff) as u32); //Only for 32-bit decoding
        debug!(
            "BAR0 set from {:#x} to {:#x}",
            bar0_raw,
            am.read32(ops, loc, BAR0)
        );
    }

    // 23 and lower are used
    static mut MSI_IRQ: u32 = 23;

    let _orig = am.read16(ops, loc, PCI_COMMAND);
    // IO Space | MEM Space | Bus Mastering | Special Cycles | PCI Interrupt Disable
    // am.write32(ops, loc, PCI_COMMAND, (orig | 0x40f) as u32);

    // find MSI cap
    let mut msi_found = false;
    let mut cap_ptr = am.read8(ops, loc, PCI_CAP_PTR) as u16;
    let mut assigned_irq = None;
    while cap_ptr > 0 {
        let cap_id = am.read8(ops, loc, cap_ptr);
        if cap_id == PCI_CAP_ID_MSI {
            let orig_ctrl = am.read32(ops, loc, cap_ptr + PCI_MSI_CTRL_CAP);
            // The manual Volume 3 Chapter 10.11 Message Signalled Interrupts
            // 0 is (usually) the apic id of the bsp.
            //am.write32(ops, loc, cap_ptr + PCI_MSI_ADDR, 0xfee00000 | (0 << 12));
            am.write32(ops, loc, cap_ptr + PCI_MSI_ADDR, 0xfee00000);
            MSI_IRQ += 1;
            let irq = MSI_IRQ;
            assigned_irq = Some(irq as usize);
            // we offset all our irq numbers by 32
            if (orig_ctrl >> 16) & (1 << 7) != 0 {
                // 64bit
                am.write32(ops, loc, cap_ptr + PCI_MSI_DATA_64, irq + 32);
            } else {
                // 32bit
                am.write32(ops, loc, cap_ptr + PCI_MSI_DATA_32, irq + 32);
            }

            // enable MSI interrupt, assuming 64bit for now
            am.write32(ops, loc, cap_ptr + PCI_MSI_CTRL_CAP, orig_ctrl | 0x10000);
            debug!(
                "MSI control {:#b}, enabling MSI interrupt {}",
                orig_ctrl >> 16,
                irq
            );
            msi_found = true;
        }
        debug!("PCI device has cap id {} at {:#X}", cap_id, cap_ptr);
        cap_ptr = am.read8(ops, loc, cap_ptr + 1) as u16;
    }

    if !msi_found {
        // am.write16(ops, loc, PCI_COMMAND, (0x2) as u16);
        am.write16(ops, loc, PCI_COMMAND, 0x6);
        am.write32(ops, loc, PCI_INTERRUPT_LINE, 33);
        debug!("MSI not found, using PCI interrupt");
    }

    debug!("pci device enable done");
    assigned_irq
}

pub const PCI_COMMAND: u16 = 0x04;
pub const BAR0: u16 = 0x10;
pub const PCI_CAP_PTR: u16 = 0x34;
pub const PCI_INTERRUPT_LINE: u16 = 0x3c;
pub const PCI_INTERRUPT_PIN: u16 = 0x3d;
pub const PCI_COMMAND_INTX_DISABLE:u16 = 0x400;

pub const PCI_MSI_CTRL_CAP: u16 = 0x00;
pub const PCI_MSI_ADDR: u16 = 0x04;
pub const PCI_MSI_UPPER_ADDR: u16 = 0x08;
pub const PCI_MSI_DATA_32: u16 = 0x08;
pub const PCI_MSI_DATA_64: u16 = 0x0C;

pub const PCI_CAP_ID_MSI: u8 = 0x05;

pub fn phys_to_virt(paddr: usize) -> usize {
    paddr
}
pub fn virt_to_phys(vaddr: usize) -> usize {
    vaddr
}

#[inline(always)]
pub fn writev<T>(addr: usize, content: T) {
    let cell = (addr) as *mut T;
    unsafe {
        write_volatile(cell, content);
    }
}
#[inline(always)]
pub fn readv<T>(addr: usize) -> T {
    let cell = (addr) as *const T;
    unsafe { read_volatile(cell) }
}

/// riscv64 qemu
pub const PCI_BASE: usize = 0x30000000;
pub const E1000_BASE: usize = 0x40000000;
pub const PCI_ACCESS: CSpaceAccessMethod = CSpaceAccessMethod::MemoryMapped(PCI_BASE as *mut u8);

pub struct PortOpsImpl;
impl PortOps for PortOpsImpl {
    unsafe fn read8(&self, port: u16) -> u8 {
        readv(PCI_BASE + port as usize)
    }
    unsafe fn read16(&self, port: u16) -> u16 {
        readv(PCI_BASE + port as usize)
    }
    unsafe fn read32(&self, port: u32) -> u32 {
        readv(PCI_BASE + port as usize)
    }
    unsafe fn write8(&self, port: u16, val: u8) {
        writev(PCI_BASE + port as usize, val);
    }
    unsafe fn write16(&self, port: u16, val: u16) {
        writev(PCI_BASE + port as usize, val);
    }
    unsafe fn write32(&self, port: u32, val: u32) {
        writev(PCI_BASE + port as usize, val);
    }
}