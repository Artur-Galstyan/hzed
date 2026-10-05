use std::sync::Arc;

use anyhow::Result;
use copilot_chat::{
    CopilotChat, CopilotChatConfiguration, PROVIDER_ID, PROVIDER_NAME, language_model,
};
use futures::{FutureExt as _, future::BoxFuture};
use gpui::{App, AsyncApp, Entity, Subscription, Task};
use language_model::{
    AuthenticateError, IconOrSvg, LanguageModel, LanguageModelClient, LanguageModelCompletionError,
    LanguageModelCompletionStream, LanguageModelProvider, LanguageModelProviderId,
    LanguageModelProviderName, LanguageModelProviderState, LanguageModelRequest, ModelRateLimiters,
    ProviderSettingsView, unavailable_error,
};
use settings::{CopilotSettings, Settings, SettingsStore};
use ui::prelude::*;

pub struct CopilotChatLanguageModelProvider {
    state: Entity<State>,
    request_limiters: ModelRateLimiters,
}

pub struct State {
    _copilot_chat_subscription: Option<Subscription>,
    _settings_subscription: Subscription,
}

impl State {
    fn is_authenticated(&self, cx: &App) -> bool {
        CopilotChat::global(cx)
            .map(|chat| {
                let chat = chat.read(cx);
                chat.is_authenticated() && chat.models().is_some_and(|models| !models.is_empty())
            })
            .unwrap_or(false)
    }
}

impl CopilotChatLanguageModelProvider {
    pub fn new(cx: &mut App) -> Self {
        let state = cx.new(|cx| {
            let copilot_chat_subscription = CopilotChat::global(cx)
                .map(|copilot_chat| cx.observe(&copilot_chat, |_, _, cx| cx.notify()));
            State {
                _copilot_chat_subscription: copilot_chat_subscription,
                _settings_subscription: cx.observe_global::<SettingsStore>(|_, cx| {
                    if let Some(copilot_chat) = CopilotChat::global(cx) {
                        let configuration = CopilotChatConfiguration {
                            enterprise_uri: CopilotSettings::get_global(cx).enterprise_uri.clone(),
                        };
                        copilot_chat.update(cx, |chat, cx| {
                            chat.set_configuration(configuration, cx);
                        });
                    }
                    cx.notify();
                }),
            }
        });

        Self {
            state,
            request_limiters: ModelRateLimiters::default(),
        }
    }

    /// The current configuration of `model`, and the Copilot Chat client that
    /// serves it, if Copilot Chat still offers it.
    fn config(
        &self,
        model: &LanguageModel,
        cx: &App,
    ) -> Result<(copilot_chat::Model, Entity<CopilotChat>), LanguageModelCompletionError> {
        let unavailable = || unavailable_error(model);
        let copilot_chat = CopilotChat::global(cx).ok_or_else(unavailable)?;
        let config = copilot_chat
            .read(cx)
            .models()
            .and_then(|models| {
                models
                    .iter()
                    .find(|config| config.id() == model.id.0.as_ref())
            })
            .cloned()
            .ok_or_else(unavailable)?;
        Ok((config, copilot_chat))
    }
}

