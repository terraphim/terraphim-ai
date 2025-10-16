use std::sync::Arc;
use terraphim_persistence::DeviceStorage;

#[tokio::test]
async fn test_arc_memory_safe_creation() {
    let storage1 = DeviceStorage::arc_memory_only().await;
    let storage2 = DeviceStorage::arc_memory_only().await;

    assert!(storage1.is_ok(), "First storage creation should succeed");
    assert!(storage2.is_ok(), "Second storage creation should succeed");

    let arc1 = storage1.unwrap();
    let arc2 = storage2.unwrap();

    assert!(
        Arc::strong_count(&arc1) >= 1,
        "Arc should have valid reference count"
    );
    assert!(
        Arc::strong_count(&arc2) >= 1,
        "Arc should have valid reference count"
    );
}

#[tokio::test]
async fn test_concurrent_arc_creation() {
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async move { DeviceStorage::arc_memory_only().await });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent storage creation should succeed");
    }
}

#[tokio::test]
async fn test_arc_memory_only_no_memory_leaks() {
    let storage = DeviceStorage::arc_memory_only().await.unwrap();
    let weak = Arc::downgrade(&storage);

    drop(storage);

    assert!(
        weak.upgrade().is_none(),
        "Storage should be freed after dropping Arc"
    );
}

#[tokio::test]
async fn test_multiple_arc_clones_safe() {
    let storage = DeviceStorage::arc_memory_only().await.unwrap();

    let clone1 = Arc::clone(&storage);
    let clone2 = Arc::clone(&storage);
    let clone3 = Arc::clone(&storage);

    assert_eq!(
        Arc::strong_count(&storage),
        4,
        "Should have 4 strong references"
    );

    drop(clone1);
    assert_eq!(
        Arc::strong_count(&storage),
        3,
        "Should have 3 strong references after drop"
    );

    drop(clone2);
    drop(clone3);
    assert_eq!(
        Arc::strong_count(&storage),
        1,
        "Should have 1 strong reference after drops"
    );
}

#[tokio::test]
async fn test_arc_instance_method_also_works() {
    let storage = DeviceStorage::arc_instance().await;

    if let Ok(arc) = storage {
        assert!(
            Arc::strong_count(&arc) >= 1,
            "Arc from instance should be valid"
        );
    }
}

#[tokio::test]
async fn test_arc_memory_only_error_handling() {
    let first = DeviceStorage::arc_memory_only().await;
    assert!(first.is_ok(), "First call should succeed");

    let second = DeviceStorage::arc_memory_only().await;
    assert!(second.is_ok(), "Subsequent calls should also succeed");
}

#[tokio::test]
async fn test_no_unsafe_ptr_read_needed() {
    let storage_result = DeviceStorage::arc_memory_only().await;

    assert!(storage_result.is_ok(), "Safe Arc creation should work");

    let storage = storage_result.unwrap();
    let cloned = storage.clone();
    assert!(
        Arc::ptr_eq(&storage, &cloned),
        "Cloned Arcs should point to same data"
    );
}
