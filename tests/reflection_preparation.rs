use my_summer_slop::reference::reflections::{ProbeJob, ProbeSchedule, prepare_probe_face};

#[test]
fn runtime_prepares_one_face_and_no_faces_for_mips() {
    let mut schedule = ProbeSchedule::default();
    assert!(schedule.complete(ProbeJob::FirstMip).is_err());
    assert_eq!(schedule.job(), ProbeJob::Face(0));
    for job in (0..6)
        .map(ProbeJob::Face)
        .chain([ProbeJob::FirstMip, ProbeJob::RemainingMipsAndPublish])
    {
        assert_eq!(schedule.job(), job);
        let faces: Vec<_> = (0..6)
            .filter(|&face| prepare_probe_face(Some(job), face, false))
            .collect();
        match job {
            ProbeJob::Face(face) => assert_eq!(faces, [usize::from(face)]),
            _ => assert!(faces.is_empty()),
        }
        // A pending job retains its required face until acknowledgement.
        assert_eq!(schedule.job(), job);
        schedule.complete(job).unwrap();
    }
    assert_eq!(schedule.job(), ProbeJob::Face(0));
    assert!(schedule.published());
    assert!((0..6).all(|face| !prepare_probe_face(None, face, false)));
}

#[test]
fn startup_and_reload_prepare_all_faces_without_advancing_capture() {
    for job in [
        None,
        Some(ProbeJob::Face(4)),
        Some(ProbeJob::FirstMip),
        Some(ProbeJob::RemainingMipsAndPublish),
    ] {
        assert!((0..6).all(|face| prepare_probe_face(job, face, true)));
        assert!(!prepare_probe_face(job, 6, true));
    }
    let schedule = ProbeSchedule::default();
    assert_eq!(schedule.job(), ProbeJob::Face(0));
    assert!(!schedule.published());
}
