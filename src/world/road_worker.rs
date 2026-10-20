//! Identity owned by one road request in one world session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadJobId {
    pub session: u64,
    pub revision: u64,
    pub owner: u32,
    pub sequence: u64,
}
