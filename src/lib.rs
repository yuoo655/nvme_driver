#![no_std]
#![feature(associated_type_defaults)]
#![feature(generic_associated_types)]
#![feature(associated_type_bounds)]
#![feature(linkage)]
extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use lock::Mutex;
use lock::MutexGuard;

mod nvme;

pub use nvme::*;



#[linkage = "weak"]
pub struct NvmeData<A, T>
where
    A: NvmeTraits + 'static,
{
    pub queues: Mutex<NvmeQueues<A, T>>,
    pub db_stride: usize,
    pub bar: Arc<NvmeCommonData<A>>
}


impl <A, T: 'static> NvmeData<A, T>
where
    A: NvmeTraits + 'static,
{

    pub fn read_block(&self, block_id: usize, read_buf: &mut [u8; 512]) {

        let io_queue = &self.queues.lock().io_queues[0];
        let mut cmd = NvmeCommand {
        rw: NvmeRw {
                opcode: 0x2 as u8,
                command_id: 100,
                nsid: 1.into(),
                slba: (block_id as u64).into(),
                length: (0 as u16).into(),
                ..NvmeRw::default()
            },
        };

        let read_buffer_addr  = read_buf as *mut [u8; 512] as usize;
    
        // let addr = ptr as usize;

        cmd.rw.prp1 = (read_buffer_addr as u64).into();


        io_queue.submit_command(&cmd, true);
        io_queue.nvme_poll_cq();


        // let bar = &io_queue.device_data.bar;

        // let mut head = io_queue.cq_head.load(Ordering::Relaxed);
        // let mut phase = io_queue.cq_phase.load(Ordering::Relaxed);

        // loop {
        //     let cqe = io_queue.cq.read_volatile(head.into()).unwrap();
        //     if cqe.status.into() & 1 != phase {
        //         break;
        //     }
        //     let cqe = io_queue.cq.read_volatile(head.into()).unwrap();
        //     head += 1;
        //     if head == io_queue.q_depth {
        //         head = 0;
        //         phase ^= 1;
        //     }            
        // }

        // io_queue.cq_head.store(head, Ordering::Relaxed);
        // io_queue.cq_phase.store(phase, Ordering::Relaxed);

        // bar.writel(head.into(), io_queue.db_offset + 0x4);
    }

    // prp1 = write_buf physical address
    // prp2 = 0
    // SLBA = start logical block address
    // length = 0 = 512B
    pub fn write_block(&self, block_id: usize, write_buf: &[u8; 512]) {

        let io_queue = &self.queues.lock().io_queues[0];
        let mut cmd = NvmeCommand {
        rw: NvmeRw {
                opcode: 0x1 as u8,
                command_id: 101,
                nsid: 1.into(),
                slba: (block_id as u64).into(),
                length: (0 as u16).into(),
                ..NvmeRw::default()
            },
        };


        let write_buffer_addr  = write_buf as *const [u8; 512] as usize;
    

        cmd.rw.prp1 = (write_buffer_addr as u64).into();


        io_queue.submit_command(&cmd, true);

        io_queue.nvme_poll_cq();

        // let bar = &io_queue.device_data.bar;

        // let mut head = io_queue.cq_head.load(Ordering::Relaxed);
        // let mut phase = io_queue.cq_phase.load(Ordering::Relaxed);

        // loop {
        //     let cqe = io_queue.cq.read_volatile(head.into()).unwrap();
        //     if cqe.status.into() & 1 != phase {
        //         break;
        //     }
        //     let cqe = io_queue.cq.read_volatile(head.into()).unwrap();
        //     head += 1;
        //     if head == io_queue.q_depth {
        //         head = 0;
        //         phase ^= 1;
        //     }            
        // }

        // io_queue.cq_head.store(head, Ordering::Relaxed);
        // io_queue.cq_phase.store(phase, Ordering::Relaxed);

        // bar.writel(head.into(), io_queue.db_offset + 0x4);
    }


}
    

use core::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering};

