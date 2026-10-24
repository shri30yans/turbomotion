//! A small bounded cache for rendered layers, keyed by the parameters that produced them. Eviction is first-in
//! first-out: layers are rebuilt cheaply on a miss, so recency tracking would only add cost.

use std::{cell::RefCell, collections::{HashMap, VecDeque}, hash::Hash, rc::Rc};

pub struct Cache<K, V> {
    cap: usize,
    inner: RefCell<Inner<K, V>>,
}

struct Inner<K, V> {
    map: HashMap<K, Rc<V>>,
    order: VecDeque<K>,
}

impl<K: Hash + Eq + Clone, V> Cache<K, V> {
    pub fn new(cap: usize) -> Self {
        Self { cap, inner: RefCell::new(Inner { map: HashMap::new(), order: VecDeque::new() }) }
    }

    /// The cached value for `key`, building it with `make` on a miss.
    pub fn get_or_try_insert<E>(&self, key: K, make: impl FnOnce() -> Result<V, E>) -> Result<Rc<V>, E> {
        if let Some(hit) = self.inner.borrow().map.get(&key) {
            return Ok(hit.clone());
        }
        let value = Rc::new(make()?);
        let mut inner = self.inner.borrow_mut();
        if inner.map.len() >= self.cap {
            if let Some(old) = inner.order.pop_front() {
                inner.map.remove(&old);
            }
        }
        inner.order.push_back(key.clone());
        inner.map.insert(key, value.clone());
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_the_oldest_and_builds_once() {
        let cache: Cache<u32, u32> = Cache::new(2);
        let mut builds = 0;
        for k in [1, 1, 2, 3, 1] {
            cache.get_or_try_insert::<()>(k, || {
                builds += 1;
                Ok(k * 10)
            })
            .unwrap();
        }
        assert_eq!(builds, 4); // 1, 2, 3 built; 1 evicted by 3 and rebuilt
    }
}
