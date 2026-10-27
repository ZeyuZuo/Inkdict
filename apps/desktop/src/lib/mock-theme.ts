// Browser-only stand-in for the CSS returned by Rust; no network font imports.
// Only the variables and rules the invented entries in mock-data.ts use.
export const themeCss = `
:root{--font-ui:system-ui,"Microsoft YaHei",sans-serif;--c-fg:#1b1c1f;--c-muted:#6c6e75;--c-rule:#e4e4e0;--c-box:#f3f3f1}
@media(prefers-color-scheme:dark){:root:not([data-theme="light"]){--c-fg:#e5e5e7;--c-muted:#9b9da4;--c-rule:#2d2e33;--c-box:#1e1f23}}
:root[data-theme="dark"]{--c-fg:#e5e5e7;--c-muted:#9b9da4;--c-rule:#2d2e33;--c-box:#1e1f23}
button.dx-icon{border:0;background:none;color:inherit;cursor:pointer;padding:0 .12em;font:inherit;line-height:1}
button.dx-icon:hover{opacity:.7}
.dx-marker{position:absolute;left:0;text-align:left}
.dx-link{color:inherit;text-decoration:none;border-bottom:1px solid var(--c-rule)}.dx-link:hover{text-decoration:underline}
.dx-title{font-family:var(--font-ui);font-weight:700;font-size:.8em;letter-spacing:.06em;color:var(--c-muted);margin-bottom:.2em}
`;
