use std::sync::Arc;

use client::{Client, UserStore};
use gpui::{Action, Entity, IntoElement, ParentElement};
use ui::prelude::*;

pub struct EditPredictionOnboarding {
    copilot_is_configured: bool,
    continue_with_copilot: Arc<dyn Fn(&mut Window, &mut App)>,
}

impl EditPredictionOnboarding {
    pub fn new(
        _user_store: Entity<UserStore>,
        _client: Arc<Client>,
        copilot_is_configured: bool,
        _continue_with_zed_ai: Arc<dyn Fn(&mut Window, &mut App)>,
        continue_with_copilot: Arc<dyn Fn(&mut Window, &mut App)>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            copilot_is_configured,
            continue_with_copilot,
        }
    }
}

impl Render for EditPredictionOnboarding {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let github_copilot = v_flex()
            .gap_1()
            .child(Label::new(if self.copilot_is_configured {
                "Alternatively, you can continue to use GitHub Copilot as that's already set up."
            } else {
                "Alternatively, you can use GitHub Copilot as your edit prediction provider."
            }))
            .child(
                Button::new(
                    "configure-copilot",
                    if self.copilot_is_configured {
                        "Use Copilot"
                    } else {
                        "Configure Copilot"
                    },
                )
                .full_width()
                .style(ButtonStyle::Outlined)
                .on_click({
                    let callback = self.continue_with_copilot.clone();
                    move |_, window, cx| callback(window, cx)
                }),
            );

        v_flex()
            .gap_2()
            .child(Label::new(
                "Choose an edit prediction provider in settings.",
            ))
            .child(
                Button::new("configure-edit-predictions", "Configure Providers")
                    .full_width()
                    .style(ButtonStyle::Outlined)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(
                            zed_actions::OpenSettingsAt {
                                path: "edit_predictions.providers".to_string(),
                                target: None,
                            }
                            .boxed_clone(),
                            cx,
                        );
                    }),
            )
            .child(ui::Divider::horizontal())
            .child(github_copilot)
    }
}
