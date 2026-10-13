//! Capacity-based source graph accounting. Immutable inputs are charged once per shared graph.
use super::*;
use std::{
    collections::BTreeMap,
    mem::{align_of, size_of},
};

pub(crate) trait RetainedBytes {
    fn heap_bytes(&self) -> usize;
}
macro_rules! inline {($($kind:ty),*)=>{$(impl RetainedBytes for $kind {fn heap_bytes(&self)->usize {0}})*};}
inline!(
    bool,
    u8,
    u32,
    i32,
    i64,
    f32,
    f64,
    AssetId,
    Transform,
    WorldPrototypeRole
);
impl RetainedBytes for String {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}
impl<T: RetainedBytes> RetainedBytes for Vec<T> {
    fn heap_bytes(&self) -> usize {
        self.capacity() * size_of::<T>() + self.iter().map(RetainedBytes::heap_bytes).sum::<usize>()
    }
}
impl<T: RetainedBytes, const N: usize> RetainedBytes for [T; N] {
    fn heap_bytes(&self) -> usize {
        self.iter().map(RetainedBytes::heap_bytes).sum()
    }
}
impl<T: RetainedBytes> RetainedBytes for Option<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, RetainedBytes::heap_bytes)
    }
}
impl<K: RetainedBytes, V: RetainedBytes> RetainedBytes for BTreeMap<K, V> {
    fn heap_bytes(&self) -> usize {
        // Rust's B-tree has eleven entry slots and twelve child slots. Every
        // non-root node has at least five entries. Charge every possible node
        // as an internal node, including alignment padding. This is an upper
        // bound on container payload, not an allocator residency measurement.
        let nodes = if self.is_empty() {
            0
        } else {
            1 + (self.len() - 1) / 5
        };
        let node = 4 * size_of::<usize>()
            + 11 * (size_of::<K>() + size_of::<V>())
            + 12 * size_of::<usize>()
            + 4 * align_of::<K>().max(align_of::<V>());
        nodes * node
            + self
                .iter()
                .map(|(k, v)| k.heap_bytes() + v.heap_bytes())
                .sum::<usize>()
    }
}
macro_rules! fields {($kind:ty:$($field:ident),*)=>{impl RetainedBytes for $kind {fn heap_bytes(&self)->usize {0$(+self.$field.heap_bytes())*}}};}
fields!(SourceId:file);
fields!(SourceHash:file,sha256);
fields!(LibraryProvenance:manifest_sha256,catalog_sha256,source_manifest_sha256);
fields!(super::super::rendering::ResourceReference:source,path,sha256);
fields!(super::super::rendering::ShaderReference:compiled_data,source,path,sha256,name);
fields!(super::super::rendering::LightingTextureRecord:mip_data,source_data,source,path,sha256,sample_channel);
fields!(super::super::rendering::TextureLink:property);
impl RetainedBytes for WorldValue {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Text(value) => value.heap_bytes(),
            Self::Link(value) => value.heap_bytes(),
            Self::Array(value) => value.heap_bytes(),
            Self::Record(value) => value.heap_bytes(),
            _ => 0,
        }
    }
}
impl RetainedBytes for ColliderGeometry {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Convex { points } => points.heap_bytes(),
            Self::Trimesh { points, triangles } => points.heap_bytes() + triangles.heap_bytes(),
            _ => 0,
        }
    }
}
impl WorldReference {
    /// Decoded vector/string capacities plus a conservative B-tree container bound.
    /// Allocator metadata and process residency require the separate memory acceptance.
    pub fn retained_cpu_bytes(&self) -> usize {
        size_of::<Self>() + 2 * size_of::<usize>() + self.heap_bytes()
    }
}
fields!(WorldReference:sample_payload_sha256,managed_sha256,library,settings_context,generation_frame,distances,source_hashes,road,terrain,buildings,catalog,part_scope_slots,generators,startup,startup_holes,collision_rules,grass_displacement,lod,initial_materials,prototypes,records,receipts,exclusions,material_textures);
fields!(WorldPrototypeReference:source,role,asset,nodes,controls,excluded_actions,deferred_materials,skins,bodies,colliders,prefab_pose);
fields!(WorldPrototypeNode:source,name,parent,active,renderer_enabled,layer,transform,gltf_node);
fields!(WorldSkinReference:source,mesh,node,bones,bind_poses,positions,normals,tangents,uv,joints,weights,triangles);
fields!(WorldColliderReference:body,source,node,enabled,trigger,material,deformed_skin,source_empty_mesh,geometry);
fields!(WorldSourceReceipt:source,sha256);
fields!(WorldSourceLink:field,target);
fields!(WorldSourceRecord:source,class_name,fields,links);
fields!(WorldExclusion:source,field,target,reason,catalog);
fields!(WorldStartupReference:group,source,pose);
fields!(WorldCollisionRules:source,record_sha256,layers);
fields!(WorldStartupHole:component,filter,mesh,mesh_sha256,vertices);
fields!(WorldNoiseReference:amplitude,frequency,lacunarity,octaves,persistence);
fields!(WorldRoadReference:source,prototypes,length,road_bone_num,road_bone_length,road_segment_length,road_above_terrain,width,scale,xmax_angle,ymax_angle,start_straight,straight_modifier,align_edge_of_the_world,aliign_road,align_buildings,force_homesick,def_point_range,def_point_step,add_point,building_bone_num,noises);
fields!(WorldTerrainReference:source,prefabs,materials,textures,game_settings,tiles,edge_of_world,edge_of_world_gradient,edge_of_world_generation_dist,edge_of_world_warp_dist,edge_of_world_warp_offset,random_seed,seed);
fields!(WorldGameSettings:source,biome,homesick_mode);
fields!(WorldTileReference:tile_size,resolution,draw_distance,y_coord);
fields!(WorldGeneratorReference:source,types,seedplus,no_spawn_chance,point_dist,chunk_dist,height_add,affected_by_road,affects_terrain,align_to_building,align_to_road,buildingspawn,wait_for_terrain);
fields!(WorldObjectTypeReference:prototype,in_desert,in_grass,in_winter,affected_by_road,chance,max_dist_from_road,min_dist_from_building,min_dist_from_road);
fields!(WorldBuildingReference:source,types,groups,default_sides,min_start_dist,max_start_dist,dist_modifier);
fields!(WorldBuildingType:type_id,prototype);
fields!(WorldBuildingGroup:available,current_available,current_count,current_max_count,max_count,min_count,types);
fields!(WorldBuildingSelection:type_id,dont_use_default,use_modifier,sides,max_dist,min_dist,x_dist_modifier);
fields!(WorldBuildingSide:max_xpos,max_y_angle,min_xpos,min_y_angle);
fields!(WorldCatalogReference:source,items,categories);
fields!(WorldCatalogItem:field,compatible_slots,index,name,source,prototype,exclusion,excluded_actions);
fields!(WorldCategory:invalid_category_indices,initialized,index,name,items,categories,item_chances,category_chances,depend_on_car);
fields!(WorldPrototypeBody:source,node,mass_kg,linear_damping,angular_damping,kinematic,use_gravity,constraints,pose);
fields!(WorldPartScopeSlot:source,main_types,part_types);
fields!(WorldSourceMaterial:source,valid_keywords,shader,source_data,floats,colors,textures,render_targets,texture_coordinates);
fields!(WorldRenderTarget:property,source,scale,offset);

