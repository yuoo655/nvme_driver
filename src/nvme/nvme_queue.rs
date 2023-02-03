use alloc::slice;
use core::cell::Ref;
use core::marker::PhantomData;
use volatile::Volatile;

use super::NvmeCommand;
use super::NvmeCompletion;

use super::NVME_QUEUE_DEPTH;

use crate::dma;
use crate::dma::DmaInfo;
use crate::dma::DmaAllocator;
use crate::dma::dma_alloc;
use crate::iomem::IoMapper;
use crate::iomem::IoMem;

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

pub struct NvmeQueueInner<I: IoMapper> {
    irq_data: PhantomData<I>,
    sq_tail: u16,
    last_sq_tail: u16,
    irq: Option<I>,
}

impl <I:IoMapper> NvmeQueueInner<I>{
    pub fn new() -> Self{

        Self{
            irq_data: PhantomData,
            sq_tail: 0,
            last_sq_tail: 0,
            irq: None
        }
    }
}

pub(crate) struct NvmeQueue<D: DmaAllocator, I: IoMapper> {

    // pub(crate) data: Ref<DeviceData>,

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

impl<D: DmaAllocator, I: IoMapper> NvmeQueue<D, I> {
    pub(crate) fn new(
        // data: Ref<DeviceData>,
        qid: u16, 
        depth: u16, 
        vector: u16, 
        polled: bool, 
        db_stride: usize
    ) -> Self {

        let cq: DmaInfo<NvmeCompletion, D> = dma_alloc::<NvmeCompletion,D>(depth.into());
        let sq: DmaInfo<NvmeCommand, D> = dma_alloc::<NvmeCommand, D>(depth.into());

        // Zero out all completions. This is necessary so that we can check the phase.
        for i in 0..depth {
            cq.write_volatile(i.into(), &NvmeCompletion::default());
        }

        let sdb_offset = (qid as usize) * db_stride * 2;
        let db_offset = sdb_offset + 4096;


        let inner = Mutex::new(NvmeQueueInner::<I>::new());

        NvmeQueue {
            // data,
            db_offset,
            sdb_index: sdb_offset / 4,
            qid,
            sq,
            cq,
            q_depth: depth,
            cq_vector: vector,
            cq_head: AtomicU16::new(0),
            cq_phase: AtomicU16::new(1),                        
            inner: inner,
            polled,
        }
    }


    // /// Processes the completion queue.
    // ///
    // /// Returns `true` if at least one entry was processed, `false` otherwise.
    // pub(crate) fn process_completions(&self) -> i32 {
    //     let mut head = self.cq_head.load(Ordering::Relaxed);
    //     let mut phase = self.cq_phase.load(Ordering::Relaxed);
    //     let mut found = 0;

    //     loop {
    //         let cqe = self.cq.read_volatile(head.into()).unwrap();

    //         if cqe.status.into() & 1 != phase {
    //             break;
    //         }

    //         let cqe = self.cq.read_volatile(head.into()).unwrap();

    //         found += 1;
    //         head += 1;
    //         if head == self.q_depth {
    //             head = 0;
    //             phase ^= 1;
    //         }
    //     }    
    //         // cqe.result.into(), Ordering::Relaxed;
    //         // cqe.status.into() >> 1, Ordering::Relaxed

    //     if found == 0 {
    //         return found;
    //     }

    //     if self.dbbuf_update_and_check_event(head.into(), self.data.db_stride / 4) {
    //         self.data.bar.writel(head.into(), self.db_offset + self.data.db_stride);
    //     }

    //     // TODO: Comment on why it's ok.
    //     self.cq_head.store(head, Ordering::Relaxed);
    //     self.cq_phase.store(phase, Ordering::Relaxed);

    //     found

    // }

    // pub(crate) fn dbbuf_update_and_check_event(&self, value: u16, extra_index: usize) -> bool {
    //     if self.qid == 0 {
    //         return true;
    //     }

    //     let shadow = if let Some(s) = &self.data.shadow {
    //         s
    //     } else {
    //         return true;
    //     };

    //     let index = self.sdb_index + extra_index;

    //     // TODO: This should be a wmb (sfence on x86-64).
    //     // Ensure that the queue is written before updating the doorbell in memory.
    //     fence(Ordering::SeqCst);

    //     let old_value = shadow.dbs.read_write(index, value.into()).unwrap();

    //     // Ensure that the doorbell is updated before reading the event index from memory. The
    //     // controller needs to provide similar ordering to ensure the envent index is updated
    //     // before reading the doorbell.
    //     fence(Ordering::SeqCst);

    //     let ei = shadow.eis.read_volatile(index).unwrap();
    //     Self::dbbuf_need_event(ei as _, value, old_value as _)
    // }



    // pub(crate) fn write_sq_db(&self, write_sq: bool) {
    //     let mut inner = self.inner.lock();
    //     self.write_sq_db_locked(write_sq, &mut inner);
    // }

    // fn write_sq_db_locked(&self, write_sq: bool, inner: &mut NvmeQueueInner<I>) {
    //     if !write_sq {
    //         let mut next_tail = inner.sq_tail + 1;
    //         if next_tail == self.q_depth {
    //             next_tail = 0;
    //         }
    //         if next_tail != inner.last_sq_tail {
    //             return;
    //         }
    //     }

    //     if self.dbbuf_update_and_check_event(inner.sq_tail, 0) {
    //         self.data.bar.try_writel(inner.sq_tail.into(), self.db_offset);
    //     }
    //     inner.last_sq_tail = inner.sq_tail;
    // }

    // pub(crate) fn submit_command(&self, cmd: &NvmeCommand, is_last: bool) {
    //     let mut inner = self.inner.lock();
    //     self.sq.write(inner.sq_tail.into(), cmd);
    //     inner.sq_tail += 1;
    //     if inner.sq_tail == self.q_depth {
    //         inner.sq_tail = 0;
    //     }
    //     self.write_sq_db_locked(is_last, &mut inner);
    // }
    
    // pub(crate) fn unregister_irq(&self) {
    //     // Do not drop registration while spinlock is held, irq::free will take
    //     // a mutex and might sleep.
    //     let mut registration = self.inner.lock().irq.take();
    //     drop(registration);
    // }

    // pub(crate) fn register_irq(self: &Ref<Self>){
    //     info!(
    //         "Registering irq for queue qid: {}, vector {}\n",
    //         self.qid,
    //         self.cq_vector
    //     );
    // }   

}



