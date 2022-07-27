use std::alloc::{self};

use crate::{allocation::Allocation, slabring::SlabRing};

#[derive(Debug, Clone)]
pub struct Allocator {
    slabs: SlabRing,
}

impl Allocator {
    pub fn build() -> Config {
        Config::default()
    }

    pub fn allocate(&self, length: usize) -> Allocation {
        if let Some(allocation) = self.slabs.allocate(length) {
            allocation
        } else {
            Allocation::global(length)
        }
    }
}

impl Default for Allocator {
    fn default() -> Self {
        Self::build().finish().unwrap()
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct Config {
    pub minimum_allocation_size: usize,
    pub maximum_allocation_size: usize,
    pub memory_limit: Option<usize>,
    pub slab_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            minimum_allocation_size: 16,
            maximum_allocation_size: 16 * 1024,
            memory_limit: None,
            slab_size: 256 * 1024,
        }
    }
}

impl Config {
    pub fn maximum_allocation_size(mut self, maximum_allocation_size: usize) -> Self {
        self.maximum_allocation_size = maximum_allocation_size;
        self
    }
    pub fn minimum_allocation_size(mut self, minimum_allocation_size: usize) -> Self {
        self.minimum_allocation_size = minimum_allocation_size;
        self
    }
    pub fn memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = Some(memory_limit);
        self
    }
    pub fn batch_allocation_size(mut self, batch_allocation_size: usize) -> Self {
        self.slab_size = batch_allocation_size;
        self
    }

    pub fn finish(mut self) -> Result<Allocator, alloc::LayoutError> {
        if self.slab_size < self.maximum_allocation_size {
            self.maximum_allocation_size = self.slab_size;
        }
        Ok(Allocator {
            slabs: SlabRing::new(self)?,
        })
    }
}
