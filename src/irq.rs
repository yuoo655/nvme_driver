pub trait IrqController {
    fn disable_irq(irq_num: usize);

    fn enable_irq(irq_num: usize);

    fn register_irq(irq: usize, handler: fn(usize));

    fn unregister_irq(irq: usize);
}
