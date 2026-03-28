use bevy::ecs::bundle::Bundle;
use bevy::ecs::world::World;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::prelude::{Commands, Entity};

pub fn safe_despawn_recursive(commands: &mut Commands, entity: Entity) {
    commands.add(move |world: &mut World| {
        if world.get_entity(entity).is_some() {
            world.entity_mut(entity).despawn_recursive();
        }
    });
}

pub fn safe_insert_bundle<B: Bundle + Send + Sync + 'static>(
    commands: &mut Commands,
    entity: Entity,
    bundle: B,
) {
    commands.add(move |world: &mut World| {
        if let Some(mut entity_mut) = world.get_entity_mut(entity) {
            entity_mut.insert(bundle);
        }
    });
}
