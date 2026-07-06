use crate::model::shape::{CommonObjAttr, TextWrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCompatibilityOptions {
    pub clamp_non_overlay_body_tables_to_flow: bool,
}

impl RenderCompatibilityOptions {
    pub const RHWP_NATIVE: Self = Self {
        clamp_non_overlay_body_tables_to_flow: false,
    };

    pub const HANCOM_RENDER_COMPATIBILITY: Self = Self {
        clamp_non_overlay_body_tables_to_flow: true,
    };
}

impl Default for RenderCompatibilityOptions {
    fn default() -> Self {
        Self::RHWP_NATIVE
    }
}

pub(crate) fn should_clamp_non_overlay_body_table_to_flow(
    options: RenderCompatibilityOptions,
    common: &CommonObjAttr,
    depth: usize,
    body_flow_context: bool,
) -> bool {
    options.clamp_non_overlay_body_tables_to_flow
        && depth == 0
        && body_flow_context
        && !common.treat_as_char
        && !matches!(
            common.text_wrap,
            TextWrap::BehindText | TextWrap::InFrontOfText
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_compatibility_options_default_to_rhwp_native() {
        assert_eq!(
            RenderCompatibilityOptions::default(),
            RenderCompatibilityOptions::RHWP_NATIVE
        );
    }

    #[test]
    fn hancom_render_compatibility_clamps_non_overlay_body_tables_only() {
        let mut common = CommonObjAttr::default();
        common.text_wrap = TextWrap::TopAndBottom;

        assert!(should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            &common,
            0,
            true,
        ));

        assert!(!should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::RHWP_NATIVE,
            &common,
            0,
            true,
        ));
        assert!(!should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            &common,
            1,
            true,
        ));
        assert!(!should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            &common,
            0,
            false,
        ));

        common.treat_as_char = true;
        assert!(!should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            &common,
            0,
            true,
        ));
    }

    #[test]
    fn hancom_render_compatibility_does_not_clamp_overlay_tables() {
        let mut common = CommonObjAttr::default();

        common.text_wrap = TextWrap::BehindText;
        assert!(!should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            &common,
            0,
            true,
        ));

        common.text_wrap = TextWrap::InFrontOfText;
        assert!(!should_clamp_non_overlay_body_table_to_flow(
            RenderCompatibilityOptions::HANCOM_RENDER_COMPATIBILITY,
            &common,
            0,
            true,
        ));
    }
}
