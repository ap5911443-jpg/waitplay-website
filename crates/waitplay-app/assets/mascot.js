/* Vibe Play's companion: a small original character drawn as an inline SVG and animated with CSS transforms plus a few timers.
 * No images, no animation library, nothing running while it has nothing to say: the steady motions are slow transform-only CSS
 * animations (compositor friendly), the "life" (blinks, glances, naps) is a handful of timers that stop when the window is hidden.
 *
 *   const m = Mascot.mount(hostElement);
 *   m.configure({ on, intensity: 'subtle'|'playful', idle, pointer, reduce });
 *   m.set('idle'|'sleep'|'starting'|'working'|'waiting'|'completed'|'error');   // what the app is really doing
 *   m.celebrate(ms)  // a short happy reaction, then back to the base state (one of several agents finished)
 *   m.flash('error', ms) / m.look(x, y) (-1..1, pointer eyes) / m.look(null)
 *
 * It only draws the state it is given; it never detects anything. */
const Mascot = (function () {
  const CSS = `
.mc{display:block;width:100%;height:100%;overflow:visible;--gc:#6cc8ff;--a:1}
.mc *{transform-box:fill-box;transform-origin:center}
.mc .glow{opacity:0;transition:opacity .5s ease}
.mc[data-s=starting] .glow,.mc[data-s=working] .glow,.mc[data-s=waiting] .glow,.mc[data-s=completed] .glow,.mc[data-s=error] .glow{opacity:1}
.mc[data-s=working]{--gc:#3be3b4}.mc[data-s=waiting]{--gc:#ffb02e}.mc[data-s=completed]{--gc:#5dff9b}.mc[data-s=error]{--gc:#ff6159}
.mc[data-i=playful]{--a:1.7}
.mc .rig{transform-origin:50% 100%}
.mc[data-s=idle] .rig{animation:mcf 4.6s ease-in-out infinite alternate}
.mc[data-s=working] .rig{animation:mcf 2.4s ease-in-out infinite alternate;--a:.55}
.mc[data-s=waiting] .rig{animation:mcf 3.4s ease-in-out infinite alternate;--a:.6}
.mc[data-s=starting] .rig,.mc[data-s=completed] .rig{animation:mcf 2s ease-in-out infinite alternate;--a:.5}
@keyframes mcf{to{transform:translateY(calc(-1.6px * var(--a)))}}
.mc .tilt{transition:transform .45s cubic-bezier(.3,.8,.3,1)}
.mc[data-s=waiting] .tilt{transform:rotate(-9deg)}
.mc[data-s=error] .tilt{transform:rotate(4deg)}
.mc .body{transform-origin:50% 100%}
.mc[data-s=idle] .body{animation:mcb 3.4s ease-in-out infinite}
.mc[data-s=sleep] .body{animation:mcb 5.2s ease-in-out infinite;--k:1.045}
.mc[data-s=working] .body{animation:mcw 1.3s ease-in-out infinite}
@keyframes mcb{50%{transform:scale(var(--k,1.025),calc(var(--k,1.025) + .012))}}
@keyframes mcw{50%{transform:scale(1.02,.985)}}
.mc[data-s=starting] .pop{animation:mcwake .8s cubic-bezier(.3,.7,.3,1) both}
.mc[data-s=completed] .pop{animation:mcjoy 1.5s ease-in-out both}
.mc[data-s=error] .pop{animation:mcshake .5s ease-in-out both}
@keyframes mcwake{0%{transform:scale(.88)}35%{transform:translateY(calc(-4px * var(--a))) scale(.95,1.1)}65%{transform:scale(1.07,.93)}100%{transform:none}}
@keyframes mcjoy{0%{transform:none}18%{transform:translateY(calc(-5px * var(--a))) rotate(-5deg) scale(.95,1.1)}36%{transform:scale(1.08,.92)}55%{transform:translateY(calc(-4px * var(--a))) rotate(5deg) scale(.96,1.08)}75%{transform:scale(1.04,.97)}100%{transform:none}}
@keyframes mcshake{0%,100%{transform:none}20%{transform:translateX(-2px)}45%{transform:translateX(2px)}70%{transform:translateX(-1.2px)}}
/* face */
.mc .eye{transform:scaleY(var(--eo,1));transition:transform .13s ease}
.mc[data-s=working] .eye{--eo:.74}.mc[data-s=starting] .eye{--eo:1.14}.mc[data-s=waiting] .eye{--eo:1.08}.mc[data-s=error] .eye{--eo:1.1}
.mc.bl .eye{transform:scaleY(.08)}
.mc .pup{transform:translate(var(--px,0px),var(--py,0px));transition:transform .3s cubic-bezier(.3,.7,.3,1)}
.mc[data-s=error] .pup circle:first-child{r:1.9px}
.mc[data-s=starting] .pup circle:first-child{r:2.9px}
.mc .hap,.mc .slp,.mc .brow,.mc .m-o,.mc .m-u,.mc .m-w,.mc .cheeks,.mc .qm,.mc .ex,.mc .zz,.mc .kb{opacity:0;transition:opacity .2s ease}
.mc[data-s=completed] .ew,.mc[data-s=completed] .pup,.mc[data-s=sleep] .ew,.mc[data-s=sleep] .pup{opacity:0}
.mc .ew,.mc .pup{transition:opacity .15s ease}
.mc[data-s=completed] .hap,.mc[data-s=sleep] .slp,.mc[data-s=waiting] .brow,.mc[data-s=error] .brow,.mc[data-s=completed] .cheeks{opacity:1}
.mc[data-s=completed] .m-s,.mc[data-s=starting] .m-s,.mc[data-s=error] .m-s,.mc[data-s=sleep] .m-s{opacity:0}
.mc[data-s=completed] .m-o,.mc[data-s=starting] .m-u,.mc[data-s=error] .m-w{opacity:1}
.mc .brow.r{transform:rotate(0deg)}
.mc[data-s=waiting] .brow.l{transform:translateY(-1.2px) rotate(-8deg)}
.mc[data-s=waiting] .brow.r{transform:translateY(-2.4px) rotate(10deg)}
.mc[data-s=error] .brow.l{transform:rotate(-13deg)}.mc[data-s=error] .brow.r{transform:rotate(13deg)}
.mc[data-s=waiting] .qm{opacity:1;animation:mcq 1.8s ease-in-out infinite}
@keyframes mcq{50%{transform:translateY(-1.6px) rotate(6deg)}}
.mc[data-s=error] .ex{opacity:1}
.mc[data-s=sleep] .zz{opacity:.9;animation:mcz 3.4s ease-in-out infinite}
.mc[data-s=sleep] .zz2{animation-delay:1.6s}
@keyframes mcz{0%{transform:translate(0,3px) scale(.7);opacity:0}30%{opacity:.9}100%{transform:translate(3px,-7px) scale(1.1);opacity:0}}
.mc.typing .kb{opacity:.95}
.mc .kb{transform:translateY(3px);transition:opacity .25s ease,transform .25s ease}
.mc.typing .kb{transform:none}
.mc.typing .body{animation:mctype .22s ease-in-out infinite}
@keyframes mctype{50%{transform:translateY(.7px)}}
.mc .spk path{opacity:0;transform:scale(0)}
.mc[data-s=completed] .spk path{animation:mcs 1.2s ease-out both}
.mc[data-s=completed] .spk .s2{animation-delay:.25s}.mc[data-s=completed] .spk .s3{animation-delay:.5s}
@keyframes mcs{0%{opacity:0;transform:scale(0) rotate(0)}30%{opacity:1;transform:scale(1.15) rotate(20deg)}100%{opacity:0;transform:scale(.7) rotate(60deg)}}
.mc .pt{opacity:0}
.mc.puff .pt{animation:mcp 1.8s ease-out both}.mc.puff .p2{animation-delay:.35s}.mc.puff .p3{animation-delay:.7s}
@keyframes mcp{0%{opacity:0;transform:translateY(0)}25%{opacity:.8}100%{opacity:0;transform:translateY(-14px)}}
.mc[data-r="1"] *{animation:none!important;transition:none!important}
.mc[data-r="1"][data-s=completed] .spk path{opacity:1;transform:scale(.9)}
`;
  const DEFS = `<svg width="0" height="0" style="position:absolute" aria-hidden="true"><defs><radialGradient id="mcb" cx=".35" cy=".28" r=".95"><stop offset="0" stop-color="#a6f5ff"/><stop offset=".55" stop-color="#4fbaff"/><stop offset="1" stop-color="#2f6fe0"/></radialGradient></defs></svg>`;
  const eye = (x, cls) => `<g transform="translate(${x} 25)"><g class="eye ${cls}"><ellipse class="ew" rx="4.6" ry="5.4" fill="#effdff"/><g class="pup"><circle r="2.5" fill="#0a2a55"/><circle cx="-.9" cy="-1.2" r=".85" fill="#fff"/></g></g><path class="hap" d="M-4.6 1.8Q0 -5 4.6 1.8" stroke="#0a2a55" stroke-width="2.3" fill="none" stroke-linecap="round"/><path class="slp" d="M-4.4 -.8Q0 3.4 4.4 -.8" stroke="#0a2a55" stroke-width="2" fill="none" stroke-linecap="round"/><path class="brow ${cls}" d="M-4.6 -8.6L4.4 -8.6" stroke="#0a2a55" stroke-width="1.7" stroke-linecap="round"/></g>`;
  const star = (cls, x, y, k) => `<path class="${cls}" transform="translate(${x} ${y}) scale(${k})" d="M0 -5L1.3 -1.3 5 0 1.3 1.3 0 5-1.3 1.3-5 0-1.3-1.3z" fill="#e6fff4"/>`;
  const SVG = `<svg class="mc" viewBox="-8 -8 64 64" data-s="idle" aria-hidden="true">
<g class="glow"><circle cx="24" cy="26" r="29" fill="var(--gc)" opacity=".10"/><circle cx="24" cy="26" r="25" fill="var(--gc)" opacity=".14"/></g>
<g class="rig"><g class="tilt"><g class="pop"><g class="body">
<path d="M24 5C36 5 43 14 43 25.5S36 46 24 46 5 37.5 5 25.5 12 5 24 5Z" fill="url(#mcb)"/>
<path d="M24 5C36 5 43 14 43 25.5S36 46 24 46 5 37.5 5 25.5 12 5 24 5Z" fill="none" stroke="#c6fffb" stroke-opacity=".5"/>
<ellipse cx="16" cy="13.6" rx="6.5" ry="3.1" fill="#fff" opacity=".32" transform="rotate(-28 16 13.6)"/>
<path d="M24 5V.8" stroke="#8cf0ff" stroke-width="1.6" stroke-linecap="round"/><circle cx="24" cy="-.2" r="2.1" fill="#b6f8ff"/>
<g class="cheeks"><ellipse cx="10.5" cy="32.5" rx="3" ry="1.8" fill="#ff9fc4" opacity=".7"/><ellipse cx="37.5" cy="32.5" rx="3" ry="1.8" fill="#ff9fc4" opacity=".7"/></g>
${eye(17.5, "l")}${eye(30.5, "r")}
<g transform="translate(24 35.5)"><path class="m-s" d="M-3 0Q0 2.6 3 0" stroke="#0a2a55" stroke-width="1.5" fill="none" stroke-linecap="round"/><path class="m-o" d="M-3.4 -.6Q0 5.4 3.4 -.6Z" fill="#0a2a55"/><circle class="m-u" r="1.7" fill="none" stroke="#0a2a55" stroke-width="1.4"/><path class="m-w" d="M-3.2 1.4Q-1.6 -.6 0 1.2T3.2 1.2" stroke="#0a2a55" stroke-width="1.4" fill="none" stroke-linecap="round"/></g>
</g></g></g>
<g class="spk">${star("s1", 44, 9, 1)}${star("s2", 2, 15, .75)}${star("s3", 42, 40, .6)}</g>
<text class="qm" x="40" y="9" font-size="15" font-weight="800" fill="#ffb02e" font-family="system-ui,sans-serif">?</text>
<text class="ex" x="41" y="9" font-size="15" font-weight="800" fill="#ff6159" font-family="system-ui,sans-serif">!</text>
<text class="zz" x="39" y="9" font-size="9" font-weight="800" fill="#bfe9ff" font-family="system-ui,sans-serif">z</text><text class="zz zz2" x="43" y="3" font-size="6.5" font-weight="800" fill="#bfe9ff" font-family="system-ui,sans-serif">z</text>
<g class="kb" transform="translate(24 50.5)"><rect x="-12" y="-3.5" width="24" height="7" rx="2.2" fill="#16233f" stroke="#6fe3ff" stroke-width=".8"/><path d="M-8.5 -1H8.5M-8.5 1.6H8.5" stroke="#6fe3ff" stroke-width="1.1" stroke-dasharray="1.6 1.6" opacity=".8"/></g>
<circle class="pt p1" cx="6" cy="40" r="1.3" fill="#9ff6ff"/><circle class="pt p2" cx="42" cy="36" r="1.1" fill="#9ff6ff"/><circle class="pt p3" cx="24" cy="48" r="1.2" fill="#9ff6ff"/>
</g></svg>`;

  let styled = false;
  const rand = (a, b) => a + Math.random() * (b - a);

  function mount(host) {
    if (!styled) {
      styled = true;
      const st = document.createElement('style'); st.textContent = CSS; document.head.appendChild(st);
      const d = document.createElement('div'); d.innerHTML = DEFS; document.body.appendChild(d.firstChild);
    }
    host.innerHTML = SVG;
    const svg = host.firstElementChild;
    const reduceMq = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : { matches: false, addEventListener() {} };
    let cfg = { on: true, intensity: 'subtle', idle: true, pointer: true, reduce: false };
    let base = 'idle', shown = 'idle', over = 0, timers = [], pointerOn = false;
    const reduced = () => cfg.reduce || reduceMq.matches;
    const visible = () => !document.hidden && host.offsetParent !== null;
    const awake = () => shown !== 'sleep' && shown !== 'completed';

    function clear() { timers.forEach(clearTimeout); timers = []; svg.classList.remove('bl', 'typing', 'puff'); }
    function later(ms, fn) { const t = setTimeout(() => { timers = timers.filter(x => x !== t); if (visible()) fn(); }, ms); timers.push(t); }
    function look(x, y) { svg.style.setProperty('--px', (x * 1.7).toFixed(2) + 'px'); svg.style.setProperty('--py', (y * 1.5).toFixed(2) + 'px'); }

    function blink() {
      if (!awake()) return;
      svg.classList.add('bl');
      later(shown === 'idle' ? 190 : 130, () => {
        svg.classList.remove('bl');
        if (Math.random() < .18) later(150, () => { svg.classList.add('bl'); later(120, () => svg.classList.remove('bl')); });
      });
    }
    function loopBlink() { later(rand(2400, 5600), () => { blink(); loopBlink(); }); }
    function loopGlance() {
      const w = shown === 'working';
      later(w ? rand(1500, 3200) : rand(6000, 13000), () => {
        if (!pointerOn && awake()) {
          if (!svg.classList.contains('typing')) look(rand(-1, 1), w ? rand(-.2, .5) : rand(-.5, .3));
          later(w ? 900 : 1800, () => { if (!pointerOn && !svg.classList.contains('typing')) look(0, 0); });
        }
        loopGlance();
      });
    }
    function loopTyping() {
      later(rand(7000, 12000) / (cfg.intensity === 'playful' ? 1.8 : 1), () => {
        if (shown === 'working') { svg.classList.add('typing'); look(0, 1); later(2300, () => { svg.classList.remove('typing'); look(0, 0); }); }
        loopTyping();
      });
    }
    function loopPuff() {
      later(rand(6000, 11000) / (cfg.intensity === 'playful' ? 1.8 : 1), () => {
        if (shown === 'working') { svg.classList.add('puff'); later(2300, () => svg.classList.remove('puff')); }
        loopPuff();
      });
    }
    function loopNap() {
      // asleep: now and then a sleepy peek; awake and idle: eventually dozes off
      if (shown === 'idle') later(20000, () => { if (shown === 'idle') show('sleep'); });
    }
    function loopPeek() {
      later(rand(45000, 90000), () => {
        if (shown === 'sleep') { svg.dataset.s = 'idle'; later(2200, () => { if (shown === 'sleep') svg.dataset.s = 'sleep'; }); shown = 'sleep'; }
        loopPeek();
      });
    }
    function loopCute() {
      later(rand(70000, 150000), () => {
        if (shown === 'idle') { svg.dataset.s = 'completed'; later(1300, () => { if (shown === 'idle') svg.dataset.s = 'idle'; }); }
        loopCute();
      });
    }

    function arm() {
      clear();
      if (!cfg.on || reduced() || !visible()) return;
      if (shown === 'idle' && !cfg.idle) return;
      if (shown === 'sleep' && !cfg.idle) return;
      if (shown === 'idle' || shown === 'working' || shown === 'waiting' || shown === 'starting') { loopBlink(); loopGlance(); }
      if (shown === 'idle') { loopNap(); loopCute(); }
      if (shown === 'sleep') loopPeek();
      if (shown === 'working') { loopTyping(); loopPuff(); }
    }

    function show(state) {
      const same = shown === state;
      shown = state;
      if (same && state === 'completed') { svg.dataset.s = 'x'; void svg.getBoundingClientRect(); }
      svg.dataset.s = state;
      if (state !== 'working') look(0, 0);
      arm();
    }
    function apply() { svg.dataset.i = cfg.intensity; svg.dataset.r = reduced() ? '1' : '0'; }

    const api = {
      svg,
      configure(c) {
        const next = Object.assign({}, cfg, c || {});
        if (JSON.stringify(next) === JSON.stringify(cfg)) return;
        cfg = next;
        apply(); host.style.display = cfg.on ? '' : 'none';
        show(shown);
      },
      set(state) {
        if (base === state) return;
        base = state;
        if (over) return; // a reaction is playing; it hands back to this state
        show(state);
      },
      flash(state, ms) {
        over++; show(state);
        setTimeout(() => { over = Math.max(0, over - 1); if (!over) show(base); }, ms);
      },
      celebrate(ms) { api.flash('completed', ms || 1700); },
      look(x, y) {
        if (x == null) { pointerOn = false; look(0, 0); return; }
        if (!cfg.pointer || reduced() || !awake()) return;
        pointerOn = true; look(Math.max(-1, Math.min(1, x)), Math.max(-1, Math.min(1, y)));
      }
    };
    document.addEventListener('visibilitychange', arm);
    if (reduceMq.addEventListener) reduceMq.addEventListener('change', () => { apply(); arm(); });
    apply(); arm();
    return api;
  }
  return { mount };
})();
