use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};

use super::nvme_defs::*;
use super::nvme_queue::*;
use crate::dma::DmaAllocator;
use crate::dma::DmaInfo;
use crate::iomem::{IoMapper, IoMem};
use crate::irq::IrqController;
use lock::Mutex;
use lock::MutexGuard;

use log::info;

pub const NVME_QUEUE_DEPTH: usize = 1024;

pub const NVME_Q_DEPTH: usize = 64;

use alloc::boxed::Box;
use core::{
    cell::UnsafeCell,
    convert::TryInto,
    format_args,
    pin::Pin,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering},
};

struct NvmeNamespace {
    id: u32,
    lba_shift: u32,
}

struct NvmeResources<I: IoMapper> {
    iomapper: PhantomData<I>,
    bar: IoMem<8192, I>,
}

impl<I: IoMapper> NvmeResources<I> {
    fn new(phys_addr: usize, size: usize) -> Self {
        Self {
            iomapper: PhantomData,
            bar: IoMem::<8192, I>::new(phys_addr, size),
        }
    }
}

pub struct NvmeDevice<D: DmaAllocator, I: IoMapper> {
    dma: PhantomData<D>,
    iomem: PhantomData<I>,
}

impl<D: DmaAllocator, I: IoMapper> NvmeDevice<D, I> {
    pub fn configure_admin_queue(bar_raw: usize) {
        let mut bar = IoMem::<8192, I>::new(bar_raw, 8192);

        info!("Disable (reset) controller\n");
        bar.writel(0, OFFSET_CC);

        Self::wait_idle(&bar);

        let queue_depth = NVME_QUEUE_DEPTH;

        let admin_queue = NvmeQueue::<D, I>::new(0, queue_depth as u16, 0, false, 0);

        // //lba_shift = 2^9 512
        // let ns = Box::try_new(NvmeNamespace {
        //     id: 0,
        //     lba_shift: 9,
        // })?;

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
    }

    fn wait_ready(bar: &IoMem<8192, I>) {
        info!("Waiting for controller ready\n");

        while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY == 0 {}

        info!("Controller ready\n");
    }

    fn wait_idle(bar: &IoMem<8192, I>) {
        info!("Waiting for controller idle\n");

        while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY != 0 {}

        info!("Controller ready\n");
    }
}

// pub fn alloc_ns() {}

// pub fn setup_io_queues() {}

// pub fn submit_sync_command() {}

// pub fn set_queue_count() {}

// pub fn alloc_completion_queue() {}

// pub fn alloc_submission_queue() {}

// pub fn identify() {}

// pub fn get_features() {}

// pub fn set_features() {}

// pub fn dbbuf_set() {}
