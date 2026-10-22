use crate::{ControlStyle, MUTED, PANEL, Progress, ProgressFill, SelectionMark, Tone};
use bevy::{
    input_focus::tab_navigation::TabIndex,
    picking::{Pickable, hover::Hovered},
    prelude::*,
    ui::Checked,
    ui_widgets::{Button, Checkbox, RadioButton, RadioGroup},
};

/// Typography roles keep most screens free of font sizes and raw colors.
#[derive(Clone, Copy)]
pub enum TextStyle {
    Body,
    Muted,
    Caption,
    Value,
}
impl TextStyle {
    fn appearance(self) -> (f32, Color) {
        match self {
            Self::Body => (16.0, Tone::Peach.color()),
            Self::Muted => (14.0, MUTED),
            Self::Caption => (12.0, MUTED),
            Self::Value => (22.0, Tone::Peach.color()),
        }
    }
}

/// Native entities for binding app-specific components or attaching observers.
#[derive(Clone, Copy, Debug)]
pub struct Widget {
    pub entity: Entity,
    pub label: Entity,
}

/// A scoped builder. Children flow in order; no screen coordinates are needed.
/// Returned entities remain ordinary Bevy entities and are owned by the screen hierarchy.
pub struct Ui<'a, 'w, 's> {
    pub(crate) commands: &'a mut Commands<'w, 's>,
    pub(crate) parent: Entity,
}
impl Ui<'_, '_, '_> {
    /// Spawn a raw Bevy bundle under this container (the custom-widget escape hatch).
    pub fn spawn(&mut self, bundle: impl Bundle) -> Entity {
        let entity = self.commands.spawn(bundle).id();
        self.commands.entity(self.parent).add_child(entity);
        entity
    }
    /// Attach application bindings to any returned entity, including a widget's label.
    pub fn bind(&mut self, entity: Entity, bundle: impl Bundle) {
        self.commands.entity(entity).insert(bundle);
    }
    /// A horizontal flow with standard spacing. Use weighted columns to divide its width.
    pub fn row(&mut self, children: impl FnOnce(&mut Ui)) -> Entity {
        self.container(
            Node {
                flex_direction: FlexDirection::Row,
                ..flow()
            },
            children,
        )
    }
    /// A vertical column in a row. `weight` determines its share of the available width.
    /// Panics if the weight is non-finite or non-positive.
    pub fn column(&mut self, weight: f32, children: impl FnOnce(&mut Ui)) -> Entity {
        assert!(
            weight.is_finite() && weight > 0.0,
            "column weight must be positive and finite"
        );
        self.container(
            Node {
                flex_grow: weight,
                flex_basis: px(0),
                width: Val::Auto,
                height: percent(100),
                ..flow()
            },
            children,
        )
    }
    /// A vertical stack using the shared control spacing.
    pub fn stack(&mut self, children: impl FnOnce(&mut Ui)) -> Entity {
        self.container(flow(), children)
    }
    /// A heading and its contents, stacked vertically.
    pub fn section(&mut self, title: &str, tone: Tone, children: impl FnOnce(&mut Ui)) -> Entity {
        self.stack(|ui| {
            ui.spawn(text_bundle(title, 18.0, tone.color()));
            children(ui);
        })
    }
    /// Fill unused space in a row or column, e.g. to align action buttons at the bottom.
    pub fn spacer(&mut self) -> Entity {
        self.spawn(Node {
            flex_grow: 1.0,
            min_height: px(0),
            min_width: px(0),
            ..default()
        })
    }
    pub fn text(&mut self, text: impl Into<String>, style: TextStyle) -> Entity {
        let (size, color) = style.appearance();
        self.spawn((
            text_bundle(text, size, color),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
        ))
    }
    /// Text with an application component for querying and updating `Text` later.
    pub fn readout(
        &mut self,
        text: impl Into<String>,
        style: TextStyle,
        binding: impl Bundle,
    ) -> Entity {
        let entity = self.text(text, style);
        self.bind(entity, binding);
        entity
    }
    pub fn separator(&mut self, tone: Tone) -> Entity {
        self.spawn((
            Node {
                width: percent(100),
                height: px(4),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tone.color()),
            Pickable::IGNORE,
        ))
    }
    /// An outlined button emitting Bevy `Activate` events.
    pub fn button(&mut self, text: &str, tone: Tone, binding: impl Bundle) -> Widget {
        self.control(text, tone, binding, Kind::Button, false)
    }
    /// A filled button for the main action in a section.
    pub fn primary_button(&mut self, text: &str, tone: Tone, binding: impl Bundle) -> Widget {
        self.control(text, tone, binding, Kind::Button, true)
    }
    /// A controlled checkbox emitting `ValueChange<bool>`; the app owns `Checked`.
    pub fn toggle(
        &mut self,
        text: &str,
        tone: Tone,
        checked: bool,
        binding: impl Bundle,
    ) -> Widget {
        self.control(text, tone, binding, Kind::Toggle(checked), false)
    }
    /// A controlled, keyboard-navigable radio group. Values are application components.
    /// The app handles `ValueChange<bool>` and updates `Checked` on the options.
    /// Panics before spawning if `selected` does not match exactly one option.
    pub fn radio_group<T: Component + PartialEq, S: Into<String>>(
        &mut self,
        tone: Tone,
        selected: T,
        options: impl IntoIterator<Item = (T, S)>,
    ) -> Entity {
        let options: Vec<_> = options.into_iter().collect();
        assert_eq!(
            options
                .iter()
                .filter(|(value, _)| value == &selected)
                .count(),
            1,
            "radio group must have exactly one initially selected option"
        );
        let group = self.spawn((flow(), RadioGroup, TabIndex(0)));
        let mut ui = Ui {
            commands: self.commands,
            parent: group,
        };
        for (value, label) in options {
            let checked = value == selected;
            ui.control(&label.into(), tone, value, Kind::Radio(checked), false);
        }
        group
    }
    /// A progress track. Update `Progress` on the returned entity to change its fill.
    pub fn progress(&mut self, value: f32, tone: Tone, binding: impl Bundle) -> Entity {
        let track = self.spawn((
            Node {
                width: percent(100),
                height: px(10),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(PANEL),
            Progress(value),
            binding,
        ));
        self.commands.entity(track).with_children(|p| {
            p.spawn((
                Node {
                    width: px(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(tone.color()),
                ProgressFill,
            ));
        });
        track
    }
    /// A custom region for charts or other Bevy UI. The caller supplies its dimensions.
    pub fn custom(
        &mut self,
        node: Node,
        children: impl FnOnce(&mut ChildSpawnerCommands),
    ) -> Entity {
        let entity = self.spawn((node, BackgroundColor(PANEL)));
        self.commands.entity(entity).with_children(children);
        entity
    }
    fn container(&mut self, node: Node, children: impl FnOnce(&mut Ui)) -> Entity {
        let parent = self.spawn(node);
        children(&mut Ui {
            commands: self.commands,
            parent,
        });
        parent
    }
    fn control(
        &mut self,
        text: &str,
        tone: Tone,
        binding: impl Bundle,
        kind: Kind,
        primary: bool,
    ) -> Widget {
        let entity = self.spawn((
            binding,
            ControlStyle { tone, primary },
            Hovered::default(),
            Node {
                min_width: px(0),
                flex_shrink: 0.0,
                height: px(48),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::MAX,
                padding: UiRect::horizontal(px(16)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(tone.color()),
        ));
        match kind {
            Kind::Button => {
                self.commands.entity(entity).insert((Button, TabIndex(0)));
            }
            Kind::Toggle(checked) => {
                self.commands.entity(entity).insert((Checkbox, TabIndex(0)));
                if checked {
                    self.commands.entity(entity).insert(Checked);
                }
            }
            Kind::Radio(checked) => {
                self.commands.entity(entity).insert(RadioButton);
                if checked {
                    self.commands.entity(entity).insert(Checked);
                }
            }
        }
        let mut ui = Ui {
            commands: self.commands,
            parent: entity,
        };
        let label = ui.spawn(text_bundle(text, 17.0, tone.color()));
        if !matches!(kind, Kind::Button) {
            ui.spawn((text_bundle("", 17.0, tone.color()), SelectionMark));
        }
        Widget { entity, label }
    }
}

enum Kind {
    Button,
    Toggle(bool),
    Radio(bool),
}
pub(crate) fn flow() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        row_gap: px(12),
        column_gap: px(12),
        ..default()
    }
}
pub(crate) fn text_bundle(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Pickable::IGNORE,
    )
}
