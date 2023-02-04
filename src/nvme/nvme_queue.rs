use alloc::slice;
use alloc::sync::Arc;
use alloc::sync::Weak;
use core::cell::Ref;
use core::marker::PhantomData;
use volatile::Volatile;

use super::NvmeCommand;
use super::NvmeCommon;
use super::NvmeCompletion;

use super::NVME_QUEUE_DEPTH;
// use super::NvmeDeviceData;

use crate::dma;
use crate::dma::dma_alloc;
use crate::dma::DmaAllocator;
use crate::dma::DmaInfo;
use crate::iomem::IoMapper;
use crate::iomem::IoMem;
use crate::NvmeDevice;

use crate::irq::IrqController;

const PAGE_SIZE: usize = 4096;

use core;
use core::pin::Pin;
use core::sync::atomic::fence;
use core::sync::atomic::AtomicU16;
use core::sync::atomic::Ordering;

use lock::Mutex;
use lock::MutexGuard;

use log::info;

pub struct NvmeQueueInner<I: IrqController> {
    irq_data: PhantomData<I>,
    sq_tail: u16,
    last_sq_tail: u16,
    irq: Option<I>,
}

impl<I: IrqController> NvmeQueueInner<I> {
    pub fn new() -> Self {
        Self {
            irq_data: PhantomData,
            sq_tail: 0,
            last_sq_tail: 0,
            irq: None,
        }
    }
}

pub struct NvmeQueue<D: DmaAllocator, I: IrqController, M: IoMapper> {
    // pub(crate) nvme_dev: &Arc<NvmeDeviceData<D, I, M>>,

    m: PhantomData<M>,
    pub(crate) db_offset: usize,
    pub(crate) sdb_index: usize,
    pub(crate) qid: u16,
    pub(crate) polled: bool,

    cq_head: AtomicU16,
    cq_phase: AtomicU16,

    pub(crate) sq: DmaInfo<NvmeCommand, D>,
    pub(crate) cq: DmaInfo<NvmeCompletion, D>,

    pub(crate) q_depth: u16,
    pub(crate) cq_vector: u16,

    inner: Mutex<NvmeQueueInner<I>>,
}

impl<D, I, M> NvmeQueue<D, I, M>
where
    D: DmaAllocator,
    I: IrqController,
    M: IoMapper,
{
    pub(crate) fn new(
        // nvme_dev: &Arc<NvmeDeviceData<D, I, M>>,
        qid: u16,
        depth: u16,
        vector: u16,
        polled: bool,
        db_stride: usize,
    ) -> Self {
        let cq: DmaInfo<NvmeCompletion, D> = dma_alloc::<NvmeCompletion, D>(depth.into());
        let sq: DmaInfo<NvmeCommand, D> = dma_alloc::<NvmeCommand, D>(depth.into());

        // Zero out all completions. This is necessary so that we can check the phase.
        for i in 0..depth {
            cq.write_volatile(i.into(), &NvmeCompletion::default());
        }

        let sdb_offset = (qid as usize) * db_stride * 2;
        let db_offset = sdb_offset + 4096;

        let inner = Mutex::new(NvmeQueueInner::<I>::new());

        NvmeQueue {
            // nvme_dev,
            m: PhantomData,
            db_offset,
            sdb_index: sdb_offset / 4,
            qid,
            sq,
            cq,
            q_depth: depth,
            cq_vector: vector,
            cq_head: AtomicU16::new(0),
            cq_phase: AtomicU16::new(1),
            inner,
            polled,
        }
    }

    pub(crate) fn submit_command(&self, cmd: &NvmeCommand, is_last: bool, bar: &IoMem<8192, M>) {
        let mut inner = self.inner.lock();
        self.sq.write_volatile(inner.sq_tail.into(), cmd);
        inner.sq_tail += 1;
        if inner.sq_tail == self.q_depth {
            inner.sq_tail = 0;
        }
        self.nvme_write_sq_db(is_last, &mut inner, bar);
    }

    // write submission queue doorbell to notify nvme device
    pub fn nvme_write_sq_db(&self, write_sq: bool, inner: &mut MutexGuard<NvmeQueueInner<I>>, bar: &IoMem<8192, M>) {
        if !write_sq {
            let mut next_tail = inner.sq_tail + 1;
            if next_tail == self.q_depth {
                next_tail = 0;
            }
            if next_tail != inner.last_sq_tail {
                return;
            }
        }

        // let bar = &self.nvme_dev.lock().resources.bar;
        bar.writel(inner.sq_tail.into(), self.db_offset);
        inner.last_sq_tail = inner.sq_tail;
    }

    // check if there is completed command in completion queue
    pub fn nvme_cqe_pending(&self, inner: &mut MutexGuard<NvmeQueueInner<I>>) -> bool {
        let mut head = self.cq_head.load(Ordering::Relaxed);
        let mut phase = self.cq_phase.load(Ordering::Relaxed);
        let cqe = self.cq.read_volatile(head.into()).unwrap();
        if cqe.status.into() & 1 != phase {
            return true;
        } else {
            return false;
        }
    }

    // update completion queue head
    pub fn nvme_update_cq_head(&self, inner: &mut MutexGuard<NvmeQueueInner<I>>) {
        let mut head = self.cq_head.load(Ordering::Relaxed);
        let mut phase = self.cq_phase.load(Ordering::Relaxed);

        let next_head = head + 1;
        if next_head == self.q_depth {
            head = 0;
            phase ^= 1;
        } else {
            head = next_head;
        }
    }

    // notify nvme device we've completed the command
    pub fn nvme_ring_cq_doorbell(&self, inner: &mut MutexGuard<NvmeQueueInner<I>>, bar: &IoMem<8192, M>) {
        let mut head = self.cq_head.load(Ordering::Relaxed);
        let mut phase = self.cq_phase.load(Ordering::Relaxed);
        // let bar = &self.nvme_dev.lock().resources.bar;
        bar.writel(head.into(), self.db_offset + 0x4);
    }

    // check completion queue and update cq head cq doorbell until there is no pending command
    pub fn nvme_poll_cq(&self, bar: &IoMem<8192, M>) {

        let inner = &mut self.inner.lock();

        info!("poll cq got lock");
        if !self.nvme_cqe_pending(inner) {
        }
        
        self.nvme_update_cq_head(inner);
        self.nvme_ring_cq_doorbell(inner, bar);

        
    }

    /// Processes the completion queue.
    ///
    /// Returns `true` if at least one entry was processed, `false` otherwise.
    pub(crate) fn process_completions(&self, bar: &IoMem<8192, M>) -> i32 {
        let mut head = self.cq_head.load(Ordering::Relaxed);
        let mut phase = self.cq_phase.load(Ordering::Relaxed);
        let mut found = 0;

        loop {
            let cqe = self.cq.read_volatile(head.into()).unwrap();

            if cqe.status.into() & 1 != phase {
                break;
            }

            let cqe = self.cq.read_volatile(head.into()).unwrap();

            found += 1;
            head += 1;
            if head == self.q_depth {
                head = 0;
                phase ^= 1;
            }
        }

        if found == 0 {
            return found;
        }

        // let bar = &self.nvme_dev.lock().resources.bar;
        bar.writel(head.into(), self.db_offset + 0x4);

        self.cq_head.store(head, Ordering::Relaxed);
        self.cq_phase.store(phase, Ordering::Relaxed);

        found
    }
}
