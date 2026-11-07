import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// jsdom has no Web Audio: this stand-in records the contexts made and whether they were told to
// rest or to wake. Like the real one, suspend() and resume() take effect a moment later.
class FakeContext {
  static made: FakeContext[] = [];
  state: 'running' | 'suspended' = 'running';
  currentTime = 0;
  sampleRate = 48_000;
  destination = node({});
  suspend = vi.fn(() =>
    Promise.resolve().then(() => {
      this.state = 'suspended';
    }),
  );
  resume = vi.fn(() =>
    Promise.resolve().then(() => {
      this.state = 'running';
    }),
  );
  constructor() {
    FakeContext.made.push(this);
  }
  createDynamicsCompressor = () => node({ threshold: param(), knee: param(), ratio: param(), attack: param(), release: param() });
  createGain = () => node({ gain: param() });
  createBuffer = (...args: number[]) => ({ getChannelData: () => new Float32Array(args[1] ?? 0) });
  createBufferSource = () => node({ buffer: null, start: vi.fn() });
  createBiquadFilter = () => node({ type: 'bandpass', Q: param(), frequency: param() });
  createOscillator = () => node({ type: 'sine', frequency: param(), start: vi.fn(), stop: vi.fn() });
}

function param() {
  return { value: 0, setValueAtTime: vi.fn(), exponentialRampToValueAtTime: vi.fn(), setTargetAtTime: vi.fn() };
}

function node<T extends object>(extra: T) {
  return { ...extra, connect: <N>(next: N) => next, disconnect: vi.fn() };
}

/** The module keeps its context between calls, so every test starts from a fresh copy. */
async function freshSounds() {
  vi.resetModules();
  return import('./sounds');
}

beforeEach(() => {
  vi.useFakeTimers();
  FakeContext.made = [];
  vi.stubGlobal('AudioContext', FakeContext);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('sounds', () => {
  it('makes the audio context while the shelf waits and lets it rest at once', async () => {
    const sounds = await freshSounds();
    sounds.configure(true, 30);
    sounds.prepareLater();
    await vi.advanceTimersByTimeAsync(1000);
    expect(FakeContext.made).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(600);
    expect(FakeContext.made).toHaveLength(1);
    expect(FakeContext.made[0]?.suspend).toHaveBeenCalledTimes(1);
    expect(FakeContext.made[0]?.state).toBe('suspended');
  });

  it('wakes the resting context for the first sound', async () => {
    const sounds = await freshSounds();
    sounds.configure(true, 30);
    sounds.prepareLater();
    await vi.advanceTimersByTimeAsync(1600);
    sounds.play('open');
    await vi.advanceTimersByTimeAsync(0);
    expect(FakeContext.made).toHaveLength(1);
    expect(FakeContext.made[0]?.resume).toHaveBeenCalledTimes(1);
    expect(FakeContext.made[0]?.state).toBe('running');
  });

  it('wakes the context for a sound that comes while it is still going to rest', async () => {
    const sounds = await freshSounds();
    sounds.configure(true, 30);
    sounds.prepareLater();
    // Timers only: the rest has been asked for, but it has not taken effect yet.
    vi.advanceTimersByTime(1600);
    expect(FakeContext.made[0]?.state).toBe('running');
    sounds.play('open');
    await vi.advanceTimersByTimeAsync(0);
    expect(FakeContext.made[0]?.state).toBe('running');
  });

  it('makes no context while sounds are off or when the preparation is cancelled', async () => {
    const sounds = await freshSounds();
    sounds.configure(false, 30);
    sounds.prepareLater();
    await vi.advanceTimersByTimeAsync(2000);
    sounds.configure(true, 30);
    sounds.prepareLater()();
    await vi.advanceTimersByTimeAsync(2000);
    expect(FakeContext.made).toHaveLength(0);
  });
});
