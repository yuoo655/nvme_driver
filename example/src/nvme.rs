use core::sync::atomic::*;


use nvme_driver::NvmeInterface;
use nvme_driver::DmaAllocator;
use nvme_driver::IrqController;


use lazy_static::lazy_static;

lazy_static! {
    static ref DMA_PADDR: AtomicUsize = AtomicUsize::new(0x81000000 as usize);
}

pub struct DmaProvider;

impl DmaAllocator for DmaProvider{

    fn dma_alloc(size: usize) -> usize{
        let paddr = DMA_PADDR.fetch_add(size, Ordering::SeqCst);
        paddr
    }

    fn dma_dealloc(addr: usize, size: usize) -> usize{
        0
    }

    fn phys_to_virt(phys: usize) -> usize{
        phys
    }

    fn virt_to_phys(virt: usize) -> usize{
        virt
    }
}

use core::ptr::write_volatile;

pub fn config_pci(){
    let ptr = 0x30008010 as *mut u32;
    unsafe {
        write_volatile(ptr, 0xffffffff);
    }

    let ptr = 0x30008010 as *mut u32;
    unsafe {
        write_volatile(ptr, 0x4);
    }

    let ptr = 0x30008018 as *mut u32;
    unsafe {
        write_volatile(ptr, 0xffffffff);
    }

    let ptr = 0x30008018 as *mut u32;
    unsafe {
        write_volatile(ptr, 0);
    }

    let ptr = 0x3000801C as *mut u32;
    unsafe {
        write_volatile(ptr, 0xffffffff);
    }

    let ptr = 0x3000801C as *mut u32;
    unsafe {
        write_volatile(ptr, 0);
    }


    let ptr = 0x30008020 as *mut u32;
    unsafe {
        write_volatile(ptr, 0xffffffff);
    }

    let ptr = 0x30008020 as *mut u32;
    unsafe {
        write_volatile(ptr, 0);
    }

    let ptr = 0x30008024 as *mut u32;
    unsafe {
        write_volatile(ptr, 0xffffffff);
    }

    let ptr = 0x30008024 as *mut u32;
    unsafe {
        write_volatile(ptr, 0);
    }
    let ptr = 0x30008010 as *mut u32;
    unsafe {
        write_volatile(ptr, 0x40000000);
    }

    let ptr = 0x30008004 as *mut u32;
    unsafe {
        write_volatile(ptr, 0x100006);
    }

    let ptr = 0x3000803c as *mut u32;

    unsafe {
        write_volatile(ptr, 0x21);
    }

}


pub struct IrqProvider;

impl IrqController for IrqProvider{
    fn enable_irq(irq: usize){
        
    }

    fn disable_irq(irq: usize){
        
    }
}


pub fn nvme_test() ->!{
    config_pci();
    let nvme = NvmeInterface::<DmaProvider, IrqProvider>::new(0x40000000);

    
    // for i in 0..100000{
        // }
        
    let buff = [0x11 as u8;512];
    let mut read_buf = [0u8; 512];
    for i in 0..100000{
        let write_buf:&[u8] = &[0x11 as u8;512];
        nvme.write_block(i, &write_buf);
        nvme.read_block(i, &mut read_buf);
        // println!("{:?}", i);
        assert_eq!(read_buf, buff);
    }

    panic!("Unreachable in rust_main!");
}



// cfg_if::cfg_if! {
//     if #[cfg(target_arch = "riscv64")] {
//         macro_rules! memory_fence {
//             ($pred:ident, $succ:ident) => {
//                 unsafe { core::arch::asm!(concat!("fence ", stringify!($pred), ',', stringify!($succ))) }
//             };
//         }
//     } else {
//         macro_rules! memory_fence {
//             ($pred:ident, $succ:ident) => {};
//         }
//     }
// }

macro_rules! memory_fence {
    ($pred:ident, $succ:ident) => {
        unsafe { core::arch::asm!(concat!("fence ", stringify!($pred), ',', stringify!($succ))) }
    };
}

#[inline(always)]
pub fn dma_rmb() {
    memory_fence!(r, r);
}

#[inline(always)]
pub fn dma_wmb() {
    memory_fence!(w, w);
}
