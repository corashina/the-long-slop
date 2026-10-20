//! Road worker entry. Source data belongs to this WASM instance.
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static PROTOTYPE: RefCell<Option<road_mesh::RoadPrototype>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn install(input: &[u8]) -> Result<(), JsError> {
    PROTOTYPE.with(|slot| *slot.borrow_mut() = None);
    let prototype = road_mesh::decode_prototype(input).map_err(|error| JsError::new(&error))?;
    PROTOTYPE.with(|slot| *slot.borrow_mut() = Some(prototype));
    Ok(())
}

/// Each getter moves its attribute out once.
#[wasm_bindgen]
pub struct RoadSurface(road_mesh::RoadMesh);

#[wasm_bindgen]
impl RoadSurface {
    pub fn origin(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.0.origin).to_vec()
    }
    pub fn vertices(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.0.vertices).into_flattened()
    }
    pub fn normals(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.0.normals).into_flattened()
    }
    pub fn tangents(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.0.tangents).into_flattened()
    }
    pub fn uv(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.0.uv).into_flattened()
    }
    pub fn triangles(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.0.triangles).into_flattened()
    }
    pub fn submeshes(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.0.submeshes)
            .into_iter()
            .flat_map(|range| {
                [
                    u32::try_from(range.start).expect("admitted road range"),
                    u32::try_from(range.end).expect("admitted road range"),
                ]
            })
            .collect()
    }
}

#[wasm_bindgen]
pub fn evaluate(input: &[u8]) -> Result<RoadSurface, JsError> {
    let pose = road_mesh::decode_pose(input).map_err(|error| JsError::new(&error))?;
    PROTOTYPE.with(|slot| {
        let slot = slot.borrow();
        let prototype = slot
            .as_ref()
            .ok_or_else(|| JsError::new("Road source is not installed"))?;
        Ok(RoadSurface(road_mesh::deform(prototype, &pose)))
    })
}
