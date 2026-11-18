/** Small per-user conveniences in localStorage (open collections, open tabs). Never the only
 * copy of anything: unreadable or missing data gives the fallback. */
export function loadJson<T>(key: string, fallback: T): T {
  try {
    const text = localStorage.getItem(key);
    return text === null ? fallback : (JSON.parse(text) as T);
  } catch {
    return fallback;
  }
}

export function saveJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage full or blocked: the convenience is lost, nothing else.
  }
}
