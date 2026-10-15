#[path = "../src/presentation/materials/upload_receipt.rs"]
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
