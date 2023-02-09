use alloc::sync::Arc;
use alloc::vec::Vec;
use core::marker::PhantomData;

use super::nvme_defs::*;
use super::nvme_queue::*;
use crate::dma::DmaAllocator;
use crate::iomem::{IoMapper, IoMem};
use crate::irq::IrqController;


use lock::Mutex;

use log::info;

pub const NVME_QUEUE_DEPTH: usize = 1024;

use core::{
    format_args,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering},
};

#[repr(C)]
pub struct Data<T, U, V> {
    registrations: T,
    resources: U,
    general: V,
}

// pub type NvmeDeviceData<D, I, M> = Data<usize, NvmeResources<M>, NvmeQueues<D, I, M>>;

pub struct NvmeData<D: DmaAllocator, I: IrqController, M: IoMapper> {
    pub db_stride: usize,
    pub queues: Mutex<NvmeQueues<D, I, M>>,
}

struct NvmeResources<M: IoMapper> {
    pub iomapper: PhantomData<M>,
    pub bar: IoMem<8192, M>,
}

pub struct NvmeQueues<D: DmaAllocator, I: IrqController, M: IoMapper> {
    pub admin_queue: Option<Arc<NvmeQueue<D, I, M>>>,
    pub io_queues: Vec<Arc<NvmeQueue<D, I, M>>>,
}
impl<D: DmaAllocator, I: IrqController, M: IoMapper> NvmeQueues<D, I, M> {
    pub fn new() -> Self {
        Self {
            admin_queue: None,
            io_queues: Vec::new(),
        }
    }
}

struct NvmeNamespace {
    id: u32,
    lba_shift: u32,
}

impl<I: IoMapper> NvmeResources<I> {
    fn new(phys_addr: usize, size: usize) -> Self {
        Self {
            iomapper: PhantomData,
            bar: IoMem::<8192, I>::new(phys_addr, size),
        }
    }
}

pub struct NvmeDevice<D: DmaAllocator, I: IrqController, M: IoMapper> {
    pub db_stride: usize,
    pub queues: NvmeQueues<D, I, M>,
    resources: NvmeResources<M>,
}

impl<D: DmaAllocator, I: IrqController, M: IoMapper> NvmeDevice<D, I, M> {
    pub fn new(bar_addr: usize, db_stride: usize) -> Self {
        Self {
            db_stride: db_stride,

            queues: NvmeQueues {
                admin_queue: None,
                io_queues: Vec::new(),
            },
            resources: NvmeResources {
                iomapper: PhantomData,
                bar: IoMem::<8192, M>::new(bar_addr, 8192),
            },
        }
    }
}

pub struct NvmeDriver<D: DmaAllocator, I: IrqController, M: IoMapper> {
    pub nvme_dev: Arc<Mutex<NvmeDevice<D, I, M>>>,
    dma: PhantomData<D>,
    iomem: PhantomData<M>,
    irq: PhantomData<I>,
}

impl<D: DmaAllocator, I: IrqController, M: IoMapper> NvmeDriver<D, I, M> {
    pub fn new(bar_addr: usize) -> Self {
        let nvme_dev = Arc::new(Mutex::new(NvmeDevice::<D, I, M>::new(bar_addr, 0)));

        Self {
            dma: PhantomData,
            iomem: PhantomData,
            irq: PhantomData,
            nvme_dev: nvme_dev,
        }
    }
    pub fn init(&self) {
        Self::configure_admin_queue(&self.nvme_dev);
        Self::nvme_setup_io_queues(&self.nvme_dev);
    }

    pub(crate) fn submit_sync_command(
        nvme_dev: &Arc<Mutex<NvmeDevice<D, I, M>>>,
        mut cmd: NvmeCommand,
    ) {
        let nvme = nvme_dev.lock();
        let admin_queue = nvme.queues.admin_queue.as_ref().unwrap().as_ref();
        let bar = &nvme.resources.bar;
        admin_queue.submit_command(&cmd, true, bar);
        admin_queue.nvme_poll_cq(bar);
    }

