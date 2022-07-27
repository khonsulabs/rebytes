use rebytes::{Allocator, Buffer};

fn main() {
    let allocator = Allocator::build()
        .batch_allocation_size(64 * 1024)
        .finish()
        .unwrap();
    let mut threads = Vec::new();
    for _ in 0..=256 {
        threads.push(std::thread::spawn({
            let allocator = allocator.clone();
            move || worker_thread(&allocator)
        }));
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

fn worker_thread(allocator: &Allocator) {
    for _ in 0..100000 {
        let _ = Buffer::with_capacity(4096, allocator.clone());
    }
}
