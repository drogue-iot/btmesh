use core::alloc::{GlobalAlloc, Layout};

pub struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        unimplemented!();
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        unimplemented!();
    }
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator {};
