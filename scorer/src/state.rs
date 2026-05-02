use crate::news2::News2Tier;
use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct PatientState {
    pub last_tier: Option<News2Tier>
}

pub type StateStore = DashMap<String, PatientState>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::news2::News2Tier;

    #[test]
    fn new_patient_has_no_last_tier() {
        let store: StateStore = DashMap::new();
        let entry = store.entry("p1".to_string()).or_default();
        assert!(entry.last_tier.is_none());
    }

    #[test]
    fn last_tier_is_retained_after_update() {
        let store: StateStore = DashMap::new();
        {
            let mut entry = store.entry("p1".to_string()).or_default();
            entry.last_tier = Some(News2Tier::Medium);
        }
        assert_eq!(store.get("p1").unwrap().last_tier, Some(News2Tier::Medium));
    }

    #[test]
    fn patients_have_isolated_state() {
        let store: StateStore = DashMap::new();
        {
            let mut entry = store.entry("p1".to_string()).or_default();
            entry.last_tier = Some(News2Tier::High);
        }
        {
            let mut entry = store.entry("p2".to_string()).or_default();
            entry.last_tier = Some(News2Tier::Low);
        }
        assert_eq!(store.get("p1").unwrap().last_tier, Some(News2Tier::High));
        assert_eq!(store.get("p2").unwrap().last_tier, Some(News2Tier::Low));
    }
}