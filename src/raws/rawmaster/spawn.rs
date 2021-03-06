use std::collections::HashMap;

use crate::{gamesystem, prelude::*};

use super::parse::{parse_dice_string, parse_particle, parse_particle_line};
use super::{find_slot_for_equippable_item, get_renderable_component, string_to_slot};

pub enum SpawnType {
    AtPosition { x: i32, y: i32 },
    Equipped { by: Entity },
    Carried { by: Entity },
}

pub fn spawn_position<'a>(
    pos: SpawnType,
    new_entity: EntityBuilder<'a>,
    tag: &str,
    raws: &RawMaster,
) -> EntityBuilder<'a> {
    let eb = new_entity;

    // Spawn in the specified location
    match pos {
        SpawnType::AtPosition { x, y } => eb.with(Position { x, y }),
        SpawnType::Carried { by } => eb.with(InBackpack { owner: by }),
        SpawnType::Equipped { by } => {
            let slot = find_slot_for_equippable_item(tag, raws);
            eb.with(Equipped { owner: by, slot })
        },
    }
}

pub fn spawn_base_entity<'a, T: raws::BaseRawComponent + Clone>(
    raws: &RawMaster,
    ecs: &'a mut World,
    entity_list: &[T],
    indexes: &HashMap<String, usize>,
    key: &str,
    pos: SpawnType,
) -> (EntityBuilder<'a>, T) {
    let entity_template = &entity_list[indexes[key]];
    let mut eb = ecs.create_entity().marked::<SimpleMarker<SerializeMe>>();

    // Spawn in the specified location
    eb = spawn_position(pos, eb, key, raws);

    // Renderable
    if let Some(renderable) = &entity_template.renderable() {
        eb = eb.with(get_renderable_component(renderable));

        if renderable.x_size.is_some() || renderable.y_size.is_some() {
            eb = eb.with(TileSize {
                x: renderable.x_size.unwrap_or(1),
                y: renderable.y_size.unwrap_or(1),
            });
        }
    }

    // // Name Component
    eb = eb.with(Name {
        name: entity_template.name(),
    });

    (eb, entity_template.clone())
}

#[rustfmt::skip]
macro_rules! apply_effects {
    ( $effects:expr, $eb:expr ) => {
        for effect in $effects.iter() {
        let effect_name = effect.0.as_str();
            match effect_name {
                "area_of_effect" => $eb = $eb.with(AreaOfEffect{ radius: effect.1.parse::<i32>().unwrap() }),
                "confusion" => {
                    $eb = $eb.with(Confusion{});
                    $eb = $eb.with(Duration{ turns: effect.1.parse::<i32>().unwrap() });
                }
                "damage" => $eb = $eb.with(InflictsDamage{ damage : effect.1.parse::<i32>().unwrap() }),
                "damage_over_time" => $eb = $eb.with( DamageOverTime { damage : effect.1.parse::<i32>().unwrap() } ),
                "duration" => $eb = $eb.with(Duration { turns: effect.1.parse::<i32>().unwrap() }),
                "food" => $eb = $eb.with(ProvidesFood{}),
                "identify" => $eb = $eb.with(ProvidesIdentification{}),
                "magic_mapping" => $eb = $eb.with(MagicMapper{}),
                "particle" => $eb = $eb.with(parse_particle(&effect.1)),
                "particle_line" => $eb = $eb.with(parse_particle_line(&effect.1)),
                "provides_healing" => $eb = $eb.with(ProvidesHealing{ heal_amount: effect.1.parse::<i32>().unwrap() }),
                "provides_mana" => $eb = $eb.with(ProvidesMana{ mana_amount: effect.1.parse::<i32>().unwrap() }),
                "ranged" => $eb = $eb.with(Ranged{ range: effect.1.parse::<i32>().unwrap() }),
                "remove_curse" => $eb = $eb.with(ProvidesRemoveCurse{}),
                "single_activation" => $eb = $eb.with(SingleActivation{}),
                "slow" => $eb = $eb.with(Slow{ initiative_penalty : effect.1.parse::<f32>().unwrap() }),
                "target_self" => $eb = $eb.with( AlwaysTargetsSelf{} ),
                "teach_spell" => $eb = $eb.with(TeachesSpell{ spell: effect.1.to_string() }),
                "town_portal" => $eb = $eb.with(TownPortal{}),
                _ => rltk::console::log(format!("Warning: consumable effect {} not implemented.", effect_name))
            }
        }
    };
}

