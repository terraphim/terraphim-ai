use gpui::*;
use gpui_component::{StyledExt, button::*};

use crate::models::{ChipOperator, TermChip, TermChipSet};
use crate::theme::colors::theme;

/// Term chips component for displaying parsed query terms
pub struct TermChips {
    chips: TermChipSet,
}

impl TermChips {
    pub fn new(chips: TermChipSet) -> Self {
        Self { chips }
    }

    /// Render a single term chip
    fn render_chip(&self, chip: &TermChip, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let value = chip.value.clone();
        let is_from_kg = chip.is_from_kg;
        let idx = index;

        let mut chip_div = div()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(theme::surface())
            .border_1()
            .border_color(if is_from_kg {
                theme::primary()
            } else {
                theme::border()
            })
            .child(div().flex().items_center().gap_1());

        // Add KG icon if from knowledge graph
        if is_from_kg {
            chip_div = chip_div.child(div().text_sm().text_color(theme::primary()).child("KG"));
        }

        // Add term value
        chip_div = chip_div.child(
            div()
                .text_sm()
                .text_color(theme::text_primary())
                .child(value.clone()),
        );

        // Add remove button
        chip_div.child(
            Button::new(("remove-chip", idx))
                .label("×")
                .ghost()
                .on_click(cx.listener(move |_this, _ev, _window, _cx| {
                    // TODO: Emit event to remove chip
                    log::info!("Remove chip at index: {}", idx);
                })),
        )
    }

    /// Render operator indicator between chips
    fn render_operator(&self, operator: ChipOperator) -> impl IntoElement {
        let operator_text = match operator {
            ChipOperator::And => "AND",
            ChipOperator::Or => "OR",
        };

        div()
            .px_2()
            .py_1()
            .text_sm()
            .font_medium()
            .text_color(theme::text_secondary())
            .child(operator_text)
    }
}

impl Render for TermChips {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.chips.chips.is_empty() {
            return div().into_any_element();
        }

        let chips = self.chips.chips.clone();
        let operator = self.chips.operator;

        // Build a simple list of chips with operators
        let mut container = div().flex().items_center().gap_2().flex_wrap().w_full();

        for (idx, chip) in chips.iter().enumerate() {
            // Add operator before chip (except first)
            if idx > 0 {
                if let Some(op) = operator {
                    container = container.child(self.render_operator(op));
                }
            }

            // Add chip
            container = container.child(self.render_chip(chip, idx, cx));
        }

        container.into_any_element()
    }
}
