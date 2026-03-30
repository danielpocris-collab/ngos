#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicUsize, Ordering};

type IrqHandler = fn(u8);
const IRQ_HANDLER_SLOTS: usize = 4;

type IrqHandlerTable = [[Option<IrqHandler>; IRQ_HANDLER_SLOTS]; 16];

#[cfg(target_os = "none")]
static mut IRQ_HANDLERS: IrqHandlerTable = [[None; IRQ_HANDLER_SLOTS]; 16];
#[cfg(target_os = "none")]
static IRQ_DISPATCH_COUNTS: [AtomicUsize; 16] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

pub fn register_irq_handler(line: u8, handler: IrqHandler) -> bool {
    #[cfg(not(target_os = "none"))]
    {
        let _ = (line, handler);
        false
    }
    #[cfg(target_os = "none")]
    {
        unsafe { register_irq_handler_in_table(&mut IRQ_HANDLERS, line, handler) }
    }
}

pub fn dispatch_irq(line: u8) -> bool {
    #[cfg(not(target_os = "none"))]
    {
        let _ = line;
        false
    }
    #[cfg(target_os = "none")]
    {
        if line >= 16 {
            return false;
        }
        let handlers = unsafe { IRQ_HANDLERS[line as usize] };
        IRQ_DISPATCH_COUNTS[line as usize].fetch_add(1, Ordering::Relaxed);
        let handled = dispatch_handlers(handlers, line);
        crate::diagnostics::record_irq_event(line, irq_dispatch_count(line), handled);
        handled
    }
}

pub fn irq_dispatch_count(line: u8) -> usize {
    #[cfg(not(target_os = "none"))]
    {
        let _ = line;
        0
    }
    #[cfg(target_os = "none")]
    {
        if line >= 16 {
            0
        } else {
            IRQ_DISPATCH_COUNTS[line as usize].load(Ordering::Relaxed)
        }
    }
}

fn register_irq_handler_in_table(
    table: &mut IrqHandlerTable,
    line: u8,
    handler: IrqHandler,
) -> bool {
    if line >= 16 {
        return false;
    }
    let handlers = &mut table[line as usize];
    if handlers
        .iter()
        .any(|registered| registered.is_some_and(|existing| existing as usize == handler as usize))
    {
        return true;
    }
    let Some(slot) = handlers.iter_mut().find(|slot| slot.is_none()) else {
        return false;
    };
    *slot = Some(handler);
    true
}

fn dispatch_handlers(handlers: [Option<IrqHandler>; IRQ_HANDLER_SLOTS], line: u8) -> bool {
    let mut handled = false;
    for handler in handlers.into_iter().flatten() {
        handler(line);
        handled = true;
    }
    handled
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static IRQ_A_COUNT: AtomicUsize = AtomicUsize::new(0);
    static IRQ_B_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn irq_a(_line: u8) {
        IRQ_A_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    fn irq_b(_line: u8) {
        IRQ_B_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn shared_irq_line_dispatches_all_registered_handlers() {
        let mut table = [[None; IRQ_HANDLER_SLOTS]; 16];
        IRQ_A_COUNT.store(0, Ordering::Relaxed);
        IRQ_B_COUNT.store(0, Ordering::Relaxed);

        assert!(register_irq_handler_in_table(&mut table, 11, irq_a));
        assert!(register_irq_handler_in_table(&mut table, 11, irq_b));
        assert!(dispatch_handlers(table[11], 11));
        assert_eq!(IRQ_A_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(IRQ_B_COUNT.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn shared_irq_line_rejects_registration_after_capacity() {
        fn irq_c(_line: u8) {}
        fn irq_d(_line: u8) {}
        fn irq_e(_line: u8) {}

        let mut table = [[None; IRQ_HANDLER_SLOTS]; 16];
        assert!(register_irq_handler_in_table(&mut table, 11, irq_a));
        assert!(register_irq_handler_in_table(&mut table, 11, irq_b));
        assert!(register_irq_handler_in_table(&mut table, 11, irq_c));
        assert!(register_irq_handler_in_table(&mut table, 11, irq_d));
        assert!(!register_irq_handler_in_table(&mut table, 11, irq_e));
    }
}
