//! SlotMap support for keyed fields based on their map types.
use crate::KeyedAccess;

impl<K: slotmap::Key, V> KeyedAccess for slotmap::SlotMap<K, V> {
    type Key = K;
    type Value = V;
    fn keyed(&self, key: Self::Key) -> &Self::Value {
        self.get(key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, key: Self::Key) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess for slotmap::DenseSlotMap<K, V> {
    type Key = K;
    type Value = V;
    fn keyed(&self, key: Self::Key) -> &Self::Value {
        self.get(key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, key: Self::Key) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess for slotmap::HopSlotMap<K, V> {
    type Key = K;
    type Value = V;
    fn keyed(&self, key: Self::Key) -> &Self::Value {
        self.get(key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, key: Self::Key) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess for slotmap::SecondaryMap<K, V> {
    type Key = K;
    type Value = V;
    fn keyed(&self, key: Self::Key) -> &Self::Value {
        self.get(key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, key: Self::Key) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess for slotmap::SparseSecondaryMap<K, V> {
    type Key = K;
    type Value = V;
    fn keyed(&self, key: Self::Key) -> &Self::Value {
        self.get(key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, key: Self::Key) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}
