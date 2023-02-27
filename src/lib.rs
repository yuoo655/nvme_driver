#![no_std]
#![feature(associated_type_defaults)]
#![feature(generic_associated_types)]
#![feature(associated_type_bounds)]
#![feature(linkage)]
extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use lock::mutex;

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


#[linkage = "weak"]
pub fn submit_sync_command<A: NvmeTraits, T>(nvme_dev: &Arc<NvmeData<A, T>>,mut cmd: NvmeCommand){

}




pub fn alloc_completion_queue<A: NvmeTraits, T>(
    nvme_dev: &Arc<NvmeData<A, T>>,
    queue: &NvmeQueue<A, T>
) {

    let mut flags = NVME_QUEUE_PHYS_CONTIG;
    if !queue.polled {
        flags |= NVME_CQ_IRQ_ENABLED;
    }

    let cmd = NvmeCommand {
        create_cq: NvmeCreateCq {
            opcode: NvmeAdminOpcode::create_cq as _,
            prp1: queue.cq.dma_handle.into(),
            cqid: queue.qid.into(),
            qsize: (queue.q_depth - 1).into(),
            cq_flags: flags.into(),
            irq_vector: queue.cq_vector.into(),
            ..NvmeCreateCq::default()
        },
    };
    submit_sync_command(nvme_dev, cmd);
}

pub fn alloc_submission_queue<A: NvmeTraits, T>(
    nvme_dev: &Arc<NvmeData<A, T>>,
    queue: &NvmeQueue<A, T>
) {
    let cmd = NvmeCommand {
        create_sq: NvmeCreateSq {
            opcode: NvmeAdminOpcode::create_sq as _,
            prp1: queue.sq.dma_handle.into(),
            sqid: queue.qid.into(),
            qsize: (queue.q_depth - 1).into(),
            sq_flags: (NVME_QUEUE_PHYS_CONTIG | NVME_SQ_PRIO_MEDIUM).into(),
            cqid: queue.qid.into(),
            ..NvmeCreateSq::default()
        },
    };

    submit_sync_command(nvme_dev, cmd);
}

pub fn set_queue_count<A: NvmeTraits, T>(
    count: u32,
    nvme_dev: &Arc<NvmeData<A, T>>,
) {

    let q_count = count;
    set_features(NVME_FEAT_NUM_QUEUES, q_count, 0, nvme_dev);
}

pub fn set_features<A: NvmeTraits, T>(
    fid: u32,
    dword11: u32,
    dma_addr: u64,
    nvme_dev: &Arc<NvmeData<A, T>>,
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

pub fn identify<A: NvmeTraits, T>(
    nvme_dev: &Arc<NvmeData<A, T>>,
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

pub fn get_features<A: NvmeTraits, T>(
    nvme_dev: &Arc<NvmeData<A, T>>,
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