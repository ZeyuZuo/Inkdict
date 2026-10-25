//! HTML and CSS for ZDict render trees.
//!
//! The desktop app shows entries as HTML in its WebView: [`entry_html`]
//! prints a [`zdict_core::RenderNode`] tree and [`theme_css`] gives the
//! theme's fonts, colors and the base `.dx-*` rules that HTML expects.

mod css;
mod markup;

pub use css::theme_css;
pub use markup::{Markup, html as entry_html};
