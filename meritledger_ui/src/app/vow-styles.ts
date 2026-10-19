/** Styles shared by the vow and precept pages (lazy, so not in the first load). */
export const VOW_STYLES = `
.state { display: inline-block; padding: 0.1rem 0.6rem; border-radius: 1rem; border: 1px solid var(--line); font-size: 0.9rem; }
.state.active { border-color: var(--ok); color: var(--ok); }
.state.breached { border-color: var(--accent); color: var(--accent); }
.state.completed { background: var(--ok); color: var(--card); border-color: var(--ok); }
.state.retired, .state.paused { color: var(--muted); }
.vows { list-style: none; padding: 0; margin: 0 0 0.75rem; }
.vows li { display: flex; flex-direction: column; padding: 0.5rem 0; border-bottom: 1px solid var(--line); }
.vows li.past a { color: var(--muted); }
.timeline { list-style: none; padding: 0; margin: 0; }
.timeline li { display: flex; flex-wrap: wrap; gap: 0.25rem 0.75rem; padding: 0.4rem 0; border-bottom: 1px solid var(--line); }
.timeline .when { min-width: 11rem; }
.timeline .note { flex-basis: 100%; }
.timeline li.breach span:nth-child(2) { color: var(--accent); }
.post h2, .post h3 { margin-bottom: 0.25rem; }
.post + .post { border-top: 1px solid var(--line); padding-top: 0.75rem; margin-top: 0.75rem; }
.post-body { overflow-wrap: anywhere; }
textarea { width: 100%; font: inherit; padding: 0.5rem; border: 1px solid var(--line); border-radius: 0.5rem; background: var(--bg); color: var(--text); }
.precepts { list-style: none; padding: 0; margin: 0; }
.precepts li { padding: 0.75rem 0; border-bottom: 1px solid var(--line); scroll-margin-top: 1rem; }
.precepts li.taken { border-left: 3px solid var(--ok); padding-left: 0.6rem; }
.precepts li:target { background: color-mix(in srgb, var(--accent) 10%, transparent); }
.precepts .head { display: flex; flex-wrap: wrap; gap: 0.25rem 0.75rem; align-items: baseline; }
.precepts p { margin: 0.3rem 0; }
.precepts a { margin-right: 0.5rem; overflow-wrap: anywhere; }
.zh { font-size: 0.95rem; color: var(--muted); }
.precept-record + .precept-record { border-top: 1px solid var(--line); padding-top: 0.5rem; }
.precept-record p { margin: 0.25rem 0; }
.taken { margin: 0.25rem 0 0.5rem; padding-left: 1.25rem; font-size: 0.9rem; }
`;
