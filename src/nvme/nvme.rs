use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};

use super::nvme_defs::*;
use super::nvme_queue::*;
use crate::dma::DmaAlloc;
use crate::dma::DmaAllocator;
use crate::iomem::IoMem;
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

struct NvmeResources {
    bar: IoMem<8192>,
}

pub struct NvmeRequest {
    dma_addr: AtomicU64,
    result: AtomicU32,
    status: AtomicU16,
    direction: AtomicU32,
    len: AtomicU32,
    cmd: UnsafeCell<NvmeCommand>,
    sg_count: AtomicU32,
    page_count: AtomicU32,
    first_dma: AtomicU64,
}


// struct NvmeQueues<D:DmaAllocator, I: IrqController> {
//     admin: Option<Arc<NvmeQueue<D, I>>>,
//     io: Vec<Arc<NvmeQueue<D, I>>>,
// }

// struct NvmeShadow<D: DmaAllocator> {
//     dbs: DmaAlloc<u32, D>,
//     eis: DmaAlloc<u32, D>,
// }

// pub struct NvmeData<D: DmaAllocator, I: IrqController> {

//     db_stride: usize,
//     instance: u32,
//     dma_pool: usize,

//     shadow: Option<NvmeShadow<D>>,

//     queues: Mutex<NvmeQueues<D, I>>,

//     poll_queue_count: u32,

//     irq_queue_count: u32,
// }

struct NvmeDevice;

impl NvmeDevice {
    pub fn alloc_ns() {}

    fn wait_ready(bar: IoMem) {
        info!("Waiting for controller ready\n");
        {
            while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY == 0 {
                // unsafe { bindings::mdelay(100) };
                // TODO: Add check for fatal signal pending.
                // TODO: Set timeout.
            }
        }
        info!("Controller ready\n");
    }

    fn wait_idle(bar: IoMem) {
        info!("Waiting for controller idle\n");
        {
            // let bar = &dev.resources().unwrap().bar;
            while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY != 0 {}
        }
        info!("Controller ready\n");
    }

    fn configure_admin_queue(
        bar: IoMem,
        pci_dev: &pci::Device,
    ) -> Result<(
        Ref<nvme_queue::NvmeQueue<nvme_mq::AdminQueueOperations>>,
        mq::RequestQueue<nvme_mq::AdminQueueOperations>,
    )> {
        info!("Disable (reset) controller\n");
        {
            bar.writel(0, OFFSET_CC);
        }
        Self::wait_idle(bar);

        //TODO: Depth?
        let queue_depth = NVME_Q_DEPTH;

        let admin_queue = NvmeQueue::new(0, queue_depth, 0, false);


        //lba_shift = 2^9 512
        let ns = Box::try_new(NvmeNamespace {
            id: 0,
            lba_shift: 9,
        })?;

        let mut aqa = (queue_depth - 1) as u32;
        aqa |= aqa << 16;

        let mut ctrl_config = NVME_CC_ENABLE | NVME_CC_CSS_NVM;
        ctrl_config |= (kernel::PAGE_SHIFT - 12) << NVME_CC_MPS_SHIFT;
        ctrl_config |= NVME_CC_ARB_RR | NVME_CC_SHN_NONE;
        ctrl_config |= NVME_CC_IOSQES | NVME_CC_IOCQES;


        info!("About to wait for nvme readiness\n");
        {
            // TODO: All writes should support endian conversion
            bar.writel(aqa, OFFSET_AQA);
            bar.writeq(admin_queue.sq.dma_handle, OFFSET_ASQ);
            bar.writeq(admin_queue.cq.dma_handle, OFFSET_ACQ);
            bar.writel(ctrl_config, OFFSET_CC);
        }

        Self::wait_ready(bar);

        info!("Registering admin queue irq");

        admin_queue.register_irq(pci_dev)?;

        info!("Done registering admin queue irq");

        Ok(admin_queue)
    }

    pub fn setup_io_queues() {}

    pub fn submit_sync_command() {}

    pub fn set_queue_count() {}

    pub fn alloc_completion_queue() {}

    pub fn alloc_submission_queue() {}

    pub fn identify() {}

    pub fn get_features() {}

    pub fn set_features() {}

    pub fn dbbuf_set() {}
}




/// Device data.
///
/// When a device is removed (for whatever reason, for example, because the device was unplugged or
/// because the user decided to unbind the driver), the driver is given a chance to clean its state
/// up, and all io resources should ideally not be used anymore.
///
/// However, the device data is reference-counted because other subsystems hold pointers to it. So
/// some device state must be freed and not used anymore, while others must remain accessible.
///
/// This struct separates the device data into three categories:
///   1. Registrations: are destroyed when the device is removed, but before the io resources
///      become inaccessible.
///   2. Io resources: are available until the device is removed.
///   3. General data: remain available as long as the ref count is nonzero.
///
/// This struct implements the `DeviceRemoval` trait so that it can clean resources up even if not
/// explicitly called by the device drivers.
pub struct Data<T, U, V> {
    registrations: Mutex<T>,
    resources: Mutex<U>,
    general: V,
}

impl<T, U, V> Data<T, U, V> {
    /// Creates a new instance of `Data`.
    ///
    /// It is recommended that the [`new_device_data`] macro be used as it automatically creates
    /// the lock classes.
    pub fn try_new() {}

    /// Returns the resources if they're still available.
    pub fn resources(&self) -> Option<RevocableGuard<'_, U>> {
        self.resources.try_access()
    }

    /// Returns the locked registrations if they're still available.
    pub fn registrations(&self) -> Option<RevocableMutexGuard<'_, T>> {
        self.registrations.try_write()
    }
}

impl<T, U, V> Drop for Data<T, U, V> {
    fn device_remove(&self) {
        // We revoke the registrations first so that resources are still available to them during
        // unregistration.
        self.registrations.revoke();

        // Release resources now. General data remains available.
        self.resources.revoke();
    }
}

/// Custom code within device removal.
pub trait DeviceRemoval {
    /// Cleans resources up when the device is removed.
    ///
    /// This is called when a device is removed and offers implementers the chance to run some code
    /// that cleans state up.
    fn device_remove(&self);
}
