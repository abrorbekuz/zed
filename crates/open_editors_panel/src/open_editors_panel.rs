use crate::open_editors_panel_settings::{DockSide, OpenEditorsPanelSettings};
use settings::{Settings, SettingsStore};

use editor::{Editor, EditorEvent};
use gpui::*;
use project::Fs;

use std::sync::Arc;
use ui::prelude::*;
use ui::{IconButtonShape, Tooltip};
use workspace::{
    Panel, Workspace,
    dock::{DockPosition, PanelEvent},
};

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _window, _cx| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<OpenEditorsPanel>(window, cx);
        });
    })
    .detach();
}

pub struct OpenEditorsPanel {
    fs: Arc<dyn Fs>,
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    search_query: String,
    filter_editor: Entity<Editor>,
    _subscription: Subscription,
}

impl OpenEditorsPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, window, cx| {
            OpenEditorsPanel::new(workspace, window, cx)
        })
    }

    pub fn new(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let weak = cx.weak_entity();
        let focus_handle = cx.focus_handle();
        let fs = workspace.project().read(cx).fs().clone();

        cx.new(|cx| {
            let filter_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search open editors…", window, cx);
                editor
            });

            let subscription = cx.subscribe_in(
                &filter_editor,
                window,
                |this: &mut OpenEditorsPanel, _, event, _window, cx| {
                    if let EditorEvent::BufferEdited = event {
                        this.search_query = this.filter_editor.read(cx).text(cx);
                        cx.notify();
                    }
                },
            );

            // Re-render when settings change (catches dock position changes)
            cx.observe_global_in::<SettingsStore>(window, |_this, _window, cx| {
                cx.notify();
            })
            .detach();

            Self {
                fs,
                workspace: weak,
                focus_handle,
                search_query: String::new(),
                filter_editor,
                _subscription: subscription,
            }
        })
    }

    fn open_entry(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        workspace.update(cx, |workspace, cx| {
            let pane = workspace.active_pane().clone();
            pane.update(cx, |pane, cx| {
                pane.activate_item(ix, true, true, window, cx);
            });
        });
    }
}

impl Render for OpenEditorsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(workspace) = self.workspace.upgrade() else {
            return div().into_any_element();
        };

        let active_item_id = workspace
            .read(cx)
            .active_pane()
            .read(cx)
            .active_item()
            .map(|item| item.item_id());

        let items: Vec<(usize, EntityId, SharedString, bool)> = workspace
            .read(cx)
            .active_pane()
            .read(cx)
            .items()
            .enumerate()
            .map(|(ix, item)| {
                let id = item.item_id();
                let is_active = Some(id) == active_item_id;
                (ix, id, item.tab_content_text(0, cx), is_active)
            })
            .collect();

        let query = self.search_query.clone();
        let filtered: Vec<(usize, EntityId, SharedString, bool)> = if query.is_empty() {
            items
        } else {
            items
                .into_iter()
                .filter(|(_, _, label, _)| label.to_lowercase().contains(&query.to_lowercase()))
                .collect()
        };

        let theme = cx.theme();
        let colors = theme.colors();
        let hover_color = colors.element_hover;
        let selected_bg = colors.element_selected;
        let panel_bg = colors.panel_background;
        let text_color_active = colors.text;
        let text_color_muted = colors.text_muted;
        let border_color = colors.border;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel_bg)
            .child(
                div().border_b_1().border_color(border_color).child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .px_2()
                        .py_1()
                        .child(
                            Icon::new(IconName::MagnifyingGlass)
                                .size(IconSize::Small)
                                .color(Color::Muted),
                        )
                        .child(self.filter_editor.clone())
                        .when(!self.search_query.is_empty(), |this| {
                            this.child(
                                IconButton::new("clear_filter", IconName::Close)
                                    .shape(IconButtonShape::Square)
                                    .tooltip(Tooltip::text("Clear Filter"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.filter_editor.update(cx, |editor, cx| {
                                            editor.set_text("", window, cx);
                                        });
                                        cx.notify();
                                    })),
                            )
                        }),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .children(filtered.into_iter().map(|(ix, id, label, is_active)| {
                        let bg = if is_active { selected_bg } else { panel_bg };
                        let text_color = if is_active {
                            text_color_active
                        } else {
                            text_color_muted
                        };
                        div()
                            .id(("open-editor-item", id))
                            .px_3()
                            .py_1()
                            .text_sm()
                            .rounded_md()
                            .bg(bg)
                            .text_color(text_color)
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_color))
                            .child(label)
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    this.open_entry(ix, window, cx);
                                }),
                            )
                    })),
            )
            .into_any_element()
    }
}

impl Panel for OpenEditorsPanel {
    fn persistent_name() -> &'static str {
        "OpenEditorsPanel"
    }

    fn panel_key() -> &'static str {
        "OpenEditorsPanel"
    }

    fn position(&self, _window: &Window, cx: &App) -> DockPosition {
        match OpenEditorsPanelSettings::get_global(cx).dock {
            DockSide::Left => DockPosition::Left,
            DockSide::Right => DockPosition::Right,
        }
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        settings::update_settings_file(self.fs.clone(), cx, move |settings, _| {
            let dock = match position {
                DockPosition::Left | DockPosition::Bottom => DockSide::Left,
                DockPosition::Right => DockSide::Right,
            };
            settings.open_editors_panel.get_or_insert_default().dock = Some(dock);
        });
    }

    fn default_size(&self, _window: &Window, cx: &App) -> Pixels {
        OpenEditorsPanelSettings::get_global(cx).default_width
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<ui::IconName> {
        Some(ui::IconName::ListTree)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Open Editors")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        2
    }
}

impl Focusable for OpenEditorsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for OpenEditorsPanel {}

#[derive(Clone, PartialEq, Action)]
pub struct ToggleFocus;
