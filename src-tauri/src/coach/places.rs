//! Per-match memo of the distinct `last_place` values the coach may cite
//! (V1.3 debt: `distinct_places` scanned tick_samples on every coach call).
//! Invalidated on re-analyze and delete; the demo's places never change
//! otherwise.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct PlacesCache {
    inner: Mutex<HashMap<i64, Vec<String>>>,
}

impl PlacesCache {
    pub(crate) fn get_or_load(
        &self,
        match_id: i64,
        load: impl FnOnce() -> Result<Vec<String>, String>,
    ) -> Result<Vec<String>, String> {
        if let Some(v) = self
            .inner
            .lock()
            .map_err(|_| "places cache poisoned")?
            .get(&match_id)
        {
            return Ok(v.clone());
        }
        let v = load()?;
        self.inner
            .lock()
            .map_err(|_| "places cache poisoned")?
            .insert(match_id, v.clone());
        Ok(v)
    }

    pub(crate) fn invalidate(&self, match_id: i64) {
        if let Ok(mut m) = self.inner.lock() {
            m.remove(&match_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlacesCache;
    use std::cell::Cell;

    #[test]
    fn loads_once_until_invalidated() {
        let c = PlacesCache::default();
        let loads = Cell::new(0);
        let load = || {
            loads.set(loads.get() + 1);
            Ok(vec!["BombsiteA".to_string()])
        };
        assert_eq!(c.get_or_load(8, load).unwrap(), vec!["BombsiteA"]);
        assert_eq!(c.get_or_load(8, load).unwrap(), vec!["BombsiteA"]);
        assert_eq!(loads.get(), 1);
        c.invalidate(8);
        c.get_or_load(8, load).unwrap();
        assert_eq!(loads.get(), 2);
    }

    #[test]
    fn a_failed_load_is_not_cached() {
        let c = PlacesCache::default();
        assert!(c.get_or_load(1, || Err("db".to_string())).is_err());
        assert_eq!(
            c.get_or_load(1, || Ok(vec![])).unwrap(),
            Vec::<String>::new()
        );
    }
}
