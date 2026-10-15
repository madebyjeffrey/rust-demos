use bevy::{asset::Assets, prelude::*};
use galaxy_menu::{
    GalaxyMapPlugin, PlanetCardDetail, PlanetCardName, PlanetCardState, PlanetCursorRay,
    PlanetHoverLabel, PlanetId, PlanetInteractionInput, PlanetInteractionPlugin,
    PlanetInteractionState, PlanetMarker, PlanetSelectionHighlight,
};

fn app_with_planet_interaction() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .add_plugins((GalaxyMapPlugin, PlanetInteractionPlugin));
    app.update();
    app
}

fn planet_position(app: &mut App, planet_id: PlanetId) -> Vec3 {
    let world = app.world_mut();
    let mut query = world.query::<(&PlanetMarker, &GlobalTransform)>();
    query
        .iter(world)
        .find(|(planet, _)| planet.id == planet_id)
        .map(|(_, transform)| transform.translation())
        .expect("the named planet should be present in the galaxy map")
}

fn point_cursor_at(app: &mut App, position: Vec3, click: bool) {
    let mut input = app.world_mut().resource_mut::<PlanetInteractionInput>();
    input.cursor_ray = Some(PlanetCursorRay {
        origin: position + Vec3::Z * 3.0,
        direction: Vec3::NEG_Z,
    });
    input.primary_just_pressed = click;
}

#[test]
fn hovering_a_planet_updates_the_public_app_state() {
    let mut app = app_with_planet_interaction();
    let position = planet_position(&mut app, PlanetId::Vesper);

    point_cursor_at(&mut app, position, false);
    app.update();

    let state = app.world().resource::<PlanetInteractionState>();
    assert_eq!(state.hovered_planet, Some(PlanetId::Vesper));

    let world = app.world_mut();
    let mut labels = world.query::<(&PlanetHoverLabel, &Node)>();
    let (_, label_node) = labels
        .iter(world)
        .find(|(label, _)| label.planet_id == PlanetId::Vesper)
        .expect("each planet should have a hover label");
    assert_eq!(label_node.display, Display::Flex);
}

#[test]
fn selecting_planets_moves_the_visible_highlight_to_the_selected_planet() {
    let mut app = app_with_planet_interaction();
    let vesper = planet_position(&mut app, PlanetId::Vesper);
    let kestrel = planet_position(&mut app, PlanetId::Kestrel);

    point_cursor_at(&mut app, vesper, true);
    app.update();
    assert_eq!(
        app.world()
            .resource::<PlanetInteractionState>()
            .selected_planet,
        Some(PlanetId::Vesper)
    );
    assert_eq!(visible_highlights(&mut app), vec![PlanetId::Vesper]);
    point_cursor_at(&mut app, kestrel, true);
    app.update();
    assert_eq!(
        app.world()
            .resource::<PlanetInteractionState>()
            .selected_planet,
        Some(PlanetId::Kestrel)
    );
    assert_eq!(visible_highlights(&mut app), vec![PlanetId::Kestrel]);
}

#[test]
fn selecting_a_planet_updates_the_planet_focused_card() {
    let mut app = app_with_planet_interaction();
    let vesper = planet_position(&mut app, PlanetId::Vesper);

    point_cursor_at(&mut app, vesper, true);
    app.update();

    let card = app.world().resource::<PlanetCardState>();
    assert_eq!(card.planet_id, Some(PlanetId::Vesper));

    let world = app.world_mut();
    let mut name_text = world.query_filtered::<&Text, With<PlanetCardName>>();
    assert_eq!(
        name_text.iter(world).next().unwrap().0,
        "Vesper",
        "the visible card should show the selected planet name"
    );
    let mut detail_text = world.query_filtered::<&Text, With<PlanetCardDetail>>();
    assert_eq!(
        detail_text.iter(world).next().unwrap().0,
        "Ocean world at the edge of the inner arm.",
        "the visible card should show a short planet detail"
    );
    let mut card_visuals = world.query::<(&Node, &BackgroundColor)>();
    let (_, visual_color) = card_visuals
        .iter(world)
        .find(|(node, _)| node.width == px(96) && node.height == px(96))
        .expect("the card should include a large planet visual");
    assert_eq!(visual_color.0, Color::srgb(0.35, 0.74, 1.0));
}

fn visible_highlights(app: &mut App) -> Vec<PlanetId> {
    let world = app.world_mut();
    let mut highlights = world.query::<(&PlanetSelectionHighlight, &Visibility)>();
    highlights
        .iter(world)
        .filter_map(|(highlight, visibility)| {
            (*visibility == Visibility::Visible).then_some(highlight.planet_id)
        })
        .collect()
}
