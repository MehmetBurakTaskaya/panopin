/* Cork board UI sounds, synthesized at runtime with Web Audio (no audio files).
   Character follows docs/concept/cork-reference.html; every sound is <= 400 ms.
   The real app mutes these when a full-screen app is active or Do Not Disturb is on. */
(function () {
  'use strict';

  let ctx = null, master = null, noiseBuf = null;
  let enabled = true, volume = 0.3;

  // Rate limiting: a minimum gap per sound plus a global cap on simultaneous voices,
  // so rapid key repeats never pile up into noise.
  const MIN_GAP = { copy: 70, pin: 120, unpin: 120, drop: 140, back: 120, open: 200, close: 160 };
  const GAIN = { copy: .7, pin: 1, unpin: 1, drop: .8, back: .8, open: .6, close: .45 };
  const MAX_VOICES = 3, VOICE_WINDOW_MS = 160;
  const lastPlayed = Object.create(null);
  let recent = [];

  function audio() {
    if (!ctx) {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return null;
      ctx = new AC();
      const comp = ctx.createDynamicsCompressor();
      comp.threshold.value = -18; comp.knee.value = 12; comp.ratio.value = 4;
      comp.attack.value = .003; comp.release.value = .12;
      master = ctx.createGain();
      master.gain.value = volume;
      master.connect(comp).connect(ctx.destination);
      noiseBuf = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
      const d = noiseBuf.getChannelData(0);
      for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    }
    if (ctx.state === 'suspended') ctx.resume();
    return ctx;
  }

  // Small random variation keeps repeated sounds from feeling mechanical.
  const vary = (x, amount = .04) => x * (1 + (Math.random() * 2 - 1) * amount);

  function burst(a, out, { t = 0, dur = .04, type = 'bandpass', f = 1800, f2 = 0, q = 3, g = .9 }) {
    const src = a.createBufferSource(), filter = a.createBiquadFilter(), env = a.createGain();
    const T = a.currentTime + t;
    src.buffer = noiseBuf;
    filter.type = type;
    filter.Q.value = q;
    filter.frequency.setValueAtTime(vary(f), T);
    if (f2) filter.frequency.exponentialRampToValueAtTime(vary(f2), T + dur);
    env.gain.setValueAtTime(.0001, T);
    env.gain.exponentialRampToValueAtTime(g, T + Math.min(.004, dur / 4));
    env.gain.exponentialRampToValueAtTime(.0001, T + dur);
    src.connect(filter).connect(env).connect(out);
    src.start(T, Math.random() * .6, dur + .02);
  }

  function tone(a, out, { t = 0, f = 200, f2 = 0, dur = .08, g = .5, type = 'sine' }) {
    const osc = a.createOscillator(), env = a.createGain();
    const T = a.currentTime + t;
    osc.type = type;
    osc.frequency.setValueAtTime(vary(f, .03), T);
    if (f2) osc.frequency.exponentialRampToValueAtTime(vary(f2, .03), T + dur);
    env.gain.setValueAtTime(.0001, T);
    env.gain.exponentialRampToValueAtTime(g, T + .005);
    env.gain.exponentialRampToValueAtTime(.0001, T + dur);
    osc.connect(env).connect(out);
    osc.start(T);
    osc.stop(T + dur + .02);
  }

  function crinkle(a, out, { t = 0, n = 6, span = .12, f = 3200, g = .4 }) {
    for (let i = 0; i < n; i++) {
      burst(a, out, {
        t: t + Math.random() * span, dur: .007 + Math.random() * .012,
        f: f * (.7 + Math.random() * .6), q: 2, g: g * (.4 + Math.random() * .6),
      });
    }
  }

  const SOUNDS = {
    // soft paper touch
    copy(a, o) { crinkle(a, o, { n: 4, span: .05, f: 3600, g: .32 }); tone(a, o, { f: 230, f2: 170, dur: .05, g: .16 }); },
    // needle prick, then the muffled "tok" of the pin entering cork; t = impact time
    pin(a, o, t) {
      burst(a, o, { t, dur: .012, type: 'highpass', f: 3000, q: .7, g: .45 });
      burst(a, o, { t: t + .01, dur: .07, type: 'lowpass', f: 420, q: 1, g: .85 });
      tone(a, o, { t: t + .01, f: 150, f2: 85, dur: .09, g: .5 });
    },
    // small pop, then tape rustle
    unpin(a, o) { tone(a, o, { f: 650, f2: 1150, dur: .05, g: .22, type: 'triangle' }); crinkle(a, o, { t: .07, n: 6, span: .1, f: 2800, g: .38 }); },
    // tape tearing off, paper rustle, short glide
    drop(a, o) {
      burst(a, o, { dur: .16, f: 1400, f2: 4200, q: 1.4, g: .3 });
      crinkle(a, o, { t: .02, n: 8, span: .14, f: 3000, g: .42 });
      burst(a, o, { t: .12, dur: .26, type: 'lowpass', f: 1200, f2: 300, q: .6, g: .2 });
    },
    // tape sticking back down
    back(a, o) { crinkle(a, o, { n: 8, span: .11, f: 2600, g: .42 }); tone(a, o, { t: .1, f: 200, f2: 140, dur: .05, g: .22 }); },
    // board sliding, then a light knock of the wooden frame
    open(a, o) {
      burst(a, o, { dur: .32, type: 'lowpass', f: 700, f2: 250, q: .7, g: .28 });
      burst(a, o, { t: .3, dur: .08, type: 'lowpass', f: 300, q: 1, g: .75 });
      tone(a, o, { t: .3, f: 110, f2: 70, dur: .1, g: .45 });
    },
    // short, soft slide
    close(a, o) { burst(a, o, { dur: .18, type: 'lowpass', f: 900, f2: 400, q: .6, g: .28 }); },
  };

  function play(name, { delay = 0 } = {}) {
    if (!enabled || volume <= 0 || !SOUNDS[name]) return;
    // browsers only allow audio after a user gesture
    if (navigator.userActivation && !navigator.userActivation.hasBeenActive) return;
    const now = performance.now();
    if (now - (lastPlayed[name] || 0) < MIN_GAP[name]) return;
    recent = recent.filter((ts) => now - ts < VOICE_WINDOW_MS);
    if (recent.length >= MAX_VOICES) return;
    const a = audio();
    if (!a) return;
    lastPlayed[name] = now;
    recent.push(now);
    const out = a.createGain();
    out.gain.value = GAIN[name];
    out.connect(master);
    SOUNDS[name](a, out, delay);
    setTimeout(() => out.disconnect(), (delay + 1) * 1000);
  }

  window.CorkSound = {
    play,
    unlock() { audio(); },
    setEnabled(v) { enabled = !!v; },
    setVolume(v) {
      volume = Math.max(0, Math.min(1, Number(v)));
      if (master) master.gain.setTargetAtTime(volume, ctx.currentTime, .02);
    },
  };
})();
