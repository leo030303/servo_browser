use egui::{Button, Layout, Vec2, WidgetInfo, WidgetType};
use servo::WebView;

use crate::{browser_window::BrowserWindow, running_app_state::UserInterfaceCommand};

use super::gui::{FAVICON_SIZE, TAB_WIDTH};

/// Draws a browser tab, checking for clicks and queues appropriate [`UserInterfaceCommand`]s.
/// Using a custom widget here would've been nice, but it doesn't seem as though egui
/// supports that, so we arrange multiple Widgets in a way that they look connected.
pub fn create_browser_tab(
    ui: &mut egui::Ui,
    window: &BrowserWindow,
    webview: WebView,
    event_queue: &mut Vec<UserInterfaceCommand>,
    favicon_texture: Option<egui::load::SizedTexture>,
    theme: winit::window::Theme,
) {
    let label = match (webview.page_title(), webview.url()) {
        (Some(title), _) if !title.is_empty() => title,
        (_, Some(url)) => url.to_string(),
        _ => "New Tab".into(),
    };

    let inactive_bg_color = ui.visuals().window_fill;
    let active_bg_color = ui.visuals().widgets.active.weak_bg_fill;
    let active = window.active_webview().map(|webview| webview.id()) == Some(webview.id());

    // Setup a tab frame that will contain the favicon, title and close button
    let mut tab_frame = egui::Frame::NONE.corner_radius(4).begin(ui);
    {
        tab_frame.content_ui.add_space(5.0);
        tab_frame.content_ui.with_layout(
            Layout::left_to_right(egui::Align::Center),
            |tab_frame_ui| {
                let visuals = tab_frame_ui.visuals_mut();
                // Remove the stroke so we don't see the border between the close button and the label
                visuals.widgets.active.bg_stroke.width = 0.0;
                visuals.widgets.hovered.bg_stroke.width = 0.0;
                // Now we make sure the fill color is always the same, irrespective of state, that way
                // we can make sure that both the label and close button have the same background color
                visuals.widgets.noninteractive.weak_bg_fill = inactive_bg_color;
                visuals.widgets.inactive.weak_bg_fill = inactive_bg_color;
                visuals.widgets.hovered.weak_bg_fill = active_bg_color;
                visuals.widgets.active.weak_bg_fill = active_bg_color;
                visuals.selection.bg_fill = active_bg_color;
                visuals.selection.stroke.color = visuals.widgets.active.fg_stroke.color;
                visuals.widgets.hovered.fg_stroke.color = visuals.widgets.active.fg_stroke.color;

                // Expansion would also show that they are 2 separate widgets
                visuals.widgets.active.expansion = 0.0;
                visuals.widgets.hovered.expansion = 0.0;

                if let Some(favicon) = favicon_texture {
                    tab_frame_ui.add(
                        egui::Image::from_texture(favicon)
                            .fit_to_exact_size(egui::vec2(FAVICON_SIZE, FAVICON_SIZE))
                            .bg_fill(egui::Color32::TRANSPARENT),
                    );
                }

                let tab = tab_frame_ui
                    .add(
                        Button::selectable(active, truncate_with_ellipsis(&label, 16)).min_size(
                            Vec2::new(TAB_WIDTH - FAVICON_SIZE - 40.0 - FAVICON_SIZE, 0.0),
                        ),
                    )
                    .on_hover_ui(|ui| {
                        ui.label(&label);
                    });

                let close_button = tab_frame_ui.add(
                    egui::Button::image(match theme {
                        winit::window::Theme::Dark => {
                            egui::include_image!("../../resources/icons/close_dark.svg")
                        }
                        winit::window::Theme::Light => {
                            egui::include_image!("../../resources/icons/close_light.svg")
                        }
                    })
                    .fill(egui::Color32::TRANSPARENT)
                    .min_size(Vec2::new(FAVICON_SIZE, FAVICON_SIZE)),
                );
                close_button.widget_info(|| {
                    let mut info = WidgetInfo::new(WidgetType::Button);
                    info.label = Some("Close".into());
                    info
                });
                if close_button.clicked() || close_button.middle_clicked() || tab.middle_clicked() {
                    event_queue.push(UserInterfaceCommand::CloseWebView(webview.id()))
                } else if !active && tab.clicked() {
                    window.activate_webview(webview.id());
                }
            },
        );
    }

    let response = tab_frame.allocate_space(ui);
    let fill_color = if active || response.hovered() {
        active_bg_color
    } else {
        inactive_bg_color
    };
    tab_frame.frame.fill = fill_color;
    tab_frame.end(ui);
}

fn truncate_with_ellipsis(input: &str, max_length: usize) -> String {
    if input.chars().count() > max_length {
        let truncated: String = input.chars().take(max_length.saturating_sub(1)).collect();
        format!("{truncated}…")
    } else {
        input.to_string()
    }
}
