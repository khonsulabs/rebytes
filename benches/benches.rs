use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use rebytes::{Allocator, Buffer};

fn benchmark_push_with_allocator(allocator: &Allocator, bench: &mut Bencher) {
    let mut buffer = Buffer::new(allocator.clone());

    bench.iter(|| buffer.push(1));
}

fn benchmark_4k_alloc_with_allocator(allocator: &Allocator, bench: &mut Bencher) {
    bench.iter(|| black_box(Buffer::with_capacity(4096, allocator.clone())));
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("push");
    group.bench_function("rebytes", |b| {
        benchmark_push_with_allocator(&Allocator::default(), b)
    });
    // group.bench_function("alloc", |b| {
    //     benchmark_push_with_allocator(
    //         &Allocator::build()
    //             .maximum_allocation_size(0)
    //             .finish()
    //             .unwrap(),
    //         b,
    //     )
    // });
    drop(group);
    let mut group = c.benchmark_group("4k-alloc");
    group.bench_function("rebytes", |b| {
        benchmark_4k_alloc_with_allocator(&Allocator::default(), b)
    });
    // group.bench_function("alloc", |b| {
    //     benchmark_4k_alloc_with_allocator(
    //         &Allocator::build()
    //             .maximum_allocation_size(0)
    //             .finish()
    //             .unwrap(),
    //         b,
    //     )
    // });
    // group.bench_function("vec-uninit", |b| {
    //     b.iter(|| black_box(Vec::<u8>::with_capacity(4096)))
    // });
    // group.bench_function("vec-init", |b| {
    //     b.iter(|| black_box(vec![0; 4096]));
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