fields!(WorldGrassDisplacement:camera,follow,displacement,transform,follow_target,target,follow_height,border,writers);
fields!(WorldGrassWriter:transform,owner,renderer,filter,mesh,materials,layer,enabled);

fields!(super::super::AssetRecord:path,sha256,root,dependencies);
fields!(super::super::rendering::MaterialRecord:source,condition,keywords,properties);
fields!(super::super::rendering::MaterialCondition:source);
fields!(super::super::rendering::ShaderProperties:floats,colors,textures,defaults,exclusions);
fields!(super::super::rendering::TextureRecord:slot,source,file,mips);
fields!(super::super::rendering::TextureMip:file);
fields!(super::super::rendering::TextureExclusion:slot,source);
inline!(super::super::rendering::TextureDefault);
impl super::super::rendering::MaterialRecord {
    pub fn retained_cpu_bytes(&self) -> usize {
        size_of::<Self>() + self.heap_bytes()
    }
}
impl ReferenceManifest {
    /// The shared world graph also owns catalog asset receipts and primary material records.
    pub fn world_retained_cpu_bytes(&self) -> usize {
        self.world.retained_cpu_bytes()
            + self.assets.capacity() * size_of::<super::super::AssetRecord>()
            + self
                .assets
                .iter()
                .filter(|asset| asset.usage == AssetUsage::WorldPrototype)
                .map(RetainedBytes::heap_bytes)
                .sum::<usize>()
            + self.rendering.materials.capacity()
                * size_of::<super::super::rendering::MaterialRecord>()
            + self
                .rendering
                .materials
                .iter()
                .filter(|material| {
                    self.world
                        .prototypes
                        .iter()
                        .any(|prototype| prototype.asset == material.asset)
                })
                .map(RetainedBytes::heap_bytes)
                .sum::<usize>()
    }
}

fields!(WorldLodReference:quality,dither);
fields!(WorldDitherReference:engine_sha256,creator_sha256,binding_sha256,property_name);

fields!(WorldInitialMaterial:controller,renderer,node,material);
