use crate::nvme_defs::*;
use crate::nvme_traits::*;
use crate::nvme_queue::*;
use crate::*;


use alloc::sync::Arc;
use alloc::vec::Vec;



pub fn wait_ready<A: NvmeTraits>(bar: &IoMem<8192, A>) {

    // let bar = &dev_data.bar;
    while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY == 0 {}

}


pub fn wait_idle<A: NvmeTraits>(bar: &IoMem<8192, A>) {
    // let bar = &dev_data.bar;
    while bar.readl(OFFSET_CSTS) & NVME_CSTS_RDY != 0 {}
}


pub fn config_admin_queue<A: NvmeTraits, T>(bar: &IoMem<8192, A>, admin_queue: &NvmeQueue<A, T>) {
    
    // let bar = bar;
    bar.writel(0, OFFSET_CC);
    wait_idle(bar);

    let mut aqa = (NVME_QUEUE_DEPTH - 1) as u32;
    aqa |= aqa << 16;
    let mut ctrl_config = NVME_CC_ENABLE | NVME_CC_CSS_NVM;
    ctrl_config |= 0 << NVME_CC_MPS_SHIFT;
    ctrl_config |= NVME_CC_ARB_RR | NVME_CC_SHN_NONE;
    ctrl_config |= NVME_CC_IOSQES | NVME_CC_IOCQES;
    {
        bar.writel(aqa, OFFSET_AQA);
        bar.writeq(admin_queue.sq.dma_handle, OFFSET_ASQ);
        bar.writeq(admin_queue.cq.dma_handle, OFFSET_ACQ);
        bar.writel(ctrl_config, OFFSET_CC);
    }
    wait_ready(bar);
}





