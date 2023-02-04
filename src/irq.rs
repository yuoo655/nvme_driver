use alloc::sync::Arc;

pub trait IrqController {
    fn request_irq(irq_num: usize) {}

    fn enable_irq(irq_num: usize) {}

    fn disable_irq(irq_num: usize) {}
}

/// An irq handler.
pub trait Handler<T> {
    /// Called from interrupt context when the irq happens.
    fn handle_irq(data: Arc<T>) -> Return;
}

/// see include/linux/irqreturn.h
/// The return value from interrupt handlers.
pub enum Return {
    /// The interrupt was not from this device or was not handled.
    None = (0 << 0) as _,

    /// The interrupt was handled by this device.
    Handled = (1 << 0) as _,

    /// The handler wants the handler thread to wake up.
    WakeThread = (1 << 1) as _,
}
