mod edit_prediction_button;
mod edit_prediction_context_view;

use command_palette_hooks::CommandPaletteFilter;
use edit_prediction::ResetOnboarding;
use edit_prediction_context_view::EditPredictionContextView;
use gpui::actions;
use project::DisableAiSettings;
use settings::{Settings as _, SettingsStore};
use std::any::{Any as _, TypeId};
use ui::{App, prelude::*};
use workspace::{SplitDirection, Workspace};

pub use edit_prediction_button::{
    EditPredictionButton, ToggleMenu, get_available_providers, set_completion_provider,
};

actions!(
    dev,
    [
        /// Opens the edit prediction context view.
        OpenEditPredictionContextView,
    ]
);

pub fn init(cx: &mut App) {
    feature_gate_predict_edits_actions(cx);

    cx.observe_new(move |workspace: &mut Workspace, _, _cx| {
        workspace.register_action_renderer(|div, _, _, cx| {
            div.on_action(cx.listener(
                move |workspace, _: &OpenEditPredictionContextView, window, cx| {
                    let project = workspace.project();
                    workspace.split_item(
                        SplitDirection::Right,
                        Box::new(cx.new(|cx| {
                            EditPredictionContextView::new(
                                project.clone(),
                                workspace.client(),
                                workspace.user_store(),
                                window,
                                cx,
                            )
                        })),
                        window,
                        cx,
                    );
                },
            ))
        });
    })
    .detach();
}

fn feature_gate_predict_edits_actions(cx: &mut App) {
    let reset_onboarding_action_types = [TypeId::of::<ResetOnboarding>()];
    let all_action_types = [
        TypeId::of::<edit_prediction::ResetOnboarding>(),
        zed_actions::OpenZedPredictOnboarding.type_id(),
        TypeId::of::<edit_prediction::ClearHistory>(),
    ];

    CommandPaletteFilter::update_global(cx, |filter, _cx| {
        filter.hide_action_types(&reset_onboarding_action_types);
        filter.hide_action_types(&[zed_actions::OpenZedPredictOnboarding.type_id()]);
    });

    cx.observe_global::<SettingsStore>(move |cx| {
        if DisableAiSettings::get_global(cx).disable_ai {
            CommandPaletteFilter::update_global(cx, |filter, _cx| {
                filter.hide_action_types(&all_action_types);
            });
        }
    })
    .detach();
}