#[linkage = "weak"]
pub fn submit_sync_command<A: NvmeTraits, T: 'static>(nvme_dev: Arc<NvmeData<A, T>>,mut cmd: NvmeCommand){

    let queues = &nvme_dev.queues;
    let queues = queues.lock();
    let admin_queue = queues.admin_queue.as_ref().unwrap();

    admin_queue.submit_command(&mut cmd, true);
    admin_queue.nvme_poll_cq();

    
}




pub fn alloc_completion_queue<A: NvmeTraits, T: 'static>(
    nvme_dev: Arc<NvmeData<A, T>>,
    queue: &NvmeQueue<A, T>
) {

    let mut flags = NVME_QUEUE_PHYS_CONTIG;
    // if !queue.polled {
    //     flags |= NVME_CQ_IRQ_ENABLED;
    // }

    let cmd = NvmeCommand {
        create_cq: NvmeCreateCq {
            opcode: NvmeAdminOpcode::create_cq as _,
            prp1: queue.cq.dma_handle.into(),
            cqid: (1).into(),
            qsize: (queue.q_depth - 1).into(),
            cq_flags: flags.into(),
            // irq_vector: queue.cq_vector.into(),
            rsvd1: [0; 5].into(),
            ..NvmeCreateCq::default()
        },
    };
    submit_sync_command(nvme_dev, cmd);
}

pub fn alloc_submission_queue<A: NvmeTraits, T: 'static>(
    nvme_dev: Arc<NvmeData<A, T>>,
    queue: &NvmeQueue<A, T>
) {
    let cmd = NvmeCommand {
        create_sq: NvmeCreateSq {
            opcode: NvmeAdminOpcode::create_sq as _,
            prp1: queue.sq.dma_handle.into(),
            sqid: queue.qid.into(),
            qsize: (queue.q_depth - 1).into(),
            sq_flags: (1).into(),
            cqid: queue.qid.into(),
            rsvd1: [0; 5].into(),
            ..NvmeCreateSq::default()
        },
    };

    submit_sync_command(nvme_dev, cmd);
}

pub fn set_queue_count<A: NvmeTraits, T: 'static>(
    count: u32,
    nvme_dev: Arc<NvmeData<A, T>>,
) {

    let q_count = count;
    set_features(NVME_FEAT_NUM_QUEUES, q_count, 0, nvme_dev);
}

pub fn set_features<A: NvmeTraits, T: 'static>(
    fid: u32,
    dword11: u32,
    dma_addr: u64,
    nvme_dev: Arc<NvmeData<A, T>>,
) {

    submit_sync_command(
        nvme_dev,
        NvmeCommand {
            features: NvmeFeatures {
                opcode: NvmeAdminOpcode::set_features as _,
                // prp1: dma_addr.into(),
                fid: fid.into(),
                dword11: dword11.into(),
                ..NvmeFeatures::default()
            },
        },
    );
    
}

pub fn identify<A: NvmeTraits, T: 'static>(
    nvme_dev: Arc<NvmeData<A, T>>,
    nsid: u32,
    cns: u32,
    dma_addr: u64,
) {
    
    submit_sync_command(
        nvme_dev,
        NvmeCommand {
            identify: NvmeIdentify {
                opcode: NvmeAdminOpcode::identify as _,
                nsid: nsid.into(),
                prp1: dma_addr.into(),
                cns: cns.into(),
                ..NvmeIdentify::default()
            },
        },
    )
}

pub fn get_features<A: NvmeTraits, T: 'static>(
    nvme_dev: Arc<NvmeData<A, T>>,
    fid: u32,
    nsid: u32,
    dma_addr: u64,
) {
    
    submit_sync_command(
        nvme_dev,
        NvmeCommand {
            features: NvmeFeatures {
                opcode: NvmeAdminOpcode::get_features as _,
                nsid: nsid.into(),
                prp1: dma_addr.into(),
                fid: fid.into(),
                ..NvmeFeatures::default()
            },
        },
    )
}


