use std::{
    alloc::{self, Layout},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use parking_lot::{RwLock, RwLockReadGuard};

use crate::{slab::Slab, Allocation, Config};

#[derive(Clone, Debug)]
pub struct SlabRing {
    data: Arc<Data>,
}

#[derive(Debug)]
struct Data {
    entries: RwLock<Vec<Slab>>,
    cycle: AtomicUsize,
    layout: Layout,
    config: Config,
}

impl SlabRing {
    pub fn new(config: Config) -> Result<Self, alloc::LayoutError> {
        let layout = Layout::array::<u8>(config.slab_size)?;
        Ok(Self {
            data: Arc::new(Data {
                entries: RwLock::default(),
                cycle: AtomicUsize::default(),
                layout,
                config,
            }),
        })
    }

    pub fn allocate(&self, length: usize) -> Option<Allocation> {
        if length < self.data.config.maximum_allocation_size {
            // Try to allocate in all existing slabs.
            for slab in self.iter() {
                if let Some(allocation) = slab.allocate(length) {
                    return Some(allocation);
                }
            }

            // No current slabs had any space available. Allocate a new slab if
            // we aren't at our memory limit.
            loop {
                let new_slab = self.new_slab();
                if let Some(new_slab) = new_slab {
                    if let Some(allocation) = new_slab.allocate(length) {
                        return Some(allocation);
                    }
                } else {
                    // At the memory limit, fall back to the global allocator
                    break;
                }
            }
        }

        None
    }

    pub fn new_slab(&self) -> Option<Slab> {
        let mut entries = self.data.entries.write();
        if self.data.config.memory_limit.map_or(true, |limit| {
            entries.len() * self.data.config.slab_size < limit
        }) {
            let slab = Slab::new(
                self.data.config.slab_size,
                self.data.layout,
                self.data.config.minimum_allocation_size,
            );
            entries.push(slab.clone());
            Some(slab)
        } else {
            None
        }
    }

    pub fn iter(&self) -> SlabRingIter<'_> {
        let entries = self.data.entries.read();
        let start = if entries.is_empty() {
            0
        } else {
            // Start iterating at the previous slot
            loop {
                let current_cycle = self.data.cycle.load(Ordering::Acquire);
                let next_cycle = current_cycle.checked_sub(1).unwrap_or(entries.len() - 1);
                if self
                    .data
                    .cycle
                    .compare_exchange(
                        current_cycle,
                        next_cycle,
                        Ordering::Release,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    break next_cycle;
                }
            }
        };
        SlabRingIter {
            entries,
            start,
            position: None,
        }
    }
}

pub struct SlabRingIter<'a> {
    entries: RwLockReadGuard<'a, Vec<Slab>>,
    start: usize,
    position: Option<usize>,
}

impl<'a> Iterator for SlabRingIter<'a> {
    type Item = Slab;

    fn next(&mut self) -> Option<Self::Item> {
        match self.position {
            Some(position) => {
                // Cycle through the entries
                let next = match position + 1 {
                    next if next == self.entries.len() => 0,
                    next => next,
                };
                if next == self.start {
                    // Full cycle
                    None
                } else {
                    self.position = Some(next);
                    Some(self.entries[next].clone())
                }
            }
            None => {
                if self.entries.is_empty() {
                    None
                } else {
                    self.position = Some(self.start);
                    Some(self.entries[self.start].clone())
                }
            }
        }
    }
}
