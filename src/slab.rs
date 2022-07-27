use std::{
    alloc::{self, Layout},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::allocation::Allocation;

/// A reference counted, fixed-size allocation of memory.
#[derive(Debug, Clone)]
pub struct Slab {
    data: Arc<Data>,
}

#[derive(Debug)]
struct Data {
    layout: Layout,
    minimum_allocation_size: usize,
    bytes: *mut u8,
    free_spans: Mutex<Vec<Span>>,
}

// SAFETY: u8 is Send, and data is always initialized.
unsafe impl Send for Data {}

// SAFETY: u8 is Sync, and data is always initialized.
unsafe impl Sync for Data {}

impl Slab {
    pub fn new(length: usize, layout: Layout, minimum_allocation_size: usize) -> Self {
        let total_stripes = length / minimum_allocation_size;
        // SAFETY: This can panic in out of memory situations, but no undefined
        // behavior should be possible from this call. This pointer is dealloced
        // in Drop.
        let bytes = unsafe { alloc::alloc_zeroed(layout) };
        Self {
            data: Arc::new(Data {
                layout,
                minimum_allocation_size,
                bytes,
                free_spans: Mutex::new(vec![Span {
                    offset: 0,
                    stripes: total_stripes,
                }]),
            }),
        }
    }

    pub fn allocate(&self, length: usize) -> Option<Allocation> {
        struct BestSpan {
            index: usize,
            extra_stripes: usize,
        }
        // To prevent a degree of fragmentation and provide interior alignment
        // guarantees, we're going to allocate in "stripes" of
        // minimum_allocation_size.
        let stripes_needed =
            (length + (self.data.minimum_allocation_size - 1)) / self.data.minimum_allocation_size;

        let mut free_spans = self.data.free_spans.try_lock()?;
        let mut best_span = None;

        // Find the span with the tightest fit.
        for (index, span) in free_spans.iter().enumerate() {
            if let Some(extra_stripes) = span.stripes.checked_sub(stripes_needed) {
                if best_span.as_ref().map_or(true, |best_span: &BestSpan| {
                    extra_stripes < best_span.extra_stripes
                }) {
                    best_span = Some(BestSpan {
                        index,
                        extra_stripes,
                    });
                    if extra_stripes == 0 {
                        break;
                    }
                }
            }
        }

        if let Some(best_span) = best_span {
            let span = &mut free_spans[best_span.index];
            span.stripes -= stripes_needed;
            // SAFETY: span.offset will always be within the allocated range.
            let bytes = unsafe { self.data.bytes.add(span.offset) };
            let allocated_length = stripes_needed * self.data.minimum_allocation_size;
            span.offset += allocated_length;
            if span.stripes == 0 {
                free_spans.remove(best_span.index);
            }
            Some(Allocation::slab(bytes, allocated_length, self.clone()))
        } else {
            None
        }
    }

    pub fn free(&self, allocation: *mut u8, length: usize) {
        // SAFETY: This is an internal type, and this function can only be
        // called from this crate. It is only called with `allocation` being
        // from the same slab, as a reference to the clone when the allocation
        // was created is used to call this function. As such, allocation must
        // lie within the allocated range of self.data.bytes.
        let offset = usize::try_from(unsafe { allocation.offset_from(self.data.bytes) })
            .expect("invalid allocation pointer");
        let freed_span = Span {
            offset,
            stripes: length / self.data.minimum_allocation_size,
        };
        let mut free_spans = self.data.free_spans.lock();

        for (index, span) in free_spans.iter_mut().enumerate() {
            if span.offset < freed_span.offset
                && span.end(self.data.minimum_allocation_size) == freed_span.offset
            {
                // The new span intersects with the beginning of a new span.
                // We can merge.
                span.stripes += freed_span.stripes;
                let new_end = span.end(self.data.minimum_allocation_size);
                Self::merge_next_span_if_possible(&mut free_spans, index, new_end);
                return;
            } else if freed_span.offset < span.offset {
                if span.offset == freed_span.end(self.data.minimum_allocation_size) {
                    // The freed span can just extend the next entry
                    span.offset = freed_span.offset;
                    span.stripes += freed_span.stripes;
                    let new_end = span.end(self.data.minimum_allocation_size);
                    Self::merge_next_span_if_possible(&mut free_spans, index, new_end);
                    return;
                }

                // Cannot be merged, insert the span standalone.
                free_spans.insert(index, freed_span);
                return;
            }
        }

        // Freed span is at the end.
        free_spans.push(freed_span);
    }

    fn merge_next_span_if_possible(free_spans: &mut Vec<Span>, index: usize, new_end: usize) {
        if let Some(next_span) = free_spans.get(index + 1) {
            if next_span.offset == new_end {
                // The freed span can just extend the next entry
                free_spans[index].stripes += next_span.stripes;
                free_spans.remove(index + 1);
            }
        }
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        // SAFETY: This is the only location where dealloc is called, and drop
        // can only be called once. Because Data is held within an Arc,
        // individual instances of Slab will not cause deallocation, but only
        // the final one when the final Arc is dropped.
        unsafe {
            alloc::dealloc(self.bytes, self.layout);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Span {
    offset: usize,
    stripes: usize,
}

impl Span {
    const fn end(&self, minimum_allocation_size: usize) -> usize {
        self.offset + self.stripes * minimum_allocation_size
    }
}

#[test]
fn basic_tests() {
    let slab = Slab::new(64, Layout::array::<u8>(64).unwrap(), 16);

    // We should be able to allocate 4 blocks. Each should be rounded up to 16
    // bytes, our minimum allocation length.
    let alloc1 = slab.allocate(1).unwrap();
    let alloc2 = slab.allocate(2).unwrap();
    let alloc3 = slab.allocate(8).unwrap();
    let alloc4 = slab.allocate(16).unwrap();

    assert!(slab.allocate(16).is_none(), "slab should be full");

    // Free and reallocate
    drop(alloc1);
    let alloc1 = slab.allocate(16).unwrap();

    // Discontiguous frees
    drop(alloc2);
    drop(alloc4);
    drop(alloc3);

    // Now we should be able to allocate 48 bytes.
    let alloc2 = slab.allocate(48).unwrap();

    // Free everything
    drop(alloc2);
    drop(alloc1);

    // allocate the entire slab
    let alloc1 = slab.allocate(64).unwrap();
    drop(alloc1);
}
