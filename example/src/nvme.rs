use core::sync::atomic::*;

use nvme_driver::*;


use lazy_static::lazy_static;

lazy_static! {
    static ref DMA_PADDR: AtomicUsize = AtomicUsize::new(0x81000000 as usize);
}


pub struct NvmeTraitsImpl;


impl NvmeTraitsImpl for NvmeTraits{

    fn dma_alloc(size: usize, dma_handle: &mut u64) -> usize{
        let paddr = DMA_PADDR.fetch_add(size, Ordering::SeqCst);
        *dma_handle = paddr as u64;
        paddr
    }

    fn dma_dealloc(cpu_addr: *mut (), dma_handle: u64, size: usize){

    }

    fn ioremap(start: usize, size: usize) -> usize{

        start
    }

    fn iounmap(start: usize){

    }

    fn writew(val: u16, offset: usize) {
        unsafe {
            write_volatile(self.ptr as *mut u16, val);
        }
    }

    fn readl(offset: usize) -> u32 {
        let val = unsafe { read_volatile((self.ptr + offset) as *mut u32) };
        val
    }

    fn writel(val: u32, offset: usize) {
        unsafe {
            write_volatile((self.ptr + offset) as *mut u32, val);
        }
    }

    fn readq(offset: usize) -> u64 {
        let val = unsafe { read_volatile((self.ptr + offset) as *mut u64) };
        val
    }
    fn writeq(val: u64, offset: usize) {
        unsafe {
            write_volatile((self.ptr + offset) as *mut u64, val);
        }
    }

}



pub fn nvme_test() ->!{
    config_pci();

    // let bar = IoMem::<8192, NvmeTraitsImpl>::new(0x40000000 as usize, 8192);

    // let nvme_data = NvmeData{
    //     queues: nvme_queues,
    //     bar: bar,
    //     db_stride: 0,
    // };

    // let nvme_dev = NvmeTraitsImpl::new(0);
    // let admin_queue = NvmeQueue::<NvmeTraitsImpl, usize>::new(
    //         nvme_dev,
    //         0x0,
    //         nvme_data.clone(),
    //         0,
    //         (NVME_QUEUE_DEPTH ) as u16,
    //         0,
    //         false,
    //         0,
    //     );


    // let io_queue = NvmeQueue::<NvmeTraitsImpl, *mut bindings::device>::new(
    //         nvme_dev,
    //         0x0,
    //         nvme_data.clone(),
    //         1,
    //         (NVME_QUEUE_DEPTH)as u16,
    //         1,
    //         false,
    //         0x4,
    //     );
    // config_admin_queue()












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