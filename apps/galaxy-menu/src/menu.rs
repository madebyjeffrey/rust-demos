//! Operable startup controls over the full-window galaxy backdrop.

use std::collections::{HashMap, HashSet};

use bevy::{
    ecs::spawn::SpawnIter,
    input::{
        ButtonState, InputSystems,
        keyboard::KeyboardInput,
        touch::{TouchInput, TouchPhase},
    },
    input_focus::{
        FocusCause, FocusedInput, InputFocus, InputFocusSystems, InputFocusVisible,
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    },
    picking::{Pickable, PickingSystems},
    prelude::*,
    ui::UiGlobalTransform,
    ui_widgets::{Activate, Button},
    window::{PrimaryWindow, WindowEvent, WindowFocused},
};

const BACKGROUND: Color = Color::srgb(0.025, 0.035, 0.09);
const ACCENT: Color = Color::srgb(0.96, 0.66, 0.39);
const PRIMARY_TEXT: Color = Color::srgb(0.98, 0.83, 0.6);
const SECONDARY_TEXT: Color = Color::srgb(0.61, 0.68, 0.79);
const QUIET_TEXT: Color = Color::srgb(0.39, 0.47, 0.61);

/// Identifies a startup action. New Game, Load Game, and Settings remain placeholders.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuAction {
    NewGame,
    LoadGame,
    Settings,
    Quit,
}

/// Lets tests and presentation code distinguish the two levels of menu emphasis.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuEmphasis {
    Primary,
    Secondary,
}

/// Adds the menu shell and startup controls to the app.
pub struct MenuPlugin;

#[derive(Resource, Default)]
struct KeyboardPress(Option<(Entity, KeyCode)>);

