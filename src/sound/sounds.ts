// Cork board sounds, synthesised at runtime with Web Audio (D-026): no audio files. Ported from
// the approved concept (docs/concept/mockup/sound.js); every sound is at most 400 ms.

export type SoundName = 'copy' | 'pin' | 'unpin' | 'drop' | 'back' | 'open' | 'close';

type Out = { ctx: AudioContext; out: AudioNode; noise: AudioBuffer };

// Rate limiting: a minimum gap per sound and at most three voices per 160 ms, so key repeats
// never pile up into noise.
const MIN_GAP: Record<SoundName, number> = { copy: 70, pin: 120, unpin: 120, drop: 140, back: 120, open: 200, close: 160 };
const GAIN: Record<SoundName, number> = { copy: 0.7, pin: 1, unpin: 1, drop: 0.8, back: 0.8, open: 0.6, close: 0.45 };
const MAX_VOICES = 3;
const VOICE_WINDOW_MS = 160;

let ctx: AudioContext | null = null;
let master: GainNode | null = null;
let noise: AudioBuffer | null = null;
let enabled = true;
let muted = false;
let volume = 0.3;
const lastPlayed: Partial<Record<SoundName, number>> = {};
let recent: number[] = [];

function audio(): Out | null {
  if (!ctx) {
    if (typeof AudioContext === 'undefined') return null;
    ctx = new AudioContext();
    const comp = ctx.createDynamicsCompressor();
    comp.threshold.value = -18;
    comp.knee.value = 12;
    comp.ratio.value = 4;
    comp.attack.value = 0.003;
    comp.release.value = 0.12;
    master = ctx.createGain();
    master.gain.value = volume;
    master.connect(comp).connect(ctx.destination);
    noise = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
    const data = noise.getChannelData(0);
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
  }
  if (ctx.state === 'suspended') void ctx.resume();
  return master && noise ? { ctx, out: master, noise } : null;
}

// A little random variation keeps repeated sounds from feeling mechanical.
const vary = (x: number, amount = 0.04) => x * (1 + (Math.random() * 2 - 1) * amount);

interface BurstOptions { t?: number; dur?: number; type?: BiquadFilterType; f?: number; f2?: number; q?: number; g?: number }

function burst(a: Out, { t = 0, dur = 0.04, type = 'bandpass', f = 1800, f2 = 0, q = 3, g = 0.9 }: BurstOptions) {
  const src = a.ctx.createBufferSource();
  const filter = a.ctx.createBiquadFilter();
  const env = a.ctx.createGain();
  const at = a.ctx.currentTime + t;
  src.buffer = a.noise;
  filter.type = type;
  filter.Q.value = q;
  filter.frequency.setValueAtTime(vary(f), at);
  if (f2) filter.frequency.exponentialRampToValueAtTime(vary(f2), at + dur);
  env.gain.setValueAtTime(0.0001, at);
  env.gain.exponentialRampToValueAtTime(g, at + Math.min(0.004, dur / 4));
  env.gain.exponentialRampToValueAtTime(0.0001, at + dur);
  src.connect(filter).connect(env).connect(a.out);
  src.start(at, Math.random() * 0.6, dur + 0.02);
}

interface ToneOptions { t?: number; f?: number; f2?: number; dur?: number; g?: number; type?: OscillatorType }

function tone(a: Out, { t = 0, f = 200, f2 = 0, dur = 0.08, g = 0.5, type = 'sine' }: ToneOptions) {
  const osc = a.ctx.createOscillator();
  const env = a.ctx.createGain();
  const at = a.ctx.currentTime + t;
  osc.type = type;
  osc.frequency.setValueAtTime(vary(f, 0.03), at);
  if (f2) osc.frequency.exponentialRampToValueAtTime(vary(f2, 0.03), at + dur);
  env.gain.setValueAtTime(0.0001, at);
  env.gain.exponentialRampToValueAtTime(g, at + 0.005);
  env.gain.exponentialRampToValueAtTime(0.0001, at + dur);
  osc.connect(env).connect(a.out);
  osc.start(at);
  osc.stop(at + dur + 0.02);
}

