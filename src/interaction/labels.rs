//! Player-facing names for recovered interaction items.
use crate::reference::{ItemId, ItemKind, ReferenceManifest};

pub fn display_name<'a>(
    reference: &'a ReferenceManifest,
    items: &'a crate::interaction::items::ItemStore,
    id: ItemId,
) -> &'a str {
    let Some(item) = reference.items.iter().find(|item| item.id == id) else {
        return items.name(id).unwrap_or("");
    };
    let label = match item.name.as_str() {
        "GarageDoorRightCollider" | "GarageDoorLeftCollider"
            if reference.mechanisms.iter().any(|m| m.item == id) =>
        {
            Some("Garage door")
        }
        "TireDisztarcsa" => Some("Hubcap"),
        "SideMirror" | "LeftMirror" | "RightMirror" => Some("Side mirror"),
        "FrontMirror" => Some("Rearview mirror"),
        "Kesztyutarto" => Some("Glovebox"),
        "napellenzo" => Some("Sun visor"),
        _ => None,
    };
    if let Some(label) = label {
        return label;
    }
    if let Some(mechanism) = reference.mechanisms.iter().find(|m| m.item == id)
        && mechanism.turn.is_none()
        && let Some(parent) = mechanism.parent_item
    {
        return display_name(reference, items, parent);
    }
    match item.kind {
        ItemKind::Decoration => reference
            .mechanisms
            .iter()
            .find(|m| m.item == id)
            .and_then(|m| m.turn.as_ref())
            .map_or("", |turn| match turn.action {
                crate::reference::TurnAction::ToggleHandbrake => "Handbrake",
                crate::reference::TurnAction::CycleHeadlights => "Headlights",
                crate::reference::TurnAction::InteriorLight => "Cabin light",
                crate::reference::TurnAction::ToggleHomeLight { .. } => "Light switch",
            }),
        ItemKind::Door => {
            if crate::starting_area::body_vehicle(reference, item.body).is_some() {
                "Car door"
            } else {
                item.name.as_str()
            }
        }
        ItemKind::Hood => "Hood",
        ItemKind::Trunk => "Trunk",
        ItemKind::Engine => "Engine",
        ItemKind::Radiator => "Radiator",
        ItemKind::Tire => "Tire",
        ItemKind::Rim => "Rim",
        ItemKind::Wheel => "Wheel",
        ItemKind::Seat => {
            if reference
                .seats
                .iter()
                .any(|seat| seat.item == id && seat.driver)
            {
                "Driver seat"
            } else {
                "Seat"
            }
        }
        ItemKind::Cap => {
            if reference
                .openings
                .iter()
                .any(|opening| opening.cap == id && opening.can_tune)
            {
                return "Water tap";
            }
            let tank = reference
                .openings
                .iter()
                .find(|opening| opening.cap == id)
                .map(|opening| opening.tank);
            let engines = || reference.vehicles.iter().map(|vehicle| &vehicle.engine);
            if engines().any(|engine| tank == Some(engine.fuel_tank)) {
                "Fuel cap"
            } else if engines().any(|engine| tank == Some(engine.oil_tank)) {
                "Oil cap"
            } else if engines().any(|engine| tank == Some(engine.coolant_tank)) {
                "Coolant cap"
            } else {
                "Container cap"
            }
        }
        ItemKind::Container => {
            if reference
                .vehicles
                .iter()
                .any(|vehicle| vehicle.body == item.body)
            {
                return "Fuel tank";
            }
            if let Some(tank) = reference.tanks.iter().find(|tank| tank.item == id) {
                if tank.quantities_liters[0] > 0.0 {
                    "Fuel can"
                } else if tank.quantities_liters[1] > 0.0 {
                    "Oil can"
                } else if tank.quantities_liters[3] > 0.0 {
                    "Water container"
                } else {
                    "Container"
                }
            } else {
                "Container"
            }
        }
    }
}
