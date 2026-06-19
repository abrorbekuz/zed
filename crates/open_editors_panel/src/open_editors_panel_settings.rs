use gpui::Pixels;
use settings::RegisterSetting;
pub use settings::{DockSide, Settings};

#[derive(Debug, Clone, Copy, PartialEq, RegisterSetting)]
pub struct OpenEditorsPanelSettings {
    pub default_width: Pixels,
    pub dock: DockSide,
}

impl Settings for OpenEditorsPanelSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        let panel = content.open_editors_panel.as_ref().unwrap();
        Self {
            default_width: panel.default_width.map(gpui::px).unwrap(),
            dock: panel.dock.unwrap(),
        }
    }
}
