#[generic_tests::define]
mod relocatable_container {
    use elkodon_bb_container::{queue::RelocatableQueue, vec::Vec};
    use elkodon_bb_elementary::relocatable_container::RelocatableContainer;
    use elkodon_bb_lock_free::{
        mpmc::{container::Container, unique_index_set::UniqueIndexSet},
        spsc::{
            index_queue::RelocatableIndexQueue,
            safely_overflowing_index_queue::RelocatableSafelyOverflowingIndexQueue,
        },
    };
    use elkodon_bb_memory::{bump_allocator::BumpAllocator, memory::Memory};
    use elkodon_bb_testing::assert_that;
    use pin_init::PtrPinWith;

    #[test]
    fn init_acquires_less_or_equal_the_required_size_of_bytes<T: RelocatableContainer>() {
        const MAX_CAPACITY: usize = 128;

        for capacity in 1..MAX_CAPACITY {
            let memory = Box::pin_with(Memory::<131072, BumpAllocator>::new()).unwrap();

            let sut = unsafe { T::new_uninit(capacity) };
            let require_memory_size = T::memory_size(capacity);

            assert_that!(unsafe { sut.init(memory.allocator()) }, is_ok);
            assert_that!(memory.allocator().used_space(), le require_memory_size);
        }
    }

    #[test]
    fn init_acquires_less_or_equal_the_required_size_of_bytes_multiple_allocations<
        T: RelocatableContainer,
    >() {
        const MAX_CAPACITY: usize = 18;

        let memory = Box::pin_with(Memory::<131072, BumpAllocator>::new()).unwrap();

        let mut current_size = 0;
        for capacity in 1..MAX_CAPACITY {
            let sut = unsafe { T::new_uninit(capacity) };
            let require_memory_size = T::memory_size(capacity);

            assert_that!(unsafe { sut.init(memory.allocator()) }, is_ok);
            assert_that!(memory.allocator().used_space(), le current_size + require_memory_size);
            current_size = memory.allocator().used_space();
        }
    }

    #[instantiate_tests(<Vec<u64>>)]
    mod vec_u64 {}

    #[instantiate_tests(<Vec<u128>>)]
    mod vec_u128 {}

    #[instantiate_tests(<Vec<[u8; 123]>>)]
    mod vec_special {}

    #[instantiate_tests(<RelocatableQueue<u64>>)]
    mod queue_u64 {}

    #[instantiate_tests(<RelocatableQueue<u128>>)]
    mod queue_u128 {}

    #[instantiate_tests(<RelocatableQueue<[u8; 123]>>)]
    mod queue_special {}

    #[instantiate_tests(<Container<u64>>)]
    mod container_u64 {}

    #[instantiate_tests(<Container<u128>>)]
    mod container_u128 {}

    #[instantiate_tests(<Container<[u8; 123]>>)]
    mod container_special {}

    #[instantiate_tests(<UniqueIndexSet>)]
    mod unique_index_set {}

    #[instantiate_tests(<RelocatableIndexQueue>)]
    mod index_queue {}

    #[instantiate_tests(<RelocatableSafelyOverflowingIndexQueue>)]
    mod safely_overflowing_index_queue {}
}