impl LanguageModelProviderState for CopilotChatLanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for CopilotChatLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::Icon(IconName::Copilot)
    }

    fn default_model(&self, cx: &App) -> Option<LanguageModel> {
        let copilot_chat = CopilotChat::global(cx)?;
        Some(language_model(copilot_chat.read(cx).models()?.first()?))
    }

    fn default_fast_model(&self, cx: &App) -> Option<LanguageModel> {
        // The default model should be Copilot Chat's 'base model', which is likely a relatively fast
        // model (e.g. 4o) and a sensible choice when considering premium requests
        self.default_model(cx)
    }

    fn provided_models(&self, cx: &App) -> Vec<LanguageModel> {
        let Some(copilot_chat) = CopilotChat::global(cx) else {
            return Vec::new();
        };
        let Some(models) = copilot_chat.read(cx).models() else {
            return Vec::new();
        };
        models.iter().map(language_model).collect()
    }

    fn is_authenticated(&self, cx: &App) -> bool {
        self.state.read(cx).is_authenticated(cx)
    }

    fn authenticate(&self, cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        if self.is_authenticated(cx) {
            return Task::ready(Ok(()));
        }
        let Some(copilot_chat) = CopilotChat::global(cx) else {
            return Task::ready(Err(AuthenticateError::CredentialsNotFound));
        };
        copilot_chat.update(cx, |chat, cx| chat.refresh_models(cx))
    }

    fn settings_view(&self, cx: &mut App) -> Option<ProviderSettingsView> {
        let is_authenticated = CopilotChat::global(cx)
            .map(|chat| chat.read(cx).is_authenticated())
            .unwrap_or(false);
        let title = if is_authenticated {
            None
        } else {
            Some("Configure Copilot Chat".into())
        };
        let description = if is_authenticated {
            None
        } else {
            Some(language_model::InlineDescription::Text(
                "Requires an active GitHub Copilot subscription.".into(),
            ))
        };

        Some(ProviderSettingsView::Inline(
            language_model::InlineProviderSettings {
                title,
                description,
                create_view: Arc::new(|_window, cx| {
                    cx.new(|cx| {
                        copilot_ui::ConfigurationView::new(
                            |cx| {
                                CopilotChat::global(cx)
                                    .map(|model| model.read(cx).is_authenticated())
                                    .unwrap_or(false)
                            },
                            copilot_ui::ConfigurationMode::Chat,
                            cx,
                        )
                        .compact()
                    })
                    .into()
                }),
            },
        ))
    }

    fn set_api_key(&self, key: Option<String>, cx: &mut App) -> Task<Result<()>> {
        // Copilot authenticates via an OAuth device flow rather than an API key,
        // so the only meaningful credential change here is clearing it (which
        // signs the user out of the agent provider).
        if key.is_some() {
            return Task::ready(Ok(()));
        }
        let Some(copilot_chat) = CopilotChat::global(cx) else {
            return Task::ready(Ok(()));
        };
        copilot_chat.update(cx, |chat, cx| chat.sign_out(cx))
    }
}

