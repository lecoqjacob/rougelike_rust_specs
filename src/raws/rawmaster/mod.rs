use std::collections::HashMap;

use crate::prelude::*;

mod build;
mod load;
mod parse;

pub mod spawn;
pub use build::*;
pub use load::*;
pub use parse::*;
pub use spawn::*;

pub struct RawMaster {
    raws: Raws,
    item_index: HashMap<String, usize>,
    mob_index: HashMap<String, usize>,
    prop_index: HashMap<String, usize>,
    loot_index: HashMap<String, usize>,
    faction_index: HashMap<String, HashMap<String, raws::Reaction>>,
    spell_index: HashMap<String, usize>,
}

impl RawMaster {
    pub fn empty() -> RawMaster {
        RawMaster {
            raws: Raws {
                items: Vec::new(),
                mobs: Vec::new(),
                props: Vec::new(),
                spawn_table: Vec::new(),
                loot_tables: Vec::new(),
                faction_table: Vec::new(),
                spells: Vec::new(),
                weapon_traits: Vec::new(),
            },
            item_index: HashMap::new(),
            mob_index: HashMap::new(),
            prop_index: HashMap::new(),
            loot_index: HashMap::new(),
            faction_index: HashMap::new(),
            spell_index: HashMap::new(),
        }
    }
}

pub fn get_spawn_table_for_depth(raws: &RawMaster, depth: i32) -> MasterTable {
    let available_options: Vec<&raws::SpawnTableEntry> = raws
        .raws
        .spawn_table
        .iter()
        .filter(|a| depth >= a.min_depth && depth <= a.max_depth)
        .collect();

    let mut rt = MasterTable::new();

    for e in available_options.iter() {
        let mut weight = e.weight;

        if e.add_map_depth_to_weight.is_some() {
            weight += depth;
        }

        rt.add(e.name.clone(), weight, raws);
    }

    rt
}

pub fn faction_reaction(my_faction: &str, their_faction: &str, raws: &RawMaster) -> raws::Reaction {
    if raws.faction_index.contains_key(my_faction) {
        let mf = &raws.faction_index[my_faction];

        if mf.contains_key(their_faction) {
            return mf[their_faction];
        } else if mf.contains_key("Default") {
            return mf["Default"];
        } else {
            return raws::Reaction::Ignore;
        }
    }

    raws::Reaction::Ignore
}

pub fn get_renderable_component(renderable: &super::Renderable) -> crate::components::Renderable {
    crate::components::Renderable {
        glyph: rltk::to_cp437(renderable.glyph.chars().next().unwrap()),
        fg: rltk::RGB::from_hex(&renderable.fg).expect("Invalid RGB"),
        bg: rltk::RGB::from_hex(&renderable.bg).expect("Invalid RGB"),
        render_order: renderable.order,
    }
}

pub fn string_to_slot(slot: &str) -> EquipmentSlot {
    match slot {
        "Shield" => EquipmentSlot::Shield,
        "Head" => EquipmentSlot::Head,
        "Torso" => EquipmentSlot::Torso,
        "Legs" => EquipmentSlot::Legs,
        "Feet" => EquipmentSlot::Feet,
        "Hands" => EquipmentSlot::Hands,
        "Melee" => EquipmentSlot::Melee,
        _ => {
            rltk::console::log(format!("Warning: unknown equipment slot type [{}])", slot));
            EquipmentSlot::Melee
        },
    }
}

fn find_slot_for_equippable_item(tag: &str, raws: &RawMaster) -> EquipmentSlot {
    if !raws.item_index.contains_key(tag) {
        panic!("Trying to equip an unknown item: {}", tag);
    }

    let item_index = raws.item_index[tag];
    let item = &raws.raws.items[item_index];

    if let Some(_wpn) = &item.weapon {
        return EquipmentSlot::Melee;
    } else if let Some(wearable) = &item.wearable {
        return string_to_slot(&wearable.slot);
    }

    panic!("Trying to equip {}, but it has no slot tag.", tag);
}

pub fn get_item_drop(raws: &RawMaster, table: &str) -> Option<String> {
    if !raws.loot_index.contains_key(table) {
        return None;
    }

    let mut rt = RandomTable::new();
    let available_options = &raws.raws.loot_tables[raws.loot_index[table]];

    for item in available_options.drops.iter() {
        rt.add(item.name.clone(), item.weight);
    }

    Some(rt.roll())
}

pub fn get_vendor_items(categories: &[String], raws: &RawMaster) -> Vec<(String, f32)> {
    let mut result: Vec<(String, f32)> = Vec::new();

    for item in raws.raws.items.iter() {
        if let Some(cat) = &item.vendor_category {
            if categories.contains(cat) && item.base_value.is_some() {
                result.push((item.name.clone(), item.base_value.unwrap()));
            }
        }
    }

    result
}

pub fn get_scroll_tags() -> Vec<String> {
    let raws = &crate::raws::RAWS.lock().unwrap();
    let mut result = Vec::new();

    for item in raws.raws.items.iter() {
        if let Some(magic) = &item.magic {
            if &magic.naming == "scroll" {
                result.push(item.name.clone());
            }
        }
    }

    result
}

pub fn get_potion_tags() -> Vec<String> {
    let raws = &crate::raws::RAWS.lock().unwrap();
    let mut result = Vec::new();

    for item in raws.raws.items.iter() {
        if let Some(magic) = &item.magic {
            if &magic.naming == "potion" {
                result.push(item.name.clone());
            }
        }
    }

    result
}

pub fn is_tag_magic(tag: &str) -> bool {
    let raws = &crate::raws::RAWS.lock().unwrap();
    if raws.item_index.contains_key(tag) {
        let item_template = &raws.raws.items[raws.item_index[tag]];
        item_template.magic.is_some()
    } else {
        false
    }
}

pub fn find_spell_entity(ecs: &World, name: &str) -> Option<Entity> {
    let names = ecs.read_storage::<Name>();
    let spell_templates = ecs.read_storage::<SpellTemplate>();
    let entities = ecs.entities();

    for (entity, sname, _template) in (&entities, &names, &spell_templates).join() {
        if name == sname.name {
            return Some(entity);
        }
    }
    None
}

pub fn find_spell_entity_by_name(
    name: &str,
    names: &ReadStorage<Name>,
    spell_templates: &ReadStorage<SpellTemplate>,
    entities: &Entities,
) -> Option<Entity> {
    for (entity, sname, _template) in (entities, names, spell_templates).join() {
        if name == sname.name {
            return Some(entity);
        }
    }

    None
}
