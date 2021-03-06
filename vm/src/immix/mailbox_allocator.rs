//! Allocator for process mailboxes
//!
//! Each mailbox has its own allocator and its own heap. Incoming messages are
//! copied into this heap. When a message is received its copied from the
//! mailbox heap to the process local heap.

use std::ops::Drop;

use immix::copy_object::CopyObject;
use immix::bucket::{Bucket, MAILBOX};
use immix::block::BLOCK_SIZE;
use immix::global_allocator::RcGlobalAllocator;

use object::Object;
use object_pointer::ObjectPointer;

pub struct MailboxAllocator {
    global_allocator: RcGlobalAllocator,
    pub bucket: Bucket,

    /// The number of blocks that have been allocated since the last garbage
    /// collection cycle.
    pub block_allocations: usize,

    /// The number of blocks that can be allocated before garbage collection
    /// is triggered.
    pub block_allocation_threshold: usize,
}

impl MailboxAllocator {
    pub fn new(global_allocator: RcGlobalAllocator) -> Self {
        MailboxAllocator {
            global_allocator: global_allocator,
            bucket: Bucket::with_age(MAILBOX),
            block_allocations: 0,
            block_allocation_threshold: (1 * 1024 * 1024) / BLOCK_SIZE,
        }
    }

    pub fn allocate(&mut self, object: Object) -> ObjectPointer {
        let (new_block, pointer) = self.bucket
            .allocate(&self.global_allocator, object);

        if new_block {
            self.block_allocations += 1;
        }

        pointer
    }

    /// Prepares a garbage collection cycle, returns true if objects have to be
    /// moved around.
    pub fn prepare_for_collection(&mut self) -> bool {
        self.bucket.prepare_for_collection()
    }

    /// Returns unused blocks to the global allocator.
    pub fn reclaim_blocks(&mut self) {
        for block in self.bucket.reclaim_blocks() {
            self.global_allocator.add_block(block);
        }
    }

    pub fn allocation_threshold_exceeded(&self) -> bool {
        self.block_allocations >= self.block_allocation_threshold
    }

    /// Increments the allocation threshold by the given factor.
    pub fn increment_threshold(&mut self, factor: f64) {
        let threshold = (self.block_allocation_threshold as f64 * factor).ceil();

        self.block_allocation_threshold = threshold as usize;
    }
}

impl CopyObject for MailboxAllocator {
    fn allocate_copy(&mut self, object: Object) -> ObjectPointer {
        self.allocate(object)
    }
}

impl Drop for MailboxAllocator {
    fn drop(&mut self) {
        for mut block in self.bucket.blocks.drain(0..) {
            block.reset();
            self.global_allocator.add_block(block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use immix::global_allocator::GlobalAllocator;
    use immix::local_allocator::LocalAllocator;
    use immix::copy_object::CopyObject;
    use object::Object;
    use object_value;

    fn mailbox_allocator() -> MailboxAllocator {
        let global = GlobalAllocator::without_preallocated_blocks();

        MailboxAllocator::new(global)
    }

    #[test]
    fn test_allocate() {
        let mut alloc = mailbox_allocator();
        let pointer = alloc.allocate(Object::new(object_value::none()));

        assert!(pointer.is_mailbox());
        assert!(pointer.get().value.is_none());
    }

    #[test]
    fn test_copy_object() {
        let mut mbox_alloc = mailbox_allocator();
        let global_alloc = mbox_alloc.global_allocator.clone();
        let mut local_alloc = LocalAllocator::new(global_alloc);

        let original =
            local_alloc.allocate_without_prototype(object_value::integer(5));

        let copy = mbox_alloc.copy_object(original);

        assert!(copy.is_mailbox());
        assert!(copy.get().value.is_integer());
    }

    #[test]
    fn test_drop() {
        let mut alloc = mailbox_allocator();
        let global_alloc = alloc.global_allocator.clone();

        alloc.allocate(Object::new(object_value::none()));

        drop(alloc);

        assert_eq!(lock!(global_alloc.blocks).len(), 1);
    }

    #[test]
    fn test_allocation_threshold_exceeded() {
        let mut alloc = mailbox_allocator();

        alloc.block_allocation_threshold = 1;

        assert_eq!(alloc.allocation_threshold_exceeded(), false);

        alloc.block_allocations = 1;

        assert!(alloc.allocation_threshold_exceeded());
    }

    #[test]
    fn test_increment_threshold() {
        let mut alloc = mailbox_allocator();

        alloc.block_allocation_threshold = 1;

        alloc.increment_threshold(1.5);

        assert_eq!(alloc.block_allocation_threshold, 2);
    }
}