function crinkle(a: Out, { t = 0, n = 6, span = 0.12, f = 3200, g = 0.4 }) {
  for (let i = 0; i < n; i++) {
    burst(a, {
      t: t + Math.random() * span,
      dur: 0.007 + Math.random() * 0.012,
      f: f * (0.7 + Math.random() * 0.6),
      q: 2,
      g: g * (0.4 + Math.random() * 0.6),
    });
  }
}

const SOUNDS: Record<SoundName, (a: Out, delay: number) => void> = {
  // A soft paper touch.
  copy: (a) => {
    crinkle(a, { n: 4, span: 0.05, f: 3600, g: 0.32 });
    tone(a, { f: 230, f2: 170, dur: 0.05, g: 0.16 });
  },
  // The needle pricks, then the muffled knock of the pin entering cork; `delay` is the impact.
  pin: (a, t) => {
    burst(a, { t, dur: 0.012, type: 'highpass', f: 3000, q: 0.7, g: 0.45 });
    burst(a, { t: t + 0.01, dur: 0.07, type: 'lowpass', f: 420, q: 1, g: 0.85 });
    tone(a, { t: t + 0.01, f: 150, f2: 85, dur: 0.09, g: 0.5 });
  },
  // A small pop, then the tape rustles back.
  unpin: (a) => {
    tone(a, { f: 650, f2: 1150, dur: 0.05, g: 0.22, type: 'triangle' });
    crinkle(a, { t: 0.07, n: 6, span: 0.1, f: 2800, g: 0.38 });
  },
  // Tape tears off, the paper rustles and glides down.
  drop: (a) => {
    burst(a, { dur: 0.16, f: 1400, f2: 4200, q: 1.4, g: 0.3 });
    crinkle(a, { t: 0.02, n: 8, span: 0.14, f: 3000, g: 0.42 });
    burst(a, { t: 0.12, dur: 0.26, type: 'lowpass', f: 1200, f2: 300, q: 0.6, g: 0.2 });
  },
  // Tape sticks back down.
  back: (a) => {
    crinkle(a, { n: 8, span: 0.11, f: 2600, g: 0.42 });
    tone(a, { t: 0.1, f: 200, f2: 140, dur: 0.05, g: 0.22 });
  },
  // The board slides, then the wooden frame knocks lightly.
  open: (a) => {
    burst(a, { dur: 0.32, type: 'lowpass', f: 700, f2: 250, q: 0.7, g: 0.28 });
    burst(a, { t: 0.3, dur: 0.08, type: 'lowpass', f: 300, q: 1, g: 0.75 });
    tone(a, { t: 0.3, f: 110, f2: 70, dur: 0.1, g: 0.45 });
  },
  // A short, soft slide.
  close: (a) => burst(a, { dur: 0.18, type: 'lowpass', f: 900, f2: 400, q: 0.6, g: 0.28 }),
};

/** Plays a sound unless sounds are off, muted for this opening, or rate-limited. */
export function play(name: SoundName, delay = 0): void {
  if (!enabled || muted || volume <= 0) return;
  const now = performance.now();
  if (now - (lastPlayed[name] ?? -Infinity) < MIN_GAP[name]) return;
  recent = recent.filter((at) => now - at < VOICE_WINDOW_MS);
  if (recent.length >= MAX_VOICES) return;
  const a = audio();
  if (!a) return;
  lastPlayed[name] = now;
  recent.push(now);
  const out = a.ctx.createGain();
  out.gain.value = GAIN[name];
  out.connect(a.out);
  SOUNDS[name]({ ...a, out }, delay);
  window.setTimeout(() => out.disconnect(), (delay + 1) * 1000);
}

/** Settings → General: sounds on or off, level 0–100. */
export function configure(on: boolean, level: number): void {
  enabled = on;
  volume = Math.max(0, Math.min(1, level / 100));
  if (ctx && master) master.gain.setTargetAtTime(volume, ctx.currentTime, 0.02);
}

/** Silences the shelf while a full-screen app, presentation or focus session is active (D-021). */
export function setMuted(value: boolean): void {
  muted = value;
}
