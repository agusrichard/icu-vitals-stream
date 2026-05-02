use crate::news2::News2Tier;
use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct PatientState {
    pub last_tier: Option<News2Tier>
}

pub type StateStore = DashMap<String, PatientState>;