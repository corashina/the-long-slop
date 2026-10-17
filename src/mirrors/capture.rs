//! Shared admission seam for mirror preparation and draw insertion.
pub(crate) const TICKET_PIPELINES: usize = 32;
pub(crate) struct SourceProof {
    pub(crate) id: usize,
    pub(crate) isolated: bool,
    pub(crate) cancelled: bool,
}
#[derive(Default)]
pub(crate) struct GeneratedProof {
    pub(crate) ids: Vec<usize>,
}
impl GeneratedProof {
    pub(crate) fn include_ticket(
        &mut self,
        current: u64,
        session: u64,
        ready: bool,
        completed: bool,
        live: bool,
        ids: impl Iterator<Item = SourceProof>,
    ) {
        if current == session && ready && completed && live {
            for source in ids {
                if source.isolated || source.cancelled {
                    continue;
                }
                assert!(self.ids.len() < self.ids.capacity());
                self.ids.push(source.id);
            }
        }
    }
    pub(crate) fn finish(&mut self) {
        self.ids.sort_unstable();
        self.ids.dedup();
    }
    pub(crate) fn contains(&self, id: usize) -> bool {
        self.ids.binary_search(&id).is_ok()
    }
}
pub(crate) fn proof_capacity(capacity: usize, tickets: usize) -> usize {
    let required = tickets * TICKET_PIPELINES;
    if required <= capacity {
        capacity
    } else {
        required.max(capacity * 2)
    }
}
pub(crate) fn admitted(imported: bool, generated: bool, isolated: bool) -> bool {
    (imported || generated) && (!isolated || generated)
}
/// Owner screens must map to their disabled pipeline before either admission path.
pub(crate) fn authorize<T: Copy>(
    original: T,
    owner_screen: bool,
    disabled: Option<T>,
    admitted: impl FnOnce(T) -> bool,
) -> Option<T> {
    let selected = if owner_screen { disabled? } else { original };
    admitted(selected).then_some(selected)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sealed_imports_accept_later_generated_proof_for_prepare_and_queue() {
        let imported = [1, 2];
        let mut proof = GeneratedProof {
            ids: Vec::with_capacity(TICKET_PIPELINES),
        };
        let capacity = proof.ids.capacity();
        for (session, ready, completed, live, isolated, cancelled, allowed) in [
            (7, true, true, true, false, false, true),
            (7, true, false, true, false, false, false),
            (7, false, true, true, false, false, false),
            (6, true, true, true, false, false, false),
            (7, true, true, false, false, false, false),
            (7, true, true, true, true, false, false),
            (7, true, true, true, false, true, false),
        ] {
            proof.ids.clear();
            proof.include_ticket(
                7,
                session,
                ready,
                completed,
                live,
                [SourceProof {
                    id: 263,
                    isolated,
                    cancelled,
                }]
                .into_iter(),
            );
            proof.finish();
            let eligible = |id| admitted(imported.contains(&id), proof.contains(id), false);
            let prepared = authorize(263, false, None, eligible);
            let queued = prepared.and_then(|id| authorize(id, false, None, eligible));
            assert_eq!(prepared, allowed.then_some(263));
            assert_eq!(queued, prepared);
            assert_eq!(authorize(999, false, None, eligible), None);
            assert!(!admitted(true, false, true));
            assert_eq!(admitted(false, proof.contains(263), true), allowed);
            assert!(!admitted(false, false, false));
            assert_eq!(authorize(263, true, None, eligible), None);
            assert_eq!(authorize(263, true, Some(2), eligible), Some(2));
            assert_eq!(authorize(2, true, Some(2), eligible), Some(2));
            assert_eq!(proof.ids.capacity(), capacity);
        }
        proof.ids.clear();
        proof.finish();
        assert!(!proof.contains(263));
    }
    #[test]
    fn proof_slots_follow_admitted_ticket_capacity() {
        let mut capacity = 0;
        for tickets in 1..100 {
            let next = proof_capacity(capacity, tickets);
            assert!(next >= tickets * TICKET_PIPELINES);
            assert!(next == capacity || next >= capacity * 2);
            capacity = next;
        }
        assert!(capacity > 2048);
    }
}
