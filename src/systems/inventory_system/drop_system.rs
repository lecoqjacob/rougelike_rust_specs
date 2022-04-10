use specs::prelude::*;

use super::{
    EquipmentChanged,
    GameLog,
    InBackpack,
    MagicItem,
    MasterDungeonMap,
    Name,
    ObfuscatedName,
    Position,
    WantsToDropItem,
};

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToDropItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, InBackpack>,
        WriteStorage<'a, EquipmentChanged>,
        ReadStorage<'a, MagicItem>,
        ReadStorage<'a, ObfuscatedName>,
        ReadExpect<'a, MasterDungeonMap>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity,
            mut gamelog,
            entities,
            mut wants_drop,
            names,
            mut positions,
            mut backpack,
            mut dirty_equipment,
            magic_items,
            obfuscated_names,
            dm,
        ) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos: Position = Position { x: 0, y: 0 };
            {
                let dropped_pos = positions.get(entity).unwrap();
                dropper_pos.x = dropped_pos.x;
                dropper_pos.y = dropped_pos.y;
            }

            positions
                .insert(
                    to_drop.item,
                    Position {
                        x: dropper_pos.x,
                        y: dropper_pos.y,
                    },
                )
                .expect("Unable to insert position");

            backpack.remove(to_drop.item);
            dirty_equipment
                .insert(entity, EquipmentChanged {})
                .expect("Unable to insert");

            if entity == *player_entity {
                gamelog.add(format!(
                    "You drop the {}.",
                    super::obfuscate_name(to_drop.item, &names, &magic_items, &obfuscated_names, &dm)
                ));
            }
        }

        wants_drop.clear();
    }
}