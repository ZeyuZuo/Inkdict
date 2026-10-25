// Browser-only stand-in for the CSS returned by Rust; no network font imports.
export const themeCss = `
:root{--font-text:Georgia,"Noto Serif CJK SC","Songti SC",serif;--font-ui:system-ui,"Microsoft YaHei",sans-serif;
--c-fg:#28332f;--c-muted:#65736b;--c-faint:#7e8a81;--c-rule:#dce2da;--c-box:#f0f3ec;--c-accent:#375d4a}
@media(prefers-color-scheme:dark){:root:not([data-theme="light"]){--c-fg:#dde5dc;--c-muted:#a4b3a7;--c-faint:#87968b;
--c-rule:#344239;--c-box:#243229;--c-accent:#b2d0ba}}
:root[data-theme="dark"]{--c-fg:#dde5dc;--c-muted:#a4b3a7;--c-faint:#87968b;--c-rule:#344239;--c-box:#243229;
--c-accent:#b2d0ba}
button.dx-icon{border:0;background:none;color:inherit;cursor:pointer;padding:0 .12em;font:inherit;line-height:1}
button.dx-icon:hover{opacity:.7}.dx-svg{display:inline-block;vertical-align:-.13em}
.dx-marker{position:absolute;left:0;text-align:left}span.dx-icon{margin-right:.28em}
details>summary{list-style:none;cursor:pointer;position:relative;padding-right:1.6em}
details>summary::-webkit-details-marker{display:none}
details>summary::after{content:"";position:absolute;right:.2em;top:.35em;width:.45em;height:.45em;border-right:2px solid var(--c-faint);
border-bottom:2px solid var(--c-faint);transform:rotate(45deg);transition:transform .15s}
details:not([open])>summary::after{transform:rotate(-45deg)}
.dx-link{color:inherit;text-decoration:none;border-bottom:1px solid var(--c-rule)}.dx-link:hover{text-decoration:underline}
.dx-title{font-family:var(--font-ui);font-weight:700;font-size:.8em;letter-spacing:.06em;color:var(--c-faint);margin-bottom:.2em}
.dx-table{display:table;border-collapse:collapse}.dx-row{display:table-row}.dx-cell{display:table-cell;border-bottom:1px solid var(--c-rule)}
`;
