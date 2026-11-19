/** Request variables as an ordered list of rows; pure helpers that return new maps. */
export type Variables = Readonly<Record<string, string>>;

export function renamed(variables: Variables, from: string, to: string): Variables {
  if (from === to || to in variables) return variables;
  return Object.fromEntries(Object.entries(variables).map(([k, v]) => [k === from ? to : k, v]));
}

export function withValue(variables: Variables, name: string, value: string): Variables {
  return { ...variables, [name]: value };
}

export function without(variables: Variables, name: string): Variables {
  return Object.fromEntries(Object.entries(variables).filter(([k]) => k !== name));
}

/** A name not yet used: `variable`, `variable2`, … */
export function freshName(variables: Variables): string {
  let n = 1;
  const name = () => (n === 1 ? "variable" : `variable${String(n)}`);
  while (name() in variables) n += 1;
  return name();
}
