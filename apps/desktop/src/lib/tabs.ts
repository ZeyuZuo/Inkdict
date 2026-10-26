// Tab groups in entry HTML (spec/v1.md §4.4; markup in apps/desktop/README.md):
// <div class="dx-tabs"><div class="dx-tab-bar"><button class="dx-tab">… then
// one .dx-tab-panel per tab. The pack theme decides where they are.

function parts(group: Element) {
  const bar = group.querySelector(':scope > .dx-tab-bar');
  const tabs = bar ? [...bar.querySelectorAll<HTMLElement>(':scope > .dx-tab')] : [];
  const panels = [...group.querySelectorAll<HTMLElement>(':scope > .dx-tab-panel')];
  return { tabs, panels };
}

export function select(group: Element, index: number): void {
  const { tabs, panels } = parts(group);
  if (index < 0 || index >= panels.length) return;
  tabs.forEach((tab, i) => {
    tab.setAttribute('aria-selected', String(i === index));
    tab.tabIndex = i === index ? 0 : -1;
  });
  panels.forEach((panel, i) => { panel.hidden = i !== index; });
}

export function selected(group: Element): number {
  return Math.max(0, parts(group).panels.findIndex(panel => !panel.hidden));
}

/** Open every tab that contains `element`, outermost first. */
export function reveal(element: Element, root: Element): void {
  const chain: [Element, number][] = [];
  for (let node: Element | null = element; node && node !== root; node = node.parentElement) {
    const group = node.parentElement;
    if (node.classList.contains('dx-tab-panel') && group?.classList.contains('dx-tabs')) {
      chain.unshift([group, parts(group).panels.indexOf(node as HTMLElement)]);
    }
    if (node instanceof HTMLDetailsElement) node.open = true;
  }
  for (const [group, index] of chain) select(group, index);
}

/** The open tab of each group, in document order. */
export function state(root: Element): number[] {
  return [...root.querySelectorAll('.dx-tabs')].map(selected);
}
export function restore(root: Element, tabs: number[]): void {
  [...root.querySelectorAll('.dx-tabs')].forEach((group, i) => { if (tabs[i] !== undefined) select(group, tabs[i]); });
}

/** Click on a tab: open it. Returns whether the event was a tab click. */
export function clicked(target: Element): boolean {
  const tab = target.closest<HTMLElement>('.dx-tab');
  const group = tab?.closest('.dx-tabs');
  if (!tab || !group) return false;
  select(group, parts(group).tabs.indexOf(tab));
  return true;
}

/** ←/→, Home, End on a focused tab move between tabs. */
export function keyed(event: KeyboardEvent): boolean {
  const tab = (event.target as Element | null)?.closest?.<HTMLElement>('.dx-tab');
  const group = tab?.closest('.dx-tabs');
  if (!tab || !group) return false;
  const { tabs } = parts(group);
  const current = tabs.indexOf(tab);
  const next = { ArrowLeft: current - 1, ArrowRight: current + 1, Home: 0, End: tabs.length - 1 }[event.key];
  if (next === undefined || next < 0 || next >= tabs.length) return false;
  select(group, next);
  tabs[next].focus();
  return true;
}
