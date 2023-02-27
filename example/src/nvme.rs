use core::sync::atomic::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::write_volatile;
use core::ptr::read_volatile;
extern crate alloc;

use nvme_driver::*;


use lock::Mutex;
use lock::MutexGuard;

use lazy_static::lazy_static;

lazy_static! {
    static ref DMA_PADDR: AtomicUsize = AtomicUsize::new(0x81000000 as usize);
}


pub struct NvmeTraitsImpl;


impl NvmeTraits for NvmeTraitsImpl{

    fn dma_alloc(&self, size: usize, dma_handle: &mut u64) -> usize{
        let paddr = DMA_PADDR.fetch_add(size, Ordering::SeqCst);
        *dma_handle = paddr as u64;
        paddr
    }

    fn dma_dealloc(&self, cpu_addr: *mut (), dma_handle: u64, size: usize){

    }

    fn ioremap(start: usize, size: usize) -> usize{

        start
    }

    fn iounmap(start: usize){

    }

    fn writew(val: u16, offset: usize) {
        unsafe {
            write_volatile(offset as *mut u16, val);
        }
    }

    fn readl(offset: usize) -> u32 {
        let val = unsafe { read_volatile((offset) as *mut u32) };
        val
    }

    fn writel(val: u32, offset: usize) {
        unsafe {
            write_volatile(( offset) as *mut u32, val);
        }
    }

    fn readq(offset: usize) -> u64 {
        let val = unsafe { read_volatile((offset) as *mut u64) };
        val
    }
    fn writeq(val: u64, offset: usize) {
        unsafe {
            write_volatile((offset) as *mut u64, val);
        }
    }

}




// pub struct NvmeData<A, T>
// where
//     A: NvmeTraits + 'static,
// {
//     pub queues: NvmeQueues<A, T>,
//     pub db_stride: usize,
//     pub bar: IoMem<8192, A>,
// }


// pub struct NvmeCommonData<A>
// where
//     A: NvmeTraits + 'static,
// {
//     pub bar: IoMem<8192, A>,
// }

pub fn nvme_test() ->!{
    config_pci();

    let bar = IoMem::<8192, NvmeTraitsImpl>::new(0x40000000 as usize, 8192);

    let nvme_common_data = Arc::new(NvmeCommonData::<NvmeTraitsImpl>{
        bar: bar,
    });

    let nvme_queues = NvmeQueues::<NvmeTraitsImpl, usize>::new();

    let nvme_data = Arc::new(NvmeData{
        queues: Mutex::new(nvme_queues),
        bar: nvme_common_data,
        db_stride: 0,
    });

    let nvme_dev = NvmeTraitsImpl;

    


    let admin_queue = Arc::new(NvmeQueue::<NvmeTraitsImpl, usize>::new(
            nvme_dev,
            0x0,
            nvme_data.bar.clone(),
            0,
            (1024 ) as u16,
            0,
            false,
            0,
    ));

    let nvme_dev = NvmeTraitsImpl;

    let io_queue = Arc::new(NvmeQueue::<NvmeTraitsImpl, usize>::new(
            nvme_dev,
            0x0,
            nvme_data.bar.clone(),
            1,
            (1024)as u16,
            1,
            false,
            0x4,
    ));

    let bar = &nvme_data.bar.clone().bar;

    config_admin_queue(bar, &admin_queue);

    nvme_data.queues.lock().admin_queue = Some(admin_queue.clone());
    nvme_data.queues.lock().io_queues.push(io_queue.clone());
    set_queue_count(1, nvme_data.clone());
    
    alloc_completion_queue(nvme_data.clone(), &io_queue);
    alloc_submission_queue(nvme_data.clone(), &io_queue);

    // for i in 0..10{

    //     unsafe{
    //         let buff = [i as u8;512];
    //         let mut read_buf_addr = NvmeTraitsImpl.dma_alloc(4096, &mut 0);
            
    //         let mut read_buf = core::slice::from_raw_parts_mut(read_buf_addr as *mut u8, 512);

            
    //         let mut write_buf_addr = NvmeTraitsImpl.dma_alloc(4096, &mut 0);
    //         let mut write_buf = core::slice::from_raw_parts_mut(write_buf_addr as *mut u8, 512);
    //         write_buf.copy_from_slice(&buff);


    //         println!("read_buf_addr: {:x}", read_buf_addr);
    //         println!("write_buf_addr: {:x}", write_buf_addr);
    //         nvme_data.write_block(i, &write_buf);

    //         nvme_data.read_block(i, &mut read_buf);
    //         assert_eq!(read_buf, buff);
    //     }

    //     println!("i: {}", i);
    // }






    for i in 0..100{
        let mut read_buf = [0u8; 512];
        let buff = [i as u8;512];
        let write_buf:&[u8;512] = &[i as u8;512];
        nvme_data.write_block(i, &write_buf);
        nvme_data.read_block(i, &mut read_buf);
        assert_eq!(read_buf, buff);
    }







 
    panic!("Unreachable in rust_main!");
}








pub fn submit_sync_command<A: NvmeTraits, T>(nvme_dev: Arc<NvmeData<A, T>>,mut cmd: NvmeCommand){

    println!("submit_sync_command");

    // let mut admin_queue = nvme_dev.queues.lock().admin_queue.unwrap();
    // // self.send_command(&mut admin_queue, cmd);
    // // self.nvme_poll_cq(&mut admin_queue);

    // admin_queue.submit_command(&mut cmd, true);
    // admin_queue.nvme_poll_cq();

}








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