use bevy::{ecs::world::CommandQueue, prelude::*};
use bevy_composable_ui::prelude::*;

#[derive(Component)]
struct Marker(u8);

#[test]
fn tuples_and_vectors_compose_in_order_with_typed_tags() {
    let mut world = World::new();
    let mut queue = CommandQueue::default();
    let root = {
        let mut commands = Commands::new(&mut queue, &world);
        column((
            text("Heading", 18., Color::WHITE).tagged(Marker(0)),
            row((1..4)
                .map(|i| text(format!("Item {i}"), 12., Color::WHITE).tagged(Marker(i)))
                .collect::<Vec<_>>()),
        ))
        .mount(&mut commands)
    };
    queue.apply(&mut world);
    let children = world.get::<Children>(root).unwrap();
    assert_eq!(world.get::<Marker>(children[0]).unwrap().0, 0);
    let row = world.get::<Children>(children[1]).unwrap();
    let values: Vec<_> = row
        .iter()
        .map(|entity| world.get::<Marker>(entity).unwrap().0)
        .collect();
    assert_eq!(values, [1, 2, 3]);
}

#[test]
fn independent_mounts_do_not_share_entities_or_state() {
    let view = || {
        text("Reusable", 12., Color::WHITE)
            .padding(8.)
            .background(Color::BLACK)
    };
    let mut world = World::new();
    let mut queue = CommandQueue::default();
    let (a, b) = {
        let mut commands = Commands::new(&mut queue, &world);
        (view().mount(&mut commands), view().mount(&mut commands))
    };
    queue.apply(&mut world);
    assert_ne!(a, b);
    assert_ne!(
        world.get::<Children>(a).unwrap()[0],
        world.get::<Children>(b).unwrap()[0]
    );
    assert_eq!(world.get::<BackgroundColor>(a).unwrap().0, Color::BLACK);
}
