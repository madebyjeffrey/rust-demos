//! Operable startup controls over the full-window galaxy backdrop.

use std::collections::{HashMap, HashSet};

use bevy::{
    ecs::spawn::SpawnIter,
    input::{
        ButtonState, InputSystems,
        keyboard::KeyboardInput,
        mouse::MouseWheel,
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

use crate::window_layout::WindowSafeArea;

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
///
/// Provides ordered input dispatch and focus; disable Bevy's `InputDispatchPlugin`
/// and `PointerFocusPlugin` when assembling the application.
pub struct MenuPlugin;

#[derive(Resource, Default)]
struct KeyboardPress(Option<(Entity, KeyCode)>);

#[derive(Clone, Copy)]
enum ScrollOwner {
    Menu,
    Backdrop,
}

/// Camera input consults ownership captured at the beginning of a UI interaction.
#[derive(Resource, Default)]
pub(crate) struct MenuInputOwnership {
    mouse_capture: Option<Entity>,
    mouse_cancelled: bool,
    mouse_pressed: bool,
    mouse_position: Option<Vec2>,
    window_focused: Option<bool>,
    scroll_owner: Option<ScrollOwner>,
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
            .add_systems(Update, (layout_menu, style_buttons));
        #[cfg(not(target_os = "ios"))]
        app.add_systems(Startup, configure_window_minimum);
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
    mut keyboard_inputs: MessageReader<KeyboardInput>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut window_events: MessageReader<WindowEvent>,
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
    // Native events preserve ordering between pointer input and focus changes.
    // Direct Bevy messages remain supported when no native events of that kind exist.
    let mut events = window_events.read().cloned().collect::<Vec<_>>();
    let direct_focus = focus_events.read().cloned().collect::<Vec<_>>();
    if !events
        .iter()
        .any(|event| matches!(event, WindowEvent::WindowFocused(_)))
    {
        events.extend(direct_focus.into_iter().map(WindowEvent::WindowFocused));
    }
    let direct_touches = touches.read().copied().collect::<Vec<_>>();
    if !events
        .iter()
        .any(|event| matches!(event, WindowEvent::TouchInput(_)))
    {
        events.extend(direct_touches.into_iter().map(WindowEvent::TouchInput));
    }
    let direct_scroll = scroll_events.read().copied().collect::<Vec<_>>();
    if !events
        .iter()
        .any(|event| matches!(event, WindowEvent::MouseWheel(_)))
    {
        events.extend(direct_scroll.into_iter().map(WindowEvent::MouseWheel));
    }
    let direct_keys = keyboard_inputs.read().cloned().collect::<Vec<_>>();
    if !events
        .iter()
        .any(|event| matches!(event, WindowEvent::KeyboardInput(_)))
    {
        events.extend(direct_keys.into_iter().map(WindowEvent::KeyboardInput));
    }
    let mut focused = ownership
        .window_focused
        .unwrap_or_else(|| windows.single().is_ok_and(|window| window.focused));
    let control_at = |position| {
        buttons
            .iter()
            .find_map(|(entity, node, transform, visible)| {
                (visible.get() && node.contains_point(*transform, position)).then_some(entity)
            })
    };
    ownership.ui_touches = ownership.touch_captures.keys().copied().collect();
    let mut captured_this_frame = ownership.mouse_capture.is_some();
    let mut scroll_over_control = false;
    let mut saw_scroll_input = false;
    let mut saw_mouse_input = false;
    for event in &events {
        match event {
            WindowEvent::WindowFocused(event) if windows.contains(event.window) => {
                focused = event.focused;
                if !focused {
                    ownership.mouse_capture = None;
                    ownership.mouse_cancelled = ownership.mouse_pressed;
                    ownership.touch_captures.clear();
                    // Suppress the rest of an interrupted scroll until its end.
                    if ownership.scroll_owner.is_some() {
                        ownership.scroll_owner = Some(ScrollOwner::Menu);
                    }
                    commands.queue(clear_menu_focus);
                }
            }
            WindowEvent::KeyboardInput(input) if windows.contains(input.window) && focused => {
                let input = input.clone();
                commands.queue(move |world: &mut World| dispatch_menu_key(world, input));
            }
            WindowEvent::TouchInput(touch) if windows.contains(touch.window) => {
                let key = (touch.window, touch.id);
                match touch.phase {
                    TouchPhase::Started if focused => {
                        let window = windows.get(touch.window).unwrap();
                        if let Some(entity) = control_at(touch.position * window.scale_factor()) {
                            ownership.touch_captures.insert(key, entity);
                            ownership.ui_touches.insert(key);
                            commands
                                .queue(move |world: &mut World| focus_menu_control(world, entity));
                        } else {
                            commands.queue(clear_menu_focus);
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
                ownership.mouse_pressed = event.state == ButtonState::Pressed;
                if event.state == ButtonState::Pressed && focused {
                    ownership.mouse_capture = ownership.mouse_position.and_then(control_at);
                    captured_this_frame |= ownership.mouse_capture.is_some();
                    if let Some(entity) = ownership.mouse_capture {
                        commands.queue(move |world: &mut World| focus_menu_control(world, entity));
                    } else {
                        commands.queue(clear_menu_focus);
                    }
                } else if event.state == ButtonState::Released
                    && let Some(entity) = ownership.mouse_capture.take()
                    && focused
                    && !ownership.mouse_cancelled
                    && ownership.mouse_position.and_then(control_at) == Some(entity)
                {
                    commands.trigger(Activate { entity });
                }
                if event.state == ButtonState::Released {
                    ownership.mouse_cancelled = false;
                }
            }
            WindowEvent::MouseWheel(event) if windows.contains(event.window) => {
                saw_scroll_input = true;
                let over_control = ownership.mouse_position.and_then(control_at).is_some()
                    || ownership.mouse_capture.is_some()
                    || ownership.mouse_cancelled
                    || !ownership.ui_touches.is_empty()
                    || !focused;
                if event.phase == TouchPhase::Started {
                    ownership.scroll_owner = Some(if over_control {
                        ScrollOwner::Menu
                    } else {
                        ScrollOwner::Backdrop
                    });
                }
                scroll_over_control |= match ownership.scroll_owner {
                    Some(ScrollOwner::Menu) => true,
                    Some(ScrollOwner::Backdrop) => false,
                    None => over_control,
                };
                if matches!(event.phase, TouchPhase::Ended | TouchPhase::Canceled) {
                    ownership.scroll_owner = None;
                }
            }
            _ => {}
        }
    }
    ownership.window_focused = Some(focused);
    ownership.hovered = if focused {
        windows
            .single()
            .ok()
            .and_then(Window::physical_cursor_position)
            .and_then(control_at)
    } else {
        None
    };
    // Applications may also supply Bevy's direct mouse input without WindowEvent.
    if !saw_mouse_input && focused {
        if mouse.just_pressed(MouseButton::Left) {
            ownership.mouse_capture = ownership.hovered;
            captured_this_frame |= ownership.mouse_capture.is_some();
            if let Some(entity) = ownership.mouse_capture {
                commands.queue(move |world: &mut World| focus_menu_control(world, entity));
            } else {
                commands.queue(clear_menu_focus);
            }
        }
        if mouse.just_released(MouseButton::Left)
            && let Some(entity) = ownership.mouse_capture.take()
            && !ownership.mouse_cancelled
            && ownership.hovered == Some(entity)
        {
            commands.trigger(Activate { entity });
        }
        ownership.mouse_pressed = mouse.pressed(MouseButton::Left);
    }
    ownership.blocks_mouse = captured_this_frame || ownership.mouse_cancelled || !focused;
    ownership.blocks_scroll = ownership.blocks_mouse
        || match ownership.scroll_owner {
            Some(ScrollOwner::Menu) => true,
            Some(ScrollOwner::Backdrop) => false,
            None => !saw_scroll_input && ownership.hovered.is_some(),
        }
        || scroll_over_control
        || !ownership.ui_touches.is_empty();
    if !mouse.pressed(MouseButton::Left) {
        ownership.mouse_capture = None;
        ownership.mouse_cancelled = false;
    }
}

// Bevy's default dispatcher resolves focus once for an entire message batch.
// Resolve it after each event instead so Tab followed by Enter uses the new focus.
// The application disables InputDispatchPlugin to avoid dispatching keys twice.
fn dispatch_menu_key(world: &mut World, input: KeyboardInput) {
    let target = world
        .resource::<InputFocus>()
        .get()
        .filter(|entity| world.entities().contains(*entity))
        .unwrap_or(input.window);
    let window = input.window;
    world.trigger(FocusedInput::new(target, input, window));
    world.flush();
}

fn focus_menu_control(world: &mut World, entity: Entity) {
    world
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Pressed);
    world.resource_mut::<InputFocusVisible>().0 = false;
}

fn clear_menu_focus(world: &mut World) {
    world.resource_mut::<KeyboardPress>().0 = None;
    world.resource_mut::<InputFocus>().clear();
    world.resource_mut::<InputFocusVisible>().0 = false;
}

#[derive(Component)]
struct MenuShell;

#[derive(Component)]
struct MenuDecoration;

#[derive(Component)]
struct MenuActions;

#[derive(Component)]
struct MenuHeading;

fn setup_menu(mut commands: Commands) {
    commands.insert_resource(ClearColor(BACKGROUND));
    commands.spawn((
        MenuShell,
        TabGroup::new(0),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                MenuDecoration,
                Node {
                    width: px(62),
                    height: px(7),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(ACCENT),
            ),
            (
                MenuHeading,
                Text::new("GALAXY MENU"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(ACCENT),
            ),
            (
                MenuDecoration,
                Text::new("CHOOSE YOUR NEXT STEP"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(QUIET_TEXT),
            ),
            (
                MenuActions,
                Node {
                    display: Display::Grid,
                    ..default()
                },
                Children::spawn(
                    SpawnIter(
                        [
                            ("New Game", MenuAction::NewGame, MenuEmphasis::Primary),
                            ("Load Game", MenuAction::LoadGame, MenuEmphasis::Primary),
                            ("Settings", MenuAction::Settings, MenuEmphasis::Secondary),
                            ("Quit", MenuAction::Quit, MenuEmphasis::Secondary),
                        ]
                        .into_iter()
                        .filter(|(_, action, _)| !cfg!(target_os = "ios")
                            || *action != MenuAction::Quit)
                        .map(|(label, action, emphasis)| action_button(label, action, emphasis)),
                    )
                ),
            ),
        ],
    ));
}

// Four 44-point controls and three 4-point gaps fit in a 240×200 window
// with breathing room. Smaller desktop windows cannot reliably fit readable
// labels and all actions together; native window limits are managed by UIKit.
#[cfg(not(target_os = "ios"))]
fn configure_window_minimum(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.resize_constraints.min_width = window.resize_constraints.min_width.max(240.0);
        window.resize_constraints.min_height = window.resize_constraints.min_height.max(200.0);
    }
}

#[derive(Clone, Copy)]
struct MenuPresentation {
    button_height: f32,
    primary_font_size: f32,
    button_padding: f32,
    minimum_button_width: f32,
    padding: f32,
    top: f32,
    action_gap: f32,
    heading: bool,
    decoration: bool,
}

#[expect(
    clippy::type_complexity,
    reason = "Layout updates each presentation role and its optional text together"
)]
fn layout_menu(
    windows: Query<(&Window, Option<&WindowSafeArea>), With<PrimaryWindow>>,
    mut last_layout: Local<Option<(Vec2, WindowSafeArea)>>,
    mut nodes: Query<(
        &mut Node,
        Has<MenuShell>,
        Has<MenuDecoration>,
        Has<MenuActions>,
        Has<MenuHeading>,
        Option<&MenuEmphasis>,
        Option<&mut TextFont>,
    )>,
) {
    let Ok((window, safe_area)) = windows.single() else {
        return;
    };
    let safe_area = safe_area.copied().unwrap_or_default();
    let layout = (window.size(), safe_area);
    if *last_layout == Some(layout) {
        return;
    }
    *last_layout = Some(layout);
    let size = window.size()
        - Vec2::new(
            safe_area.left + safe_area.right,
            safe_area.top + safe_area.bottom,
        );
    let button_count = nodes.iter().filter(|node| node.5.is_some()).count().max(1) as u16;
    let regular = MenuPresentation {
        button_height: 64.0,
        primary_font_size: 32.0,
        button_padding: 14.0,
        minimum_button_width: 220.0,
        padding: 24.0,
        top: 40.0,
        action_gap: 8.0,
        heading: true,
        decoration: true,
    };
    let compact = MenuPresentation {
        button_height: 52.0,
        primary_font_size: 24.0,
        minimum_button_width: 180.0,
        padding: 12.0,
        top: 12.0,
        decoration: false,
        ..regular
    };
    let dense = MenuPresentation {
        button_height: 48.0,
        primary_font_size: 19.0,
        button_padding: 8.0,
        minimum_button_width: 128.0,
        heading: false,
        ..compact
    };
    let tight = MenuPresentation {
        button_height: 44.0,
        padding: 4.0,
        top: 4.0,
        action_gap: 4.0,
        ..dense
    };
    // Try each presentation from most spacious to most compact. Column count
    // follows the space required by the actual number of action rows and readable
    // cells, rather than waiting for a wide-window breakpoint to permit reflow.
    let fitted =
        [regular, compact, dense, tight]
            .into_iter()
            .find_map(|presentation| {
                let first_column =
                    if presentation.heading && !presentation.decoration && size.x >= 480.0 {
                        2
                    } else {
                        1
                    };
                let last_column = if presentation.decoration {
                    1
                } else {
                    button_count
                };
                (first_column..=last_column).find_map(|columns| {
                    let width = (size.x - presentation.padding * 2.0).min(if columns == 1 {
                        320.0
                    } else {
                        620.0
                    });
                    let cell_width =
                        (width - (columns - 1) as f32 * presentation.action_gap) / columns as f32;
                    let rows = button_count.div_ceil(columns);
                    let header_height = if presentation.decoration {
                        99.0
                    } else if presentation.heading {
                        32.0
                    } else {
                        0.0
                    };
                    let height = presentation.top
                        + header_height
                        + rows as f32 * presentation.button_height
                        + (rows - 1) as f32 * presentation.action_gap
                        + presentation.padding;
                    (cell_width >= presentation.minimum_button_width && height <= size.y)
                        .then_some((presentation, columns, width))
                })
            });
    // A minimized window can temporarily have no usable area. Keep the last
    // compact geometry finite until its normal, constrained window returns.
    let (presentation, columns, width) =
        fitted.unwrap_or((tight, 1, (size.x - tight.padding * 2.0).clamp(0.0, 320.0)));
    for (mut node, shell, decoration, actions, heading, emphasis, font) in &mut nodes {
        if shell {
            node.left = px(safe_area.left + presentation.padding);
            node.top = px(safe_area.top + presentation.top);
            node.width = px(width);
            node.row_gap = px(if presentation.decoration { 18.0 } else { 10.0 });
        } else if decoration {
            node.display = if presentation.decoration {
                Display::Flex
            } else {
                Display::None
            };
        } else if heading {
            node.display = if presentation.heading {
                Display::Flex
            } else {
                Display::None
            };
        } else if actions {
            node.grid_template_columns = RepeatedGridTrack::flex(columns, 1.0);
            node.row_gap = px(presentation.action_gap);
            node.column_gap = px(presentation.action_gap);
        } else if let Some(emphasis) = emphasis {
            node.height = px(presentation.button_height);
            node.min_height = px(presentation.button_height);
            node.padding = UiRect::axes(px(presentation.button_padding), px(8));
            if let Some(mut font) = font {
                font.font_size = FontSize::Px(match emphasis {
                    MenuEmphasis::Primary => presentation.primary_font_size,
                    MenuEmphasis::Secondary => 19.0,
                });
            }
        }
    }
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
