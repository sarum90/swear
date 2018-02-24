// Trait that schedules a FnOnce for immediate execution.
//
// Generally this puts the FnOnce at the end of the current run queue.
//
// If you squint at it, this is sort of like yielding to other greenthreads, and also keeps stack
// depth in check
//
// Note: this might come at the cost of data cache consistency as anything captured by the FnOnce
// cannot live on the stack of the caller to schedule.
pub trait Scheduler<'a>: Clone {
    fn schedule<F: 'a>(&self, f: F)
        where F : FnOnce();
}
