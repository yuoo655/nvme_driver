use core::{
    cell::UnsafeCell,
    clone::Clone,
    convert::TryInto,
    ffi::c_void,
    format_args,
    marker::PhantomData,
    ops::DerefMut,
    pin::Pin,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering},
};

extern crate alloc;


use alloc::sync::Arc;
use alloc::vec::Vec;

pub mod nvme_defs;
pub mod nvme_impl;
pub mod nvme_queue;
pub mod nvme_traits;

pub use nvme_defs::*;
pub use nvme_impl::*;
pub use nvme_queue::*;
pub use nvme_traits::*;



pub struct NvmeQueues<A, T>
where
    A: NvmeTraits + 'static,
{
    phantom: PhantomData<A>,
    pub admin_queue: Option<Arc<NvmeQueue<A, T>>>,
    pub io_queues: Vec<Arc<NvmeQueue<A, T>>>,
}

impl<A, T> NvmeQueues<A, T>
where
    A: NvmeTraits,
{
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
            admin_queue: None,
            io_queues: Vec::new(),
        }
    }
}


pub struct NvmeCommonData<A>
where
    A: NvmeTraits + 'static,
{
    pub bar: IoMem<8192, A>,
}



