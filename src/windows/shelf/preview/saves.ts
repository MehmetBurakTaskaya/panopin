// Edits are saved in the background; a copy, paste or drag waits for them, so it always carries
// the latest crop, marks or text.
const pending = new Set<Promise<unknown>>();

export function trackSave<T>(save: Promise<T>): Promise<T> {
  pending.add(save);
  const done = () => pending.delete(save);
  save.then(done, done);
  return save;
}

export async function whenSaved(): Promise<void> {
  while (pending.size > 0) await Promise.allSettled([...pending]);
}