pub fn spawn_named_item(raws: &RawMaster, ecs: &mut World, key: &str, pos: SpawnType) -> Option<Entity> {
    let dm = ecs.fetch::<MasterDungeonMap>();

    let scroll_names = dm.scroll_mappings.clone();
    let potion_names = dm.potion_mappings.clone();
    let identified = dm.identified_items.clone();
    std::mem::drop(dm);

    let (mut eb, item_template) = spawn_base_entity(raws, ecs, &raws.raws.items, &raws.item_index, key, pos);

    // Item Component
    eb = eb.with(Item {
        initiative_penalty: item_template.initiative_penalty.unwrap_or(0.0),
        weight_lbs: item_template.weight_lbs.unwrap_or(0.0),
        base_value: item_template.base_value.unwrap_or(0.0),
    });

    // Consumable Component
    if let Some(consumable) = &item_template.consumable {
        let max_charges = consumable.charges.unwrap_or(1);
        eb = eb.with(Consumable {
            max_charges,
            charges: max_charges,
        });
        apply_effects!(consumable.effects, eb);
    }

    // Equippables
    // Weapion Component
    if let Some(weapon) = &item_template.weapon {
        eb = eb.with(Equippable {
            slot: EquipmentSlot::Melee,
        });

        let (n_dice, die_type, bonus) = parse_dice_string(&weapon.base_damage);
        let mut wpn = Weapon {
            range: if weapon.range == "melee" {
                None
            } else {
                Some(weapon.range.parse::<i32>().expect("Not a number"))
            },
            attribute: WeaponAttribute::Might,
            damage_n_dice: n_dice,
            damage_die_type: die_type,
            damage_bonus: bonus,
            hit_bonus: weapon.hit_bonus,
            proc_chance: weapon.proc_chance,
            proc_target: weapon.proc_target.clone(),
        };

        match weapon.attribute.as_str() {
            "Quickness" => wpn.attribute = WeaponAttribute::Quickness,
            _ => wpn.attribute = WeaponAttribute::Might,
        }

        eb = eb.with(wpn);

        if let Some(proc_effects) = &weapon.proc_effects {
            apply_effects!(proc_effects, eb);
        }
    }

    // Wearable Component
    if let Some(wearable) = &item_template.wearable {
        let slot = string_to_slot(&wearable.slot);

        eb = eb.with(Equippable { slot });

        eb = eb.with(Wearable {
            slot,
            armor_class: wearable.armor_class,
        });
    }

    // Magic Component
    if let Some(magic) = &item_template.magic {
        // Class
        let class = match magic.class.as_str() {
            "rare" => MagicItemClass::Rare,
            "legendary" => MagicItemClass::Legendary,
            _ => MagicItemClass::Common,
        };
        eb = eb.with(MagicItem { class });

        // ObfuscatedName
        if !identified.contains(&item_template.name) {
            match magic.naming.as_str() {
                "scroll" => {
                    eb = eb.with(ObfuscatedName {
                        name: scroll_names[&item_template.name].clone(),
                    });
                },
                "potion" => {
                    eb = eb.with(ObfuscatedName {
                        name: potion_names[&item_template.name].clone(),
                    });
                },
                _ => {
                    eb = eb.with(ObfuscatedName {
                        name: magic.naming.clone(),
                    });
                },
            }
        }

        // Cursed item X(
        if let Some(cursed) = magic.cursed {
            if cursed {
                eb = eb.with(CursedItem {});
            }
        }
    }

    // Attributes Bonus!!!
    if let Some(ab) = &item_template.attributes {
        eb = eb.with(AttributeBonus {
            might: ab.might,
            fitness: ab.fitness,
            quickness: ab.quickness,
            intelligence: ab.intelligence,
        });
    }

    Some(eb.build())
}

