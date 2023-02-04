use core::{ptr::{read_volatile, write_volatile}, marker::PhantomData};
use log::info;

pub trait DmaAllocator {

    
    // @cpu_addr: kernel CPU-view address returned from dma_alloc_attrs
    // @dma_addr: device-view address returned from dma_alloc_attrs
    // @size: size of memory originally requested in dma_alloc_attrs
    // @attrs: attributes of mapping properties requested in dma_alloc_attrs
    
    // Map a coherent DMA buffer previously allocated by dma_alloc_attrs into user
    // space.  The coherent DMA buffer must not be freed by the driver until the
    // user space mapping has been released.
    fn dma_alloc(size: usize, dma_handle: &mut u64) -> usize{
        info!("dma_alloc");
        0
    }
    fn dma_dealloc(cpu_addr: *mut (), dma_handle: u64, size: usize){
        info!("dma_dealloc");
    }
}

pub fn dma_alloc<T, D:DmaAllocator>(count: usize) -> DmaInfo<T, D> {
    let t_size = core::mem::size_of::<T>();
    let size = count.checked_mul(t_size).unwrap();
    
    info!("dma_alloc t_size:{:#x?} count:{:?} size:{:#x?}", t_size, count, size);
    let mut dma_handle = 0;

    let cpu_addr =  D::dma_alloc(size, &mut dma_handle);

    DmaInfo::new(cpu_addr as _, dma_handle, count)
}

pub struct DmaInfo<T, D:DmaAllocator> {
    dma: PhantomData<D>,
    count: usize,
    // addr for device
    pub dma_handle: u64,
    // addr for kernel
    cpu_addr: *mut T,
}

impl<T, D: DmaAllocator> DmaInfo<T, D> {
    pub fn new(cpu_addr: *mut T, dma_handle: u64, count: usize) -> Self {
        Self {
            count: count,
            dma_handle : dma_handle,
            cpu_addr: cpu_addr,
            dma: PhantomData,

        }
    }

    pub fn read_volatile(&self, index: usize) -> Option<T> {
        if index >= self.count {
            return None;
        }

        let ptr = self.cpu_addr.wrapping_add(index);

        // SAFETY: We just checked that the index is within bounds.
        Some(unsafe { ptr.read_volatile() })
    }

    pub fn write_volatile(&self, index: usize, value: &T) -> bool
    where
        T: Copy,
    {
        if index >= self.count {
            return false;
        }

        let ptr = self.cpu_addr.wrapping_add(index);

        // SAFETY: We just checked that the index is within bounds.
        unsafe { ptr.write_volatile(*value) };
        true
    }

    
}

impl<T, D: DmaAllocator> Drop for DmaInfo<T, D> {
    fn drop(&mut self) {
        let size = self.count * core::mem::size_of::<T>();
        D::dma_dealloc(self.cpu_addr as _, self.dma_handle, size);
    }
}
