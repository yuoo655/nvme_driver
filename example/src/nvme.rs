use core::sync::atomic::*;

use nvme_driver::DmaAllocator;
use nvme_driver::IrqController;
use nvme_driver::IoMapper;
use nvme_driver::NvmeDriver;

use lazy_static::lazy_static;

lazy_static! {
    static ref DMA_PADDR: AtomicUsize = AtomicUsize::new(0x81000000 as usize);
}


pub struct DmaProvider;


impl DmaAllocator for DmaProvider{

    fn dma_alloc(size: usize, dma_handle: &mut u64) -> usize{
        let paddr = DMA_PADDR.fetch_add(size, Ordering::SeqCst);
        *dma_handle = paddr as u64;
        paddr
    }

    fn dma_dealloc(cpu_addr: *mut (), dma_handle: u64, size: usize){

    }

}

pub struct IoMapperProvider;

impl IoMapper for IoMapperProvider{

    fn ioremap(start: usize, size: usize) -> usize{

        start
    }

    fn iounmap(start: usize){

    }
}


pub struct IrqProvider;


impl IrqController for IrqProvider{

    fn request_irq(irq_num: usize) {}

    fn enable_irq(irq_num: usize) {}

    fn disable_irq(irq_num: usize) {}

}

pub fn nvme_test() ->!{
    config_pci();
    let nvme = NvmeDriver::<DmaProvider, IrqProvider, IoMapperProvider>::new(0x40000000);
    nvme.init();

    

    // for i in 0..100000{
    //     let mut read_buf = [0u8; 512];
    //     let buff = [i as u8;512];
    //     let write_buf:&[u8] = &[i as u8;512];
    //     nvme.write_block(i, &write_buf);
    //     nvme.read_block(i, &mut read_buf);
    //     // println!("{:?}", i);
    //     assert_eq!(read_buf, buff);
    // }

    panic!("Unreachable in rust_main!");
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