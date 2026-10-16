mod common;

use bevy::prelude::*;
use galaxy_menu::{MenuAction, MenuEmphasis};

#[test]
fn menu_app_shows_primary_and_secondary_actions() {
    let mut app = common::launch_app(UVec2::new(1280, 720), 1.0);

    let world = app.world_mut();
    let mut query = world.query::<(&MenuAction, &MenuEmphasis, &Text)>();
    let mut observed = query
        .iter(world)
        .map(|(action, emphasis, text)| (*action, *emphasis, text.0.as_str()))
        .collect::<Vec<_>>();
    observed.sort_by_key(|(action, _, _)| *action);

    assert_eq!(
        observed,
        vec![
            (MenuAction::NewGame, MenuEmphasis::Primary, "New Game"),
            (MenuAction::LoadGame, MenuEmphasis::Primary, "Load Game"),
            (MenuAction::Settings, MenuEmphasis::Secondary, "Settings"),
            (MenuAction::Quit, MenuEmphasis::Secondary, "Quit"),
        ]
    );
}
