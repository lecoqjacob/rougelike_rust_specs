use serde::Deserialize;
use std::sync::Mutex;

pub mod structs;
use structs::*;

pub mod rawmaster;
pub use rawmaster::*;

rltk::embedded_resource!(RAW_ITEMS_FILE, "../../raws/items.json");
rltk::embedded_resource!(RAW_MOBS_FILE, "../../raws/mobs.json");
rltk::embedded_resource!(RAW_PROPS_FILE, "../../raws/props.json");
rltk::embedded_resource!(RAW_SPAWN_TABLE_FILE, "../../raws/spawn_table.json");
rltk::embedded_resource!(RAW_LOOT_TABLES_FILE, "../../raws/loot_tables.json");
rltk::embedded_resource!(RAW_FACTION_TABLE_FILE, "../../raws/faction_table.json");
rltk::embedded_resource!(RAW_SPELLS_FILE, "../../raws/spells.json");
rltk::embedded_resource!(RAW_WEAPON_TRAITS_FILE, "../../raws/weapon_traits.json");

lazy_static! {
    pub static ref RAWS: Mutex<RawMaster> = Mutex::new(RawMaster::empty());
}

#[derive(Deserialize, Debug)]
pub struct Raws {
    pub items: Vec<Item>,
    pub mobs: Vec<Mob>,
    pub props: Vec<Prop>,
    pub spawn_table: Vec<SpawnTableEntry>,
    pub loot_tables: Vec<LootTable>,
    pub faction_table: Vec<FactionInfo>,
    pub spells: Vec<Spell>,
    pub weapon_traits: Vec<WeaponTrait>,
}

fn load_file<'a, T: serde::Deserialize<'a>>(file_path: &str) -> T {
    // Retrieve the raw data as an array of u8 (8-bit unsigned chars)
    let raw_data = rltk::embedding::EMBED
        .lock()
        .get_resource(file_path.to_string())
        .unwrap();

    let raw_string = std::str::from_utf8(raw_data).expect("Unable to convert to a valid UTF-8 string.");
    let decoded_data: T = serde_json::from_str(raw_string).expect("Unable to parse JSON");
    decoded_data
}

pub fn load_raws() {
    rltk::link_resource!(RAW_ITEMS_FILE, "../../raws/items.json");
    rltk::link_resource!(RAW_MOBS_FILE, "../../raws/mobs.json");
    rltk::link_resource!(RAW_PROPS_FILE, "../../raws/props.json");
    rltk::link_resource!(RAW_SPAWN_TABLE_FILE, "../../raws/spawn_table.json");
    rltk::link_resource!(RAW_LOOT_TABLES_FILE, "../../raws/loot_tables.json");
    rltk::link_resource!(RAW_FACTION_TABLE_FILE, "../../raws/faction_table.json");
    rltk::link_resource!(RAW_SPELLS_FILE, "../../raws/spells.json");
    rltk::link_resource!(RAW_WEAPON_TRAITS_FILE, "../../raws/weapon_traits.json");

    let items = load_file::<Vec<Item>>("../../raws/items.json");
    let mobs = load_file::<Vec<Mob>>("../../raws/mobs.json");
    let props = load_file::<Vec<Prop>>("../../raws/props.json");
    let spawn_table = load_file::<Vec<SpawnTableEntry>>("../../raws/spawn_table.json");
    let loot_tables = load_file::<Vec<LootTable>>("../../raws/loot_tables.json");
    let faction_table = load_file::<Vec<FactionInfo>>("../../raws/faction_table.json");
    let spells = load_file::<Vec<Spell>>("../../raws/spells.json");
    let weapon_traits = load_file::<Vec<WeaponTrait>>("../../raws/weapon_traits.json");

    RAWS.lock().unwrap().load(Raws {
        items,
        mobs,
        props,
        spawn_table,
        loot_tables,
        faction_table,
        spells,
        weapon_traits,
    });
}
