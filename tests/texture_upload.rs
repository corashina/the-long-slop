#[path = "../src/world/upload_receipt.rs"]
mod upload_receipt;

#[test]
fn texture_cpu_reservation_lasts_until_gpu_completion_across_forks() {
    let receipt = upload_receipt::UploadReceipt::default();
    let reservation = 48 * 1024 * 1024;
    assert_eq!(receipt.cpu_bytes(reservation, false), 0);
    assert_eq!(receipt.cpu_bytes(reservation, true), reservation);
    let fork = receipt.clone();
    assert!(fork.register());
    assert!(!receipt.register());
    assert_eq!(receipt.cpu_bytes(reservation, true), reservation);
    fork.complete();
    assert_eq!(receipt.cpu_bytes(reservation, true), 0);
    assert_eq!(fork.cpu_bytes(reservation, true), 0);
    assert!(!receipt.register());
}

use my_summer_slop::world::texture_batch_ownership::*;
fn id(request: u64) -> BatchIdentity {
    BatchIdentity {
        session: 1,
        request,
    }
}
#[test]
fn pair_admission_and_promotion() {
    let mut queue = BatchQueue::default();
    assert_eq!(
        queue.admit([(id(1), 9), (id(2), 8)], 16, false),
        Err(Admission::Permanent)
    );
    assert_eq!(
        queue.admit([(id(1), 9), (id(2), 8)], 16, true),
        Err(Admission::Wait)
    );
    queue.admit([(id(1), 9), (id(2), 8)], 17, false).unwrap();
    assert_eq!(queue.start(1), vec![id(1)]);
    assert_eq!(queue.start(1), vec![id(2)]);
    assert!(queue.start(2).is_empty());
    assert_eq!(queue.jobs(), 2);
    queue.ready(id(2));
    assert!(!queue.submit(id(2)));
    queue.ready(id(1));
    assert!(queue.submit(id(1)));
    assert!(!queue.acknowledge(BatchIdentity {
        session: 2,
        request: 1
    }));
    assert!(queue.acknowledge(id(1)));
    assert_eq!(queue.release(id(1)), Some(9));
    assert!(queue.submit(id(2)));
}
#[test]
fn cancellation_keeps_loading_and_submitted_owners() {
    let mut queue = BatchQueue::default();
    queue
        .admit([(id(1), 10), (id(2), 20)], STAGING_LIMIT, false)
        .unwrap();
    queue.start(2);
    queue.ready(id(1));
    queue.submit(id(1));
    queue.cancel(1);
    assert_eq!(queue.bytes(), 30);
    assert_eq!(queue.jobs(), 1);
    assert_eq!(queue.release(id(1)), None);
    assert_eq!(queue.release(id(2)), None);
    queue.ready(id(2));
    assert_eq!(queue.release(id(2)), Some(20));
    queue.acknowledge(id(1));
    assert_eq!(queue.release(id(1)), Some(10));
    queue.admit([(id(3), 5)], STAGING_LIMIT, false).unwrap();
    assert!(!queue.acknowledge(id(1)));
    assert_eq!(queue.bytes(), 5);
}
#[test]
fn cap_is_permanent_even_when_other_work_can_finish() {
    let mut queue = BatchQueue::default();
    assert_eq!(
        queue.admit([(id(1), STAGING_LIMIT + 1)], usize::MAX, true),
        Err(Admission::Permanent)
    );
}

#[test]
fn shared_storage_survives_cancelled_session_until_last_owner() {
    let mut queue = BatchQueue::default();
    let other = BatchIdentity {
        session: 2,
        request: 2,
    };
    queue.admit([(id(1), 100), (other, 4)], 104, false).unwrap();
    queue.share(id(1), other, 96);
    queue.start(2);
    queue.cancel(1);
    assert_eq!(queue.bytes(), 104);
    queue.ready(id(1));
    assert_eq!(queue.release(id(1)), Some(4));
    assert_eq!(queue.bytes(), 100);
    assert!(!queue.owner(other).unwrap().cancelled);
    queue.ready(other);
    assert!(queue.submit(other));
    assert!(!queue.acknowledge(id(1)));
    assert_eq!(queue.bytes(), 100);
    queue.acknowledge(other);
    assert_eq!(queue.release(other), Some(100));
    assert_eq!(queue.bytes(), 0);
}
#[test]
#[ignore = "requires installed texture package"]
fn installed_pairs_fit_unchanged_staging_limit() {
    use my_summer_slop::reference::LightingTextureRecord;
    let root = std::path::PathBuf::from(std::env::var("TEXTURE_BATCH_PACKAGE").unwrap());
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("reference/manifest.json")).unwrap()).unwrap();
    let mut reservations = Vec::new();
    let mut textures = 0;
    for group in [&manifest["lighting"]["textures"], &manifest["world"]["material_textures"], &manifest["world"]["terrain"]["textures"]] {
        for value in group.as_array().unwrap() {
            let texture: LightingTextureRecord = serde_json::from_value(value.clone()).unwrap();
            for batch in texture.batches {
                assert!(batch.decoded_bytes <= 1024 * 1024);
                assert!(batch.region_count <= 32);
                reservations.push(reservation(&batch).unwrap());
            }
            textures += 1;
        }
    }
    let maximum = reservations.iter().copied().max().unwrap();
    assert!(maximum * 2 <= STAGING_LIMIT);
    assert_eq!(textures, 166);
    assert_eq!(reservations.len(), 503);
    println!("textures={textures}, batches={}, maximum_owner={maximum}, maximum_pair={}, cap={STAGING_LIMIT}", reservations.len(), maximum*2);
}