pub fn spawn_named_mob(raws: &RawMaster, ecs: &mut World, key: &str, pos: SpawnType) -> Option<Entity> {
    let (mut eb, mob_template) = spawn_base_entity(raws, ecs, &raws.raws.mobs, &raws.mob_index, key, pos);

    match mob_template.movement.as_ref() {
        "random" => eb = eb.with(MoveMode { mode: Movement::Random }),
        "random_waypoint" => {
            eb = eb.with(MoveMode {
                mode: Movement::RandomWaypoint { path: None },
            })
        },
        _ => eb = eb.with(MoveMode { mode: Movement::Static }),
    }

    // BlocksTile
    if mob_template.blocks_tile {
        eb = eb.with(BlocksTile {});
    }

    // Viewshed
    eb = eb.with(Viewshed {
        visible_tiles: Vec::new(),
        range: mob_template.vision_range,
        dirty: true,
    });

    // Quips
    if let Some(quips) = &mob_template.quips {
        eb = eb.with(Quips {
            available: quips.clone(),
        });
    }

    // Natural Attack
    if let Some(na) = &mob_template.natural {
        let mut nature = NaturalAttackDefense {
            armor_class: na.armor_class,
            attacks: Vec::new(),
        };

        if let Some(attacks) = &na.attacks {
            for nattack in attacks.iter() {
                let (n, d, b) = parse_dice_string(&nattack.damage);
                let attack = NaturalAttack {
                    name: nattack.name.clone(),
                    hit_bonus: nattack.hit_bonus,
                    damage_n_dice: n,
                    damage_die_type: d,
                    damage_bonus: b,
                };

                nature.attacks.push(attack);
            }
        }
        eb = eb.with(nature);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Atrributes
    ///////////////////////////////////////////////////////////////////////////
    let mut mob_fitness = 11;
    let mut mob_int = 11;

    #[rustfmt::skip]
    let mut attr = Attributes{
        might: Attribute{ base: 11, modifiers: 0, bonus:gamesystem::attr_bonus(11) },
        fitness: Attribute{ base: 11, modifiers: 0, bonus:gamesystem::attr_bonus(11) },
        quickness: Attribute{ base: 11, modifiers: 0, bonus:gamesystem::attr_bonus(11) },
        intelligence: Attribute{ base: 11, modifiers: 0, bonus:gamesystem::attr_bonus(11) },
    };

    // might
    if let Some(might) = mob_template.attributes.might {
        attr.might = Attribute {
            base: might,
            modifiers: 0,
            bonus: gamesystem::attr_bonus(might),
        };
    }

    // fitness
    if let Some(fitness) = mob_template.attributes.fitness {
        attr.fitness = Attribute {
            base: fitness,
            modifiers: 0,
            bonus: gamesystem::attr_bonus(fitness),
        };
        mob_fitness = fitness;
    }

    // quickness
    if let Some(quickness) = mob_template.attributes.quickness {
        attr.quickness = Attribute {
            base: quickness,
            modifiers: 0,
            bonus: gamesystem::attr_bonus(quickness),
        };
    }

    // intelligence
    if let Some(intelligence) = mob_template.attributes.intelligence {
        attr.intelligence = Attribute {
            base: intelligence,
            modifiers: 0,
            bonus: gamesystem::attr_bonus(intelligence),
        };
        mob_int = intelligence;
    }
    eb = eb.with(attr);

    ///////////////////////////////////////////////////////////////////////////
    // Pools
    ///////////////////////////////////////////////////////////////////////////
    let mob_level = if mob_template.level.is_some() { mob_template.level.unwrap() } else { 1 };
    let mob_hp = gamesystem::npc_hp(mob_fitness, mob_level);
    let mob_mana = gamesystem::mana_at_level(mob_int, mob_level);

    let pools = Pools {
        level: mob_level,
        xp: 0,
        hit_points: Pool {
            current: mob_hp,
            max: mob_hp,
        },
        mana: Pool {
            current: mob_mana,
            max: mob_mana,
        },
        total_weight: 0.0,
        total_initiative_penalty: 0.0,
        gold: if let Some(gold) = &mob_template.gold {
            let (n, d, b) = parse_dice_string(gold);
            (crate::rng::roll_dice(n, d) + b) as f32
        } else {
            0.0
        },
        god_mode: false,
    };
    eb = eb.with(pools);

    ///////////////////////////////////////////////////////////////////////////
    // Skills
    ///////////////////////////////////////////////////////////////////////////
    let mut skills = Skills { skills: HashMap::new() };
    skills.skills.insert(Skill::Melee, 1);
    skills.skills.insert(Skill::Defense, 1);
    skills.skills.insert(Skill::Magic, 1);

    if let Some(mobskills) = &mob_template.skills {
        for sk in mobskills.iter() {
            match sk.0.as_str() {
                "Melee" => {
                    skills.skills.insert(Skill::Melee, *sk.1);
                },
                "Defense" => {
                    skills.skills.insert(Skill::Defense, *sk.1);
                },
                "Magic" => {
                    skills.skills.insert(Skill::Magic, *sk.1);
                },
                _ => {
                    rltk::console::log(format!("Unknown skill referenced: [{}]", sk.0));
                },
            }
        }
    }
    eb = eb.with(skills);

    // Loot Table
    if let Some(loot) = &mob_template.loot_table {
        eb = eb.with(LootTable { table: loot.clone() });
    }

    // Lighting
    if let Some(light) = &mob_template.light {
        eb = eb.with(LightSource {
            range: light.range,
            color: rltk::RGB::from_hex(&light.color).expect("Bad color"),
        });
    }

    // Initiative of 2
    eb = eb.with(Initiative { current: 2 });

    // Faction
    if let Some(faction) = &mob_template.faction {
        eb = eb.with(Faction { name: faction.clone() });
    } else {
        eb = eb.with(Faction {
            name: "Mindless".to_string(),
        })
    }

    // Start With EquipmentChanged
    eb = eb.with(EquipmentChanged {});

    // Vendor
    if let Some(vendor) = &mob_template.vendor {
        eb = eb.with(Vendor {
            categories: vendor.clone(),
        });
    }

    // Special Abilities!!!
    if let Some(ability_list) = &mob_template.abilities {
        let mut a = SpecialAbilities { abilities: Vec::new() };
        for ability in ability_list.iter() {
            a.abilities.push(SpecialAbility {
                chance: ability.chance,
                spell: ability.spell.clone(),
                range: ability.range,
                min_range: ability.min_range,
            });
        }
        eb = eb.with(a);
    }

    if let Some(ability_list) = &mob_template.on_death {
        let mut a = OnDeath { abilities: Vec::new() };
        for ability in ability_list.iter() {
            a.abilities.push(SpecialAbility {
                chance: ability.chance,
                spell: ability.spell.clone(),
                range: ability.range,
                min_range: ability.min_range,
            });
        }
        eb = eb.with(a);
    }

    // Build a mob person thing
    let new_mob = eb.build();

    // Are they wielding anyting?
    if let Some(wielding) = &mob_template.equipped {
        for tag in wielding.iter() {
            spawn_named_entity(raws, ecs, tag, SpawnType::Equipped { by: new_mob });
        }
    }

    Some(new_mob)
}

pub fn spawn_named_prop(new_entity: EntityBuilder, prop_template: raws::Prop) -> Option<Entity> {
    let mut eb = new_entity;

    // Hidden Trait
    if let Some(hidden) = prop_template.hidden {
        if hidden {
            eb = eb.with(Hidden {})
        };
    }

    // Blocks Visibility Trait
    if let Some(blocks_visibility) = prop_template.blocks_visibility {
        if blocks_visibility {
            eb = eb.with(BlocksVisibility {})
        };
    }

    // Door?
    if let Some(door_open) = prop_template.door_open {
        eb = eb.with(Door { open: door_open });
    }

    // Trigger Trait (Traps)
    if let Some(entry_trigger) = &prop_template.entry_trigger {
        eb = eb.with(EntryTrigger {});
        apply_effects!(entry_trigger.effects, eb);
    }

    // Light Source
    if let Some(light) = &prop_template.light {
        eb = eb.with(LightSource {
            range: light.range,
            color: rltk::RGB::from_hex(&light.color).expect("Bad color"),
        });

        eb = eb.with(Viewshed {
            range: light.range,
            dirty: true,
            visible_tiles: Vec::new(),
        });
    }

    Some(eb.build())
}

pub fn spawn_named_spell(raws: &RawMaster, ecs: &mut World, key: &str) -> Option<Entity> {
    if raws.spell_index.contains_key(key) {
        let spell_template = &raws.raws.spells[raws.spell_index[key]];

        let mut eb = ecs.create_entity().marked::<SimpleMarker<SerializeMe>>();

        eb = eb.with(SpellTemplate {
            mana_cost: spell_template.mana_cost,
        });

        eb = eb.with(Name {
            name: spell_template.name.clone(),
        });

        apply_effects!(spell_template.effects, eb);

        return Some(eb.build());
    }

    None
}

pub fn spawn_named_entity(raws: &RawMaster, ecs: &mut World, key: &str, pos: SpawnType) -> Option<Entity> {
    if raws.item_index.contains_key(key) {
        return spawn_named_item(raws, ecs, key, pos);
    } else if raws.mob_index.contains_key(key) {
        return spawn_named_mob(raws, ecs, key, pos);
    } else if raws.prop_index.contains_key(key) {
        let (eb, prop) = spawn_base_entity(raws, ecs, &raws.raws.props, &raws.prop_index, key, pos);
        return spawn_named_prop(eb, prop);
    }

    None
}

pub fn spawn_all_spells(ecs: &mut World) {
    let raws = &RAWS.lock().unwrap();
    for spell in raws.raws.spells.iter() {
        spawn_named_spell(raws, ecs, &spell.name);
    }
}

pub enum SpawnTableType {
    Item,
    Mob,
    Prop,
}

pub fn spawn_type_by_name(raws: &RawMaster, key: &str) -> SpawnTableType {
    if raws.item_index.contains_key(key) {
        SpawnTableType::Item
    } else if raws.mob_index.contains_key(key) {
        SpawnTableType::Mob
    } else {
        SpawnTableType::Prop
    }
}
