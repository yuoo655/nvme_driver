use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use core::ops::Deref;
use core::ptr::{read_volatile, write_volatile};
use core::marker::PhantomData;

use crate::irq::IrqController;
use crate::dma::DmaAllocator;
use super::nvme_defs::*;
use super::nvme_queue::*;
use lock::Mutex;
use lock::MutexGuard;


pub struct NvmeInterface<D: DmaAllocator, I: IrqController> {

    irq_data: PhantomData<I>,

    admin_queue: Arc<Mutex<NvmeQueue<D>>>,

    io_queues: Vec<Arc<Mutex<NvmeQueue<D>>>>,

    bar: usize,

    irq: usize,
}


impl<D: DmaAllocator, I: IrqController> NvmeInterface<D, I> {
    pub fn new(bar: usize) -> Self {

        let admin_queue = Arc::new(Mutex::new(NvmeQueue::new(0, 0)));

        let io_queues = vec![Arc::new(Mutex::new(NvmeQueue::new(1, 0x8)))];

        let mut interface = NvmeInterface {
            irq_data: PhantomData,
            admin_queue,
            io_queues,
            bar,
            irq: 33,
        };

        interface.init();

        interface
    }

    // config admin queue ,io queue
    pub fn init(&mut self) {

        self.nvme_configure_admin_queue();

        self.nvme_alloc_io_queue();
    }
}



impl<D: DmaAllocator, I: IrqController> NvmeInterface<D, I> {
    pub fn submit_sync_command(&mut self, cmd: NvmeCommonCommand){        
        let mut admin_queue = self.admin_queue.lock();

        let dbs = self.bar + NVME_REG_DBS;

        let sq_tail = admin_queue.sq_tail;
        let cq_head = admin_queue.cq_head;

        admin_queue.sq[sq_tail].write(cmd);
        admin_queue.sq_tail += 1;

        let admin_q_db = dbs + admin_queue.db_offset;
        unsafe { write_volatile(admin_q_db as *mut u32, (sq_tail+1) as u32) }

        loop {
            let status = admin_queue.cq[cq_head].read();
            if status.status != 0 {
                unsafe { write_volatile((admin_q_db + 0x4) as *mut u32, (cq_head+1) as u32) }
                admin_queue.cq_head += 1;
                break;
            }
        }
    }


    pub fn nvme_configure_admin_queue(&mut self) {
        let admin_queue = self.admin_queue.lock();

        let bar = self.bar;
        let dbs = bar + NVME_REG_DBS;

        let sq_dma_pa = admin_queue.sq_pa as u32;
        let cq_dma_pa = admin_queue.cq_pa as u32;

        // sq depth
        let aqa_low_16 = 31_u16;
        // cq depth
        let aqa_high_16 = 31_u16;
        let aqa = (aqa_high_16 as u32) << 16 | aqa_low_16 as u32;
        let aqa_address = bar + NVME_REG_AQA;

        // 将admin queue配置信息(sq/cq depth)写入nvme设备寄存器AQA(Admin Queue Attributes)
        unsafe {
            write_volatile(aqa_address as *mut u32, aqa);
        }

        // 将admin queue的sq dma物理地址写入nvme设备上的寄存器ASQ(Admin SQ Base Address)
        let asq_address = bar + NVME_REG_ASQ;
        unsafe {
            write_volatile(asq_address as *mut u32, sq_dma_pa);
        }

        // 将admin queue的cq dma物理地址写入nvme设备上的寄存器ACQ(Admin CQ Base Address)
        let acq_address = bar + NVME_REG_ACQ;
        unsafe {
            write_volatile(acq_address as *mut u32, cq_dma_pa);
        }

        // enable ctrl
        let mut ctrl_config = NVME_CC_ENABLE | NVME_CC_CSS_NVM;
        ctrl_config |= 0 << NVME_CC_MPS_SHIFT;
        ctrl_config |= NVME_CC_ARB_RR | NVME_CC_SHN_NONE;
        ctrl_config |= NVME_CC_IOSQES | NVME_CC_IOCQES;

        unsafe { write_volatile((bar + NVME_REG_CC) as *mut u32, ctrl_config) }

        let _dev_status = unsafe { read_volatile((bar + NVME_REG_CSTS) as *mut u32) };
        // warn!("nvme status {}", _dev_status);

    }

    pub fn nvme_alloc_io_queue(&mut self) {
        let cq_pa = self.io_queues[0].lock().cq_pa;
        let sq_pa = self.io_queues[0].lock().sq_pa;

        // nvme_set_queue_count
        let mut cmd = NvmeCommonCommand::new();
        cmd.opcode = 0x09;
        cmd.command_id = 0x2;
        cmd.nsid = 0;
        cmd.cdw10 = 0x7;
        self.submit_sync_command(cmd);
        

        //nvme create cq
        let mut cmd = NvmeCreateCq::new();
        cmd.opcode = 0x05;
        cmd.command_id = 0x3;
        cmd.nsid = 0;
        cmd.prp1 = cq_pa as u64;
        cmd.cqid = 1;
        cmd.qsize = 1023;
        cmd.cq_flags = NVME_QUEUE_PHYS_CONTIG | NVME_CQ_IRQ_ENABLED;
        let common_cmd = unsafe { core::mem::transmute(cmd) };
        self.submit_sync_command(common_cmd);


        // nvme create sq
        let mut cmd = NvmeCreateSq::new();
        cmd.opcode = 0x01;
        cmd.command_id = 0x4;
        cmd.nsid = 0;
        cmd.prp1 = sq_pa as u64;
        cmd.sqid = 1;
        cmd.qsize = 1023;
        cmd.sq_flags = 0x1;
        cmd.cqid = 0x1;
        let common_cmd = unsafe { core::mem::transmute(cmd) };
        self.submit_sync_command(common_cmd);
    }
}


