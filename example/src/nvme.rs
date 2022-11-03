use core::sync::atomic::*;


use pci_device_drivers::NvmeInterface;
use pci_device_drivers::DmaAllocator;
use pci_device_drivers::IrqController;


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

    fn dma_dealloc(addr: usize) -> usize{
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
        write_volatile(ptr, 0x4);
    }

    let ptr = 0x30008010 as *mut u32;
    unsafe {
        write_volatile(ptr, 0x40000000);
    }

    let ptr = 0x30008004 as *mut u32;
    unsafe {
        write_volatile(ptr, 0x100006);
    }

    // let ptr = 0x3000803c as *mut u32;

    // unsafe {
    //     write_volatile(ptr, 0x21);
    // }

}




pub struct IrqProvider;

impl IrqController for IrqProvider{
    fn enable_irq(irq: usize){
        
    }

    fn disable_irq(irq: usize){
        
    }

    fn register_irq(irq: usize, handler: fn(usize)){
        
    }

    fn unregister_irq(irq: usize){
        
    }
}

pub fn nvme_test() ->!{
    config_pci();
    let nvme = NvmeInterface::<DmaProvider, IrqProvider>::new(0x40000000);

    let buf1:&[u8] = &[1u8;512];
    let _r = nvme.write_block(0, &buf1);
    let mut read_buf = [0u8; 512];
    let _r = nvme.read_block(0, &mut read_buf);
    println!("read_buf: {:?}", read_buf);

    let buf2:&[u8] = &[2u8;512];
    let _r = nvme.write_block(1, &buf2);
    let mut read_buf = [0u8; 512];
    let _r = nvme.read_block(1, &mut read_buf);
    println!("read_buf: {:?}", read_buf);

    panic!("Unreachable in rust_main!");
}
