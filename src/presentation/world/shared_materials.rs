//! Scene-local immutable material bindings. Each admitted renderer owns one user.
use crate::reference::{AlphaMode, AssetId, MaterialRecord, ShaderFamily, SourceId};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MaterialIdentity {
    Primary(AssetId, u32),
    Specular(SourceId),
    /// Only GPU prop rows share impostors. Rows hold the fade and the source position.
    Impostor(SourceId),
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SharedMaterialKey {
    pub session: u64,
    pub material: MaterialIdentity,
    pub layer: u8,
    pub receive: bool,
    pub reflected: bool,
    /// GPU prop rows use their own material. Its pipeline reads the row buffers.
    pub gpu: bool,
}
impl SharedMaterialKey {
    fn heap_bytes(&self) -> usize {
        match &self.material {
            MaterialIdentity::Specular(source) | MaterialIdentity::Impostor(source) => {
                source.file.capacity()
            }
            MaterialIdentity::Primary(..) => 0,
        }
    }
}

pub(crate) fn shareable_primary(record: &MaterialRecord) -> bool {
    matches!(
        record.shader_family,
        ShaderFamily::Standard | ShaderFamily::UnlitColor
    ) && matches!(record.state.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask)
        && record.state.z_write
        && record.state.z_test == 4
        && record.state.src_blend == 1
        && record.state.dst_blend == 0
}

pub(crate) struct SharedMaterials<T> {
    entries: HashMap<SharedMaterialKey, (T, usize)>,
    heap_bytes: usize,
}
impl<T> Default for SharedMaterials<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            heap_bytes: 0,
        }
    }
}
impl<T> Clone for SharedMaterials<T> {
    fn clone(&self) -> Self {
        // Save candidates own their bindings and GPU preparation state independently.
        Self::default()
    }
}
impl<T: Clone> SharedMaterials<T> {
    pub fn acquire(&mut self, key: &SharedMaterialKey) -> Option<T> {
        let (binding, users) = self.entries.get_mut(key)?;
        *users += 1;
        Some(binding.clone())
    }
    pub fn insert(&mut self, key: SharedMaterialKey, binding: T) {
        self.heap_bytes += key.heap_bytes();
        assert!(self.entries.insert(key, (binding, 1)).is_none());
    }
    fn remove(&mut self, key: &SharedMaterialKey) -> T {
        let (key, (binding, _)) = self.entries.remove_entry(key).unwrap();
        self.heap_bytes -= key.heap_bytes();
        binding
    }
    pub fn make_private(&mut self, key: &SharedMaterialKey, copy: impl FnOnce(&T) -> T) -> T {
        if self.entries[key].1 == 1 {
            return self.remove(key);
        }
        let (binding, users) = self.entries.get_mut(key).unwrap();
        *users -= 1;
        copy(binding)
    }
    /// The last user takes the existing handles. Other users require a private copy.
    pub fn release(&mut self, key: &SharedMaterialKey) -> bool {
        let (_, users) = self
            .entries
            .get_mut(key)
            .expect("owned shared material binding");
        *users -= 1;
        if *users == 0 {
            self.remove(key);
            true
        } else {
            false
        }
    }
    pub fn bytes(&self) -> usize {
        self.entries.capacity() * (std::mem::size_of::<(SharedMaterialKey, (T, usize))>() + 1)
            + self.heap_bytes
    }
    pub fn reset(&mut self) {
        assert!(
            self.entries.is_empty(),
            "Scene bindings must retire before their cache"
        );
        self.entries = HashMap::new();
        self.heap_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> SharedMaterialKey {
        SharedMaterialKey {
            session: 1,
            material: MaterialIdentity::Primary(AssetId(7), 3),
            layer: 0,
            receive: true,
            reflected: false,
            gpu: false,
        }
    }
    #[test]
    fn sharing_retirement_and_private_mutation_keep_other_users() {
        let mut cache = SharedMaterials::default();
        // Stand-in handles address independent visible state.
        let mut assets = vec![("white", 1.0)];
        cache.insert(key(), 0);
        assert_eq!(cache.acquire(&key()), Some(0));
        let private = cache.make_private(&key(), |&visible| {
            assets.push(assets[visible]);
            1
        });
        assets[private] = ("red", 0.5);
        assert_eq!(assets[0], ("white", 1.0));
        // The sole remaining user keeps its original handle.
        let last = cache.make_private(&key(), |_| panic!("The sole user must not copy handles"));
        assert_eq!(last, 0);
        assert!(cache.acquire(&key()).is_none());
        cache.insert(key(), 30);
        assert_eq!(cache.acquire(&key()), Some(30));
        assert!(!cache.release(&key()));
        assert!(cache.release(&key()));
        assert!(cache.acquire(&key()).is_none());
        cache.reset();
        assert!(cache.entries.is_empty());
        assert_eq!(cache.bytes(), 0);
    }
    #[test]
    fn complete_keys_and_scene_owners_do_not_alias() {
        let original = key();
        let mut keys = vec![original.clone()];
        let mut changed = original.clone();
        changed.layer = 1;
        keys.push(changed);
        let mut changed = original.clone();
        changed.receive = false;
        keys.push(changed);
        let mut changed = original.clone();
        changed.reflected = true;
        keys.push(changed);
        let mut changed = original.clone();
        changed.gpu = true;
        keys.push(changed);
        let mut changed = original.clone();
        changed.session = 2;
        keys.push(changed);
        let mut changed = original.clone();
        changed.material = MaterialIdentity::Primary(AssetId(7), 4);
        keys.push(changed);
        let mut changed = original.clone();
        changed.material = MaterialIdentity::Specular(SourceId {
            file: "world".into(),
            path_id: 7,
        });
        keys.push(changed);
        let mut cache = SharedMaterials::default();
        for (index, key) in keys.iter().enumerate() {
            cache.insert(key.clone(), index);
        }
        for (index, key) in keys.iter().enumerate() {
            assert_eq!(cache.acquire(key), Some(index));
        }
        let mut candidate = cache.clone();
        assert!(candidate.acquire(&original).is_none());
        candidate.insert(original.clone(), 100);
        assert_eq!(candidate.acquire(&original), Some(100));
        assert_eq!(cache.acquire(&original), Some(0));
    }
    #[test]
    fn only_ordinary_opaque_and_cutout_primary_states_share() {
        use crate::reference::{MaterialState, ShaderProperties};
        let record = MaterialRecord {
            asset: AssetId(7),
            gltf_material: 3,
            source: SourceId {
                file: "world".into(),
                path_id: 7,
            },
            shader_family: ShaderFamily::Standard,
            enable_instancing: true,
            color: [1.; 4],
            condition: None,
            keywords: vec![],
            state: MaterialState {
                alpha_mode: AlphaMode::Opaque,
                cull: 2,
                z_write: true,
                z_test: 4,
                src_blend: 1,
                dst_blend: 0,
                render_queue: 2000,
            },
            properties: ShaderProperties {
                floats: Default::default(),
                colors: Default::default(),
                textures: vec![],
                defaults: Default::default(),
            },
        };
        assert!(shareable_primary(&record));
        let mut cutout = record.clone();
        cutout.state.alpha_mode = AlphaMode::Mask;
        assert!(shareable_primary(&cutout));
        let mut changed = record.clone();
        changed.state.alpha_mode = AlphaMode::Blend;
        assert!(!shareable_primary(&changed));
        let mut changed = record.clone();
        changed.state.z_write = false;
        assert!(!shareable_primary(&changed));
        let mut changed = record.clone();
        changed.state.z_test = 8;
        assert!(!shareable_primary(&changed));
        let mut changed = record.clone();
        changed.state.dst_blend = 1;
        assert!(!shareable_primary(&changed));
        let mut changed = record.clone();
        changed.shader_family = ShaderFamily::StainedGlass;
        assert!(!shareable_primary(&changed));
    }
}
