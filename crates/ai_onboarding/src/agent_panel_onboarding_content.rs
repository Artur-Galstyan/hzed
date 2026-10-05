use std::sync::Arc;

use client::{Client, UserStore};
use gpui::{Entity, IntoElement, ParentElement};
use language_model::{LanguageModelRegistry, ZED_CLOUD_PROVIDER_ID};
use ui::prelude::*;

use crate::{AgentPanelOnboardingCard, ApiKeysWithoutProviders};

pub struct AgentPanelOnboarding {
    has_configured_providers: bool,
    continue_with_provider: Arc<dyn Fn(&mut Window, &mut App)>,
}

impl AgentPanelOnboarding {
    pub fn new(
        _user_store: Entity<UserStore>,
        _client: Arc<Client>,
        continue_with_provider: impl Fn(&mut Window, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(
            &LanguageModelRegistry::global(cx),
            |this: &mut Self, _registry, event: &language_model::Event, cx| match event {
                language_model::Event::ProviderStateChanged(_)
                | language_model::Event::AddedProvider(_)
                | language_model::Event::RemovedProvider(_)
                | language_model::Event::ProvidersChanged => {
                    this.has_configured_providers = Self::has_configured_providers(cx)
                }
                _ => {}
            },
        )
        .detach();

        Self {
            has_configured_providers: Self::has_configured_providers(cx),
            continue_with_provider: Arc::new(continue_with_provider),
        }
    }

    fn has_configured_providers(cx: &App) -> bool {
        LanguageModelRegistry::read_global(cx)
            .visible_providers()
            .iter()
            .any(|provider| provider.is_authenticated(cx) && provider.id() != ZED_CLOUD_PROVIDER_ID)
    }
}

impl Render for AgentPanelOnboarding {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AgentPanelOnboardingCard::new()
            .when(self.has_configured_providers, |card| {
                card.child(
                    Button::new(
                        "continue-with-provider",
                        "Continue with configured provider",
                    )
                    .full_width()
                    .on_click({
                        let callback = self.continue_with_provider.clone();
                        move |_, window, cx| callback(window, cx)
                    }),
                )
            })
            .when(!self.has_configured_providers, |card| {
                card.child(ApiKeysWithoutProviders::new())
            })
    }
}
