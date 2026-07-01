//! Improv mode system prompt generation.
//!
//! Prepended to the effective input before inference so the model
//! adopts the specified interaction posture.

/// Generate a system-prompt instruction for the active improv mode.
///
/// Prepended to the effective input before inference so the model
/// adopts the specified interaction posture. Each mode has a concise
/// instruction that encodes its core constraint.
pub(super) fn improv_system_prompt(mode: &hkask_improv::ImprovMode) -> String {
    match mode {
        hkask_improv::ImprovMode::Plussing => {
            "[Improv mode: Plussing]\n\
             Find what you can agree with in the user's message. Build constructively on those points.\n\
             Silently omit anything you disagree with — never explicitly negate or reject.\n\
             If nothing is agreeable, redirect constructively: \"Let's explore this from a different angle.\"".to_string()
        }
        hkask_improv::ImprovMode::YesAnd => {
            "[Improv mode: Yes And]\n\
             Accept the user's entire message as valid. Extend it with a novel, additive layer.\n\
             Your extension must build on their contribution, not replace or contradict it.\n\
             Start with \"Yes, and also:\" or equivalent acceptance language.".to_string()
        }
        hkask_improv::ImprovMode::YesBut => {
            "[Improv mode: Yes But]\n\
             Accept the user's entire message as valid. Then append a constructive constraint\n\
             or boundary condition that narrows scope without contradicting.\n\
             Frame as additive guidance: \"Yes, and let's also consider...\" not \"No, because...\"\n\
             Never use rejecting language (no, wrong, can't, impossible).".to_string()
        }
        hkask_improv::ImprovMode::Freestyling { .. } => {
            "[Improv mode: Freestyling]\n\
             Engage in rapid, associative, creative response. Keep responses short (1-3 sentences).\n\
             Build on the energy of the conversation — this is creative exploration, not careful analysis.\n\
             Take creative leaps. Connect ideas associatively. Don't over-think.".to_string()
        }
        hkask_improv::ImprovMode::Riffing { .. } => {
            "[Improv mode: Riffing]\n\
             Take one idea from the user's message and explore it independently as a solo tangent.\n\
             Go deep, go wide, go creative — this is your independent exploration space.\n\
             When done, either return to the main topic with a synthesis of your findings,\n\
             or signal that this tangent deserves its own thread.".to_string()
        }
        hkask_improv::ImprovMode::Cascade(c) => {
            let step_labels: Vec<String> = c
                .modes
                .iter()
                .map(|m| m.label().to_string())
                .collect();
            format!(
                "[Improv mode: Cascade — {}]\n\
                 Apply these improv modes in sequence to your response:\n\
                 {}\n\
                 Each step's output feeds into the next. Stay within the matryoshka limit of 7 total applications.",
                step_labels.join(" → "),
                step_labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| format!("  {}. {}", i + 1, label))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}