impl<D: DmaAllocator, I: IrqController> NvmeInterface<D, I> {

    // 每个NVMe命令中有两个域：PRP1和PRP2，Host就是通过这两个域告诉SSD数据在内存中的位置或者数据需要写入的地址
    // 首先对prp1进行读写，如果数据还没完，就看数据量是不是在一个page内，在的话，只需要读写prp2内存地址就可以了，数据量大于1个page，就需要读出prp list

    // 由于只读一块, 小于一页, 所以只需要prp1
    // prp1 = dma_addr
    // prp2 = 0
    
    // prp设置
    // uboot中对应实现 nvme_setup_prps
    // linux中对应实现 nvme_pci_setup_prps
    // SLBA = start logical block address
    // 1 SLBA = 512B
    // length = 0 = 512B
    pub fn read_block(&self, block_id: usize, read_buf: &mut [u8]){
        // 这里dma addr 就是buffer的地址
        let ptr = read_buf.as_mut_ptr();
        let addr = D::virt_to_phys(ptr as usize);
        
        // build nvme read command
        let mut cmd = NvmeRWCommand::new_read_command();
        cmd.nsid = 1;
        cmd.prp1 = addr as u64;
        cmd.command_id = 101;
        cmd.length = 0;
        cmd.control = 0x8000;
        cmd.dsmgmt = 0x7;
        cmd.slba = block_id as u64;
        
        //transfer to common command
        let common_cmd = unsafe { core::mem::transmute(cmd) };
        

        let mut io_queue = self.io_queues[0].lock();
        self.sync_command( &mut io_queue, common_cmd);
        self.nvme_poll_cq(&mut io_queue);
    }

    // prp1 = write_buf physical address
    // prp2 = 0
    // SLBA = start logical block address
    // length = 1 = 512B
    pub fn write_block(&self, block_id: usize, write_buf: &[u8]){
        let ptr = write_buf.as_ptr();
        let addr = D::virt_to_phys(ptr as usize);

        // build nvme write command
        let mut cmd = NvmeRWCommand::new_write_command();
        cmd.nsid = 1;
        cmd.prp1 = addr as u64;
        cmd.length = 0;
        cmd.command_id = 100;
        cmd.slba = block_id as u64;
        cmd.control = 0;
        cmd.dsmgmt = 0;

        // transmute to common command
        let common_cmd = unsafe { core::mem::transmute(cmd) };

        let mut io_queue = self.io_queues[0].lock();
        self.sync_command( &mut io_queue, common_cmd);
        self.nvme_poll_cq(&mut io_queue);
    }
}



impl<D: DmaAllocator, I: IrqController> NvmeInterface<D, I> {


    pub fn nvme_poll_irqdisable(&self){

        I::disable_irq(self.irq);

        I::enable_irq(self.irq);
    }
    

    pub fn sync_command(&self, nvmeq: &mut MutexGuard<NvmeQueue<D>>, cmd: NvmeCommonCommand){
        let sq_tail = nvmeq.sq_tail;
        nvmeq.sq[sq_tail].write(cmd);
        
        if (nvmeq.sq_tail + 1) == nvmeq.q_depth{
            nvmeq.sq_tail = 0;
        }else{
            nvmeq.sq_tail += 1;
        }

        self.nvme_write_sq_db(nvmeq, true);
    }

    pub fn nvme_poll_cq(&self, nvmeq: &mut MutexGuard<NvmeQueue<D>>){

        while self.nvme_cqe_pending(nvmeq){
            self.nvme_update_cq_head(nvmeq);
            self.nvme_ring_cq_doorbell(nvmeq);
        }
    }

    pub fn nvme_cqe_pending(&self, nvmeq:&mut  MutexGuard<NvmeQueue<D>>)-> bool{

        let cq_head = nvmeq.cq_head;
        let cqe = nvmeq.cq[cq_head].read();
        if (cqe.status & 1) == (nvmeq.cq_phase as u16) {
            return true;
        }else{
            return false;
        }
    }

    pub fn nvme_ring_cq_doorbell(&self, nvmeq: &mut MutexGuard<NvmeQueue<D>>){
        let cq_head = nvmeq.cq_head;
        let q_db = self.bar + NVME_REG_DBS + nvmeq.db_offset;
        unsafe { write_volatile((q_db + 0x4) as *mut u32, cq_head as u32) }
    }

    pub fn nvme_write_sq_db(&self, nvmeq: &mut MutexGuard<NvmeQueue<D>>, write_sq: bool) {
    
	    if !write_sq {
            let mut next_tail = nvmeq.sq_tail + 1;
	    	if next_tail == nvmeq.q_depth{
	    		next_tail = 0;
            }
	    	if next_tail != nvmeq.last_sq_tail{
	    		return;
            }
	    }

        let db = self.bar + NVME_REG_DBS + nvmeq.db_offset;
        unsafe { write_volatile(db as *mut u32, nvmeq.sq_tail as u32) }
        nvmeq.last_sq_tail = nvmeq.sq_tail;
    }

    pub fn nvme_update_cq_head(&self, nvmeq: &mut MutexGuard<NvmeQueue<D>>) {
        let next_head = nvmeq.cq_head + 1;
        if next_head == nvmeq.q_depth{
            nvmeq.cq_head = 0;
            nvmeq.cq_phase ^= 1;
        }else{
            nvmeq.cq_head = next_head;
        }
    }
}