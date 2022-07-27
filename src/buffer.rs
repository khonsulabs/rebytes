use std::{
    borrow::{Borrow, BorrowMut},
    io::Write,
    ops::{Deref, DerefMut},
};

use crate::{Allocation, Allocator};

#[derive(Debug, Default)]
pub struct Buffer {
    allocator: Option<Allocator>,
    allocation: Option<Allocation>,
    length: usize,
}

impl Buffer {
    #[must_use]
    pub const fn new(allocator: Allocator) -> Self {
        Self {
            allocator: Some(allocator),
            allocation: None,
            length: 0,
        }
    }

    fn allocate(&self, length: usize) -> Allocation {
        match &self.allocator {
            Some(allocator) => allocator.allocate(length),
            None => Allocation::global(length),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize, allocator: Allocator) -> Self {
        Self {
            allocation: Some(allocator.allocate(capacity)),
            allocator: Some(allocator),
            length: 0,
        }
    }
    #[must_use]
    pub fn with_len(length: usize, allocator: Allocator) -> Self {
        Self {
            allocation: Some(allocator.allocate(length)),
            allocator: Some(allocator),
            length,
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.length
    }

    pub fn set_len(&mut self, new_length: usize) {
        self.reserve_capacity(new_length);
        self.length = new_length;
    }

    pub fn clear(&mut self) {
        self.length = 0;
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.allocation.as_ref().map_or(0, Allocation::len)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        match &self.allocation {
            Some(allocation) => &allocation.as_slice()[..self.length],
            None => &[],
        }
    }

    #[must_use]
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        // SAFETY: The length and address of this area of memory have been
        // dedicated to this Allocation instance. The Rust borrow checker will
        // prevent any other attempt to borrow slices to this range of memory
        // while this exclusive reference is held.
        match &mut self.allocation {
            Some(allocation) => &mut allocation.as_slice_mut()[..self.length],
            None => &mut [],
        }
    }

    pub fn reserve_capacity(&mut self, total_capacity: usize) {
        if self.capacity() >= total_capacity {
            return;
        }

        let mut new_allocation = self.allocate(total_capacity);
        // Copy any existing data
        if self.length > 0 {
            new_allocation.as_slice_mut()[..self.length].copy_from_slice(self.as_slice());
        }
        self.allocation = Some(new_allocation);
    }

    pub fn extend_capacity_by(&mut self, additional_bytes: usize) {
        self.reserve_capacity(self.capacity() + additional_bytes);
    }

    pub fn preallocate_for(&mut self, additional_bytes: usize) {
        self.reserve_capacity(self.len() + additional_bytes);
    }

    pub fn push(&mut self, byte: u8) {
        if self.length == self.capacity() {
            self.preallocate_for(1);
        }
        let insert_at = self.length;
        self.length += 1;
        self.as_slice_mut()[insert_at] = byte;
    }

    pub fn extend<Bytes: IntoIterator<Item = u8>>(&mut self, bytes: Bytes) {
        let bytes = bytes.into_iter();
        let (estimated_size, _) = bytes.size_hint();
        self.preallocate_for(estimated_size);
        for byte in bytes {
            self.push(byte);
        }
    }

    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.preallocate_for(bytes.len());

        let insert_at = self.length;
        self.length += bytes.len();
        self.as_slice_mut()[insert_at..].copy_from_slice(bytes);
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_slice_mut()
    }
}

impl Borrow<[u8]> for Buffer {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl BorrowMut<[u8]> for Buffer {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.as_slice_mut()
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn basic_tests() {
    // Configure the allocator to allocate in 4-byte stripes.
    let allocator = Allocator::build()
        .minimum_allocation_size(4)
        .finish()
        .unwrap();
    let mut buffer = Buffer::new(allocator);
    assert!(buffer.is_empty());
    assert_eq!(buffer.as_slice(), &[]);
    assert_eq!(buffer.as_slice_mut(), &mut []);

    buffer.push(b'h');
    assert_eq!(buffer.as_slice(), b"h");

    buffer.extend([b'e', b'l', b'l', b'o']);
    assert_eq!(buffer.as_slice(), b"hello");

    buffer.extend_from_slice(b", world!");
    assert_eq!(buffer.as_slice(), b"hello, world!");
}
