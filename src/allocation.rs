use std::{
    alloc::{self, Layout},
    slice,
};

use crate::slab::Slab;

/// An allocation of memory that may be from an [`Allocator`][crate::Allocator]
/// or from [`alloc::alloc_zeroed()`].
///
/// Automatically frees itself when dropped.
///
/// Uses [`Layout::array::<u8>()`].
#[derive(Debug)]
#[must_use]
pub struct Allocation {
    source: Source,
    bytes: *mut u8,
    size: usize,
}

impl Allocation {
    pub(crate) fn slab(bytes: *mut u8, size: usize, slab: Slab) -> Self {
        Self {
            source: Source::Slab { slab },
            bytes,
            size,
        }
    }

    /// Returns a new allocation using [`alloc::alloc_zeroed()`].
    pub fn global(size: usize) -> Self {
        let layout = Layout::array::<u8>(size).expect("invalid allocation length");
        // SAFETY: This pointer is freed in Drop. when source is Global.
        let bytes = unsafe { alloc::alloc_zeroed(layout) };
        Self {
            source: Source::Global { layout },
            bytes,
            size,
        }
    }

    #[must_use]
    pub const fn address(&self) -> *mut u8 {
        self.bytes
    }

    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.size
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: The length and address of this area of memory have been
        // dedicated to this Allocation instance. The Rust borrow checker will
        // prevent a mutable borrow from happening on top of this immutable
        // borrow.
        unsafe { slice::from_raw_parts(self.address(), self.size) }
    }

    #[must_use]
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        // SAFETY: The length and address of this area of memory have been
        // dedicated to this Allocation instance. The Rust borrow checker will
        // prevent any other attempt to borrow slices to this range of memory
        // while this exclusive reference is held.
        unsafe { slice::from_raw_parts_mut(self.address(), self.size) }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        match &self.source {
            Source::Slab { slab } => slab.free(self.bytes, self.size),
            Source::Global { layout } => {
                // SAFETY: When source is global, bytes came from alloc() not a shared slab.
                unsafe { alloc::dealloc(self.bytes, *layout) }
            }
        }
    }
}

// SAFETY: u8 is Send, and data is always initialized.
unsafe impl Send for Allocation {}

// SAFETY: u8 is Sync, and data is always initialized.
unsafe impl Sync for Allocation {}

#[derive(Debug)]
enum Source {
    Slab { slab: Slab },
    Global { layout: Layout },
}
