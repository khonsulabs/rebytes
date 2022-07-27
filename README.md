# rebytes

Reusable global allocator and Vec-like type that uses it. Allocates larger
blocks of memory from the OS and initializes them once. Individual regions of
memory are able to be allocated, and each allocation keeps a reference to the
region of memory it was allocated from. Upon dropping the allocation, the region
is marked as free and able to be allocated from again.

WIP, and not necessarily a clear win: Less syscalls, but current design can have
lock contention in multithreaded operation leading to similar performance. Using
a standard lock-free allocator approach should be possible to alleviate this
problem.