impl LanguageModelClient for CopilotChatLanguageModelProvider {
    fn stream_completion(
        &self,
        model: &LanguageModel,
        request: LanguageModelRequest,
        cx: &AsyncApp,
    ) -> BoxFuture<'static, Result<LanguageModelCompletionStream, LanguageModelCompletionError>>
    {
        let (config, copilot_chat) = match cx.update(|cx| self.config(model, cx)) {
            Ok(config) => config,
            Err(error) => return async move { Err(error) }.boxed(),
        };
        let request_limiter = self.request_limiters.for_model(&model.id);
        copilot_chat::stream_completion(&config, &copilot_chat, &request_limiter, request, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use credentials_provider::CredentialsProvider;
    use gpui::{TestAppContext, UpdateGlobal as _};
    use http_client::{AsyncBody, FakeHttpClient};
    use parking_lot::Mutex;
    use std::{
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct StoredTokenCredentialsProvider(Arc<AtomicUsize>);

    impl CredentialsProvider for StoredTokenCredentialsProvider {
        fn read_credentials<'a>(
            &'a self,
            _url: &'a str,
            _cx: &'a AsyncApp,
        ) -> Pin<Box<dyn Future<Output = Result<Option<(String, Vec<u8>)>>> + 'a>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(Some(("Bearer".into(), b"saved-token".to_vec()))) })
        }

        fn write_credentials<'a>(
            &'a self,
            _url: &'a str,
            _username: &'a str,
            _password: &'a [u8],
            _cx: &'a AsyncApp,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
            Box::pin(async { Ok(()) })
        }

        fn delete_credentials<'a>(
            &'a self,
            _url: &'a str,
            _cx: &'a AsyncApp,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[gpui::test]
    async fn saved_token_waits_for_explicit_model_discovery(cx: &mut TestAppContext) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let http_client = FakeHttpClient::create({
            let requests = requests.clone();
            move |request| {
                let requests = requests.clone();
                async move {
                    let path = request.uri().path().to_string();
                    requests.lock().push(path.clone());
                    anyhow::ensure!(
                        request
                            .headers()
                            .get("Authorization")
                            .and_then(|value| value.to_str().ok())
                            == Some("Bearer saved-token"),
                        "expected stored Copilot token"
                    );
                    let body = match path.as_str() {
                        "/graphql" => {
                            r#"{"data":{"viewer":{"copilotEndpoints":{"api":"https://api.example.test"}}}}"#
                        }
                        "/models"
                            if requests
                                .lock()
                                .iter()
                                .filter(|request| request.as_str() == "/models")
                                .count()
                                == 1 =>
                        {
                            r#"{"data":[]}"#
                        }
                        "/models" => {
                            r#"{"data":[{"billing":{"is_premium":false,"multiplier":0},"capabilities":{"family":"test","supports":{},"type":"chat"},"id":"test-model","name":"Test Model","vendor":"OpenAI","is_chat_default":true,"is_chat_fallback":false,"model_picker_enabled":true}]}"#
                        }
                        _ => anyhow::bail!("unexpected Copilot request: {path}"),
                    };
                    Ok(http_client::Response::builder()
                        .status(200)
                        .body(AsyncBody::from(body))?)
                }
            }
        });
        let credential_reads = Arc::new(AtomicUsize::new(0));
        let provider = cx.update(|cx| {
            let store = SettingsStore::test(cx);
            cx.set_global(store);
            CopilotSettings::register(cx);
            copilot_chat::init(
                http_client,
                Arc::new(StoredTokenCredentialsProvider(credential_reads.clone())),
                CopilotChatConfiguration::default(),
                cx,
            );
            CopilotChatLanguageModelProvider::new(cx)
        });

        cx.run_until_parked();
        cx.update(|cx| {
            assert!(
                CopilotChat::global(cx)
                    .expect("Copilot Chat")
                    .read(cx)
                    .is_authenticated()
            );
            assert!(!provider.is_authenticated(cx));
        });
        assert!(
            requests.lock().is_empty(),
            "constructor must not make HTTP requests"
        );

        cx.update(|cx| {
            SettingsStore::update_global(cx, |store, cx| {
                store
                    .set_user_settings(
                        r#"{"copilot":{"enterprise_uri":"https://enterprise.example.test"}}"#,
                        cx,
                    )
                    .expect("Copilot settings should load");
            });
        });
        cx.run_until_parked();
        assert!(credential_reads.load(Ordering::SeqCst) > 0);
        assert!(
            requests.lock().is_empty(),
            "configuration must not make HTTP requests"
        );

        cx.update(|cx| provider.authenticate(cx))
            .await
            .expect("explicit authentication should discover models");
        cx.update(|cx| {
            let chat = CopilotChat::global(cx).expect("Copilot Chat");
            assert!(chat.read(cx).is_authenticated());
            assert_eq!(chat.read(cx).models().map(|models| models.len()), Some(0));
            assert!(!provider.is_authenticated(cx));
        });
        cx.update(|cx| provider.authenticate(cx))
            .await
            .expect("authentication should retry when no models are available");
        cx.update(|cx| {
            assert!(provider.is_authenticated(cx));
            assert_eq!(provider.provided_models(cx).len(), 1);
        });
        assert_eq!(
            requests.lock().clone(),
            vec![
                "/graphql".to_string(),
                "/models".to_string(),
                "/graphql".to_string(),
                "/models".to_string(),
            ]
        );
    }
}