/// Camera input consults ownership captured at the beginning of a UI interaction.
#[derive(Resource, Default)]
pub(crate) struct MenuInputOwnership {
    mouse_capture: Option<Entity>,
    mouse_cancelled: bool,
    mouse_position: Option<Vec2>,
    touch_captures: HashMap<(Entity, u64), Entity>,
    hovered: Option<Entity>,
    /// Includes released UI touches for the current frame so camera gesture readers
    /// can reject their final input as well as their active movement.
    pub(crate) ui_touches: HashSet<(Entity, u64)>,
    pub(crate) blocks_mouse: bool,
    pub(crate) blocks_scroll: bool,
}

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .init_resource::<KeyboardPress>()
            .init_resource::<MenuInputOwnership>()
            .add_plugins(TabNavigationPlugin)
            .add_observer(activate_menu_action)
            .add_observer(keyboard_feedback)
            .add_systems(Startup, setup_menu)
            .add_systems(
                PreUpdate,
                capture_menu_input
                    .after(InputSystems)
                    .after(PickingSystems::Last)
                    .before(InputFocusSystems::Dispatch),
            )
            .add_systems(Update, style_buttons);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy injects independent input resources into this system"
)]
fn capture_menu_input(
    mut commands: Commands,
    mut ownership: ResMut<MenuInputOwnership>,
    mut touches: MessageReader<TouchInput>,
    mut focus_events: MessageReader<WindowFocused>,
    mut window_events: MessageReader<WindowEvent>,
    mut keyboard: ResMut<KeyboardPress>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
        ),
        With<MenuAction>,
    >,
) {
    let focused = windows.single().is_ok_and(|window| window.focused);
    let mut lost_focus = false;
    for event in focus_events.read() {
        lost_focus |= !event.focused && windows.contains(event.window);
    }
    if lost_focus {
        ownership.mouse_capture = None;
        ownership.mouse_cancelled = mouse.pressed(MouseButton::Left);
        ownership.touch_captures.clear();
        keyboard.0 = None;
        focus.clear();
        focus_visible.0 = false;
    }
    let control_at = |position| {
        buttons
            .iter()
            .find_map(|(entity, node, transform, visible)| {
                (visible.get() && node.contains_point(*transform, position)).then_some(entity)
            })
    };
    ownership.ui_touches = ownership.touch_captures.keys().copied().collect();
    for touch in touches.read() {
        let key = (touch.window, touch.id);
        match touch.phase {
            TouchPhase::Started if focused => {
                if let Ok(window) = windows.get(touch.window) {
                    let point = touch.position * window.scale_factor();
                    if let Some(entity) = control_at(point) {
                        ownership.touch_captures.insert(key, entity);
                        ownership.ui_touches.insert(key);
                        focus.set(entity, FocusCause::Pressed);
                        focus_visible.0 = false;
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                if let Some(entity) = ownership.touch_captures.remove(&key)
                    && touch.phase == TouchPhase::Ended
                    && focused
                    && windows.get(touch.window).is_ok_and(|window| {
                        control_at(touch.position * window.scale_factor()) == Some(entity)
                    })
                {
                    commands.trigger(Activate { entity });
                }
            }
            TouchPhase::Started | TouchPhase::Moved => {}
        }
    }
    ownership.hovered = if focused {
        windows
            .single()
            .ok()
            .and_then(Window::physical_cursor_position)
            .and_then(control_at)
    } else {
        None
    };
    let mut captured_this_frame = ownership.mouse_capture.is_some();
    let mut scroll_over_control = false;
    let mut saw_mouse_input = false;
    // Replay native pointer events in order: the final cursor position can already
    // be outside the control when a frame receives both a press and a movement.
    for event in window_events.read() {
        match event {
            WindowEvent::CursorMoved(event) => {
                if let Ok(window) = windows.get(event.window) {
                    ownership.mouse_position = Some(event.position * window.scale_factor());
                }
            }
            WindowEvent::CursorLeft(event) if windows.contains(event.window) => {
                ownership.mouse_position = None;
            }
            WindowEvent::MouseButtonInput(event)
                if event.button == MouseButton::Left && windows.contains(event.window) =>
            {
                saw_mouse_input = true;
                if event.state == ButtonState::Pressed && focused {
                    ownership.mouse_capture = ownership.mouse_position.and_then(control_at);
                    captured_this_frame |= ownership.mouse_capture.is_some();
                    if let Some(entity) = ownership.mouse_capture {
                        focus.set(entity, FocusCause::Pressed);
                        focus_visible.0 = false;
                    }
                } else if event.state == ButtonState::Released
                    && let Some(entity) = ownership.mouse_capture.take()
                    && focused
                    && !ownership.mouse_cancelled
                    && ownership.mouse_position.and_then(control_at) == Some(entity)
                {
                    commands.trigger(Activate { entity });
                }
            }
            WindowEvent::MouseWheel(event) if windows.contains(event.window) => {
                scroll_over_control |= ownership.mouse_position.and_then(control_at).is_some();
            }
            _ => {}
        }
    }
    // Applications may also supply Bevy's direct mouse input without WindowEvent.
    if !saw_mouse_input && focused {
        if mouse.just_pressed(MouseButton::Left) {
            ownership.mouse_capture = ownership.hovered;
            captured_this_frame |= ownership.mouse_capture.is_some();
        }
        if mouse.just_released(MouseButton::Left)
            && let Some(entity) = ownership.mouse_capture.take()
            && !ownership.mouse_cancelled
            && ownership.hovered == Some(entity)
        {
            commands.trigger(Activate { entity });
        }
    }
    ownership.blocks_mouse = captured_this_frame || ownership.mouse_cancelled || !focused;
    ownership.blocks_scroll = ownership.blocks_mouse
        || ownership.hovered.is_some()
        || scroll_over_control
        || !ownership.ui_touches.is_empty();
    if !mouse.pressed(MouseButton::Left) {
        ownership.mouse_capture = None;
        ownership.mouse_cancelled = false;
    }
}

fn setup_menu(mut commands: Commands) {
    commands.insert_resource(ClearColor(BACKGROUND));
    commands.spawn((
        TabGroup::new(0),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            right: px(24),
            top: px(40),
            max_width: px(320),
            flex_direction: FlexDirection::Column,
            row_gap: px(22),
            ..default()
        },
        children![
            (
                Node {
                    width: px(62),
                    height: px(7),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(ACCENT),
            ),
            (
                Text::new("GALAXY MENU"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(ACCENT),
            ),
            (
                Text::new("CHOOSE YOUR NEXT STEP"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(QUIET_TEXT),
                Node {
                    margin: UiRect::top(px(-13)),
                    ..default()
                },
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    margin: UiRect::top(px(14)),
                    ..default()
                },
                children![
                    action_button("New Game", MenuAction::NewGame, MenuEmphasis::Primary),
                    action_button("Load Game", MenuAction::LoadGame, MenuEmphasis::Primary),
                ],
            ),
            (Node {
                height: px(10),
                ..default()
            },),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                },
                Children::spawn(SpawnIter(
                    [
                        ("Settings", MenuAction::Settings),
                        ("Quit", MenuAction::Quit)
                    ]
                    .into_iter()
                    .filter(|(_, action)| !cfg!(target_os = "ios") || *action != MenuAction::Quit)
                    .map(|(label, action)| action_button(
                        label,
                        action,
                        MenuEmphasis::Secondary
                    )),
                )),
            ),
        ],
    ));
}

fn activate_menu_action(
    event: On<Activate>,
    actions: Query<&MenuAction>,
    mut exit: MessageWriter<AppExit>,
) {
    if actions.get(event.entity) == Ok(&MenuAction::Quit) {
        exit.write(AppExit::Success);
    }
}

fn keyboard_feedback(
    event: On<FocusedInput<KeyboardInput>>,
    actions: Query<(), With<MenuAction>>,
    mut pressed: ResMut<KeyboardPress>,
) {
    let input = &event.input;
    if !matches!(input.key_code, KeyCode::Enter | KeyCode::Space) {
        return;
    }
    if input.state == ButtonState::Released {
        pressed.0 = None;
    } else if !input.repeat && actions.contains(event.focused_entity) {
        pressed.0 = Some((event.focused_entity, input.key_code));
    }
}

fn style_buttons(
    mut buttons: Query<(Entity, &mut BackgroundColor, &mut BorderColor), With<MenuAction>>,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut keyboard: ResMut<KeyboardPress>,
    ownership: Res<MenuInputOwnership>,
) {
    if keyboard
        .0
        .is_some_and(|(entity, _)| focus.get() != Some(entity))
    {
        keyboard.0 = None;
    }
    for (entity, mut background, mut border) in &mut buttons {
        let pressed = ownership.mouse_capture == Some(entity)
            || ownership
                .touch_captures
                .values()
                .any(|captured| *captured == entity);
        *background = BackgroundColor(
            if pressed || keyboard.0.is_some_and(|(focused, _)| focused == entity) {
                Color::srgba(0.35, 0.20, 0.10, 0.95)
            } else if ownership.hovered == Some(entity) {
                Color::srgba(0.12, 0.14, 0.21, 0.95)
            } else {
                Color::srgba(0.04, 0.055, 0.11, 0.8)
            },
        );
        border.set_all(if focus_visible.0 && focus.get() == Some(entity) {
            ACCENT
        } else {
            Color::srgba(0.61, 0.68, 0.79, 0.35)
        });
    }
}

fn action_button(label: &'static str, action: MenuAction, emphasis: MenuEmphasis) -> impl Bundle {
    let (font_size, color) = match emphasis {
        MenuEmphasis::Primary => (FontSize::Px(32.0), PRIMARY_TEXT),
        MenuEmphasis::Secondary => (FontSize::Px(19.0), SECONDARY_TEXT),
    };

    (
        Button,
        TabIndex(action as i32),
        // Pointer activation uses ordered native input and captured targets. Bevy's
        // frame-final picking would otherwise activate a different control after a
        // batched drag, or emit a second Activate for a tap handled above.
        Pickable::IGNORE,
        Node {
            width: percent(100),
            min_height: px(48),
            padding: UiRect::axes(px(14), px(8)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.055, 0.11, 0.8)),
        BorderColor::all(Color::srgba(0.61, 0.68, 0.79, 0.35)),
        Text::new(label),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
        action,
        emphasis,
    )
}
