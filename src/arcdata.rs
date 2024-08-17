pub(crate) use std::sync::atomic::{fence, AtomicUsize, Ordering};

pub(crate) struct ArcData<T> {
    pub(crate) refs: AtomicUsize,
    pub(crate) data: T,
}

impl<T> ArcData<T> {
    pub(crate) fn new(data: T) -> Self {
        Self {
            refs: AtomicUsize::new(1),
            data,
        }
    }
}