    pub(crate) fn configure_admin_queue<'a>(nvme_dev: &'a Arc<Mutex<NvmeDevice<D, I, M>>>) {
        let mut dev = nvme_dev.lock();

        let bar = &dev.resources.bar;

        info!("Disable (reset) controller\n");
        bar.writel(0, OFFSET_CC);

        Self::wait_idle(&bar);

        let queue_depth = NVME_QUEUE_DEPTH;

        let admin_queue = NvmeQueue::<D, I, M>::new(0, queue_depth as u16, 0, false, 0);

        let mut aqa = (queue_depth - 1) as u32;
        aqa |= aqa << 16;

        let mut ctrl_config = NVME_CC_ENABLE | NVME_CC_CSS_NVM;
        ctrl_config |= 0 << NVME_CC_MPS_SHIFT;
        ctrl_config |= NVME_CC_ARB_RR | NVME_CC_SHN_NONE;
        ctrl_config |= NVME_CC_IOSQES | NVME_CC_IOCQES;

        {
            // TODO: All writes should support endian conversion
            bar.writel(aqa, OFFSET_AQA);
            bar.writeq(admin_queue.sq.dma_handle, OFFSET_ASQ);
            bar.writeq(admin_queue.cq.dma_handle, OFFSET_ACQ);
            bar.writel(ctrl_config, OFFSET_CC);
        }

        Self::wait_ready(&bar);

        dev.queues.admin_queue = Some(Arc::new(admin_queue));
    }

    pub(crate) fn nvme_setup_io_queues(nvme_dev: &Arc<Mutex<NvmeDevice<D, I, M>>>) {
        let queue_depth = NVME_QUEUE_DEPTH;
        let io_queue = NvmeQueue::<D, I, M>::new(1, queue_depth as u16, 0, false, 0x8);
        Self::set_queue_count(1, nvme_dev);
        Self::alloc_completion_queue(nvme_dev, &io_queue);
        Self::alloc_submission_queue(nvme_dev, &io_queue);
        nvme_dev.lock().queues.io_queues.push(Arc::new(io_queue));
    }

    pub(crate) fn alloc_completion_queue(
        nvme_dev: &Arc<Mutex<NvmeDevice<D, I, M>>>,
        queue: &NvmeQueue<D, I, M>,
    ) {
        info!("alloc cq");
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
        Self::submit_sync_command(nvme_dev, cmd);
    }

    pub(crate) fn alloc_submission_queue(
        nvme_dev: &Arc<Mutex<NvmeDevice<D, I, M>>>,
        queue: &NvmeQueue<D, I, M>,
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

        Self::submit_sync_command(nvme_dev, cmd);
    }

    fn set_queue_count(count: u32, nvme_dev: &Arc<Mutex<NvmeDevice<D, I, M>>>) {
        // let q_count = (count - 1) | ((count - 1) << 16);
        let q_count = count;
        let res = Self::set_features(NVME_FEAT_NUM_QUEUES, q_count, 0, nvme_dev);
    }

    fn set_features(
        fid: u32,
        dword11: u32,
        dma_addr: u64,
        nvme_dev: &Arc<Mutex<NvmeDevice<D, I, M>>>,
    ) -> u32 {
        info!("fid: {}, dma: {}, dword11: {}", fid, dma_addr, dword11);

        Self::submit_sync_command(
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
        info!("Set features done!");
        1
    }

    fn wait_ready(bar: &IoMem<8192, M>) {
        info!("Waiting for controller ready\n");

        while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY == 0 {}

        info!("Controller ready\n");
    }

    fn wait_idle(bar: &IoMem<8192, M>) {
        info!("Waiting for controller idle\n");

        while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY != 0 {}

        info!("Controller ready\n");
    }
}
