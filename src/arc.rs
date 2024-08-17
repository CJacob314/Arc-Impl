use crate::arcdata::*;

use std::ops::Deref;
use std::ptr::NonNull;

pub struct Arc<T> {
    data: NonNull<ArcData<T>>,
}

impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data().data
    }
}

impl<T> Arc<T> {
    /// Creates a new `Arc<T>` containing data of type `T`.
    /// # Arguments
    /// * `data` - The data to be stored in the `Arc<T>`.
    /// # Examples
    /// ```
    /// use arc::Arc;
    /// let arc = Arc::new(42);
    /// assert_eq!(*arc, 42);
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            data: NonNull::from(Box::leak(Box::new(ArcData::new(data)))),
        }
    }

    /// Returns the number of references to this `Arc<T>`.
    /// # Examples
    /// ```
    /// use arc::Arc;
    /// let arc = Arc::new(42);
    /// assert_eq!(arc.ref_count(), 1);
    /// ```
    pub fn ref_count(&self) -> usize {
        self.data().refs.load(Ordering::Relaxed)
    }

    /// Returns an [`Option::Some`] containing a mutable reference to the data if this is the only reference.
    /// Otherwise, returns [`Option::None`].
    ///
    /// # Arguments
    /// * `this` - A mutable reference to an `Arc<T>`.
    ///
    /// # Examples
    /// ```
    /// use arc::Arc;
    /// let mut arc = Arc::new(0); // Not the meaning of life
    /// if let Some(x) = Arc::get_mut(&mut arc) {
    ///     *x = 42;               // Fixed
    /// }
    /// assert_eq!(*arc, 42);
    /// ```
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        // This method takes a named mutable reference to something of type `Self` to reduce
        // ambiguity (it becomes clear the user is calling `get_mut` on the Arc<T> and not on the Deref `T`).
        if this.data().refs.load(Ordering::Relaxed) == 1 {
            // The relaxed load is a part of the all-thread-spanning total-modification-order.
            // If we relaxed-load a 1, we know our ref count is a 1 at that instant.
            fence(Ordering::Acquire);
            // The above acquire fence ensures that nothing after it gets reordered before it. This
            // ensures that this function doesn't return a mutable reference to the data before we
            // know that we are the ONLY arc (i.e., that the ref count is 1).
            // An acquire-fence was chosen here over an Acquire load for efficiency: the fence will ONLY run if the ref count is 1.

            // There is additionally no possibility that the ref count atomic integer gets incremented at *any time* after the relaxed load of a 1 in this function, since:
            // a. We must be the only Arc with this shared ArcData (ref count == 1)
            // b. The compiler will not let any other functions which borrow (mutably or immutably) this Arc (and change the ref count) be called since we have a mutable (exclusive) reference.
            Some(&mut this.data_mut().data)
        } else {
            None
        }
    }

    // Private functions
    fn data(&self) -> &ArcData<T> {
        unsafe { self.data.as_ref() }
    }

    fn data_mut(&mut self) -> &mut ArcData<T> {
        unsafe { self.data.as_mut() }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().refs.fetch_add(1, Ordering::Relaxed) > usize::MAX / 3 {
            std::process::abort();
        }
        Self { data: self.data }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // The atomic memory orderings here are only to prevent the compiler from reordering (and maybe some wacky architectures) the drop (not an atomic operation)
        // before the fetch_sub.
        //
        // x86-64, ARM, PowerPC and other architectures that use MESI or MOESI cache coherence protocols already guarantee that even a
        // relaxed atomic operation will be "immediately" visible to all other cores in the system
        // (since it had to get the cache line in exclusive mode to perform the operation).
        if self.data().refs.fetch_sub(1, Ordering::Release) == 1 {
            // The above release and everything before it "happens before" the following acquire fence and everything after it.
            fence(Ordering::Acquire);
            drop(unsafe { Box::from_raw(self.data.as_ptr()) });
        }
    }
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}
