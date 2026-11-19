/** Pure helpers for the spec browser: the tag tree and short labels for schemas. */
import type { Api, Operation, Tag } from "../../ipc";

export type TagNode = {
  name: string;
  label: string;
  children: TagNode[];
  operations: Operation[];
};

/** Tag kinds that label operations rather than group them (OAS 3.2). */
const NON_GROUPING = new Set(["badge", "audience"]);
export const UNTAGGED = "Other";

function grouping(tags: Tag[], name: string): boolean {
  const kind = tags.find((t) => t.name === name)?.kind;
  return kind == null || !NON_GROUPING.has(kind);
}

const matches = (op: Operation, filter: string): boolean => {
  if (filter === "") return true;
  const text = `${op.method} ${op.path} ${op.summary ?? ""} ${op.id}`.toLowerCase();
  return filter
    .toLowerCase()
    .split(/\s+/)
    .every((word) => text.includes(word));
};

/** Tags nested by `parent`, each with its operations (first grouping tag); untagged ones last.
 * Empty branches are left out when `filter` hides all their operations. */
export function tagTree(api: Api, filter = ""): TagNode[] {
  const byTag = new Map<string, Operation[]>();
  for (const op of api.operations.filter((o) => matches(o, filter))) {
    const tag = op.tags.find((t) => grouping(api.tags, t)) ?? UNTAGGED;
    byTag.set(tag, [...(byTag.get(tag) ?? []), op]);
  }
  const declared = api.tags.filter((t) => grouping(api.tags, t.name));
  const names = [
    ...declared.map((t) => t.name),
    ...[...byTag.keys()].filter((n) => n !== UNTAGGED && !declared.some((t) => t.name === n)),
  ];
  const parentOf = (name: string) => api.tags.find((t) => t.name === name)?.parent ?? null;
  const build = (name: string, seen: Set<string>): TagNode => {
    const tag = api.tags.find((t) => t.name === name);
    const next = new Set([...seen, name]);
    return {
      name,
      label: tag?.summary ?? name,
      operations: byTag.get(name) ?? [],
      children: names
        .filter((n) => parentOf(n) === name && !next.has(n))
        .map((n) => build(n, next)),
    };
  };
  const roots = names
    .filter((n) => {
      const parent = parentOf(n);
      return parent === null || !names.includes(parent);
    })
    .map((n) => build(n, new Set()));
  const untagged = byTag.get(UNTAGGED);
  const all = untagged
    ? [...roots, { name: UNTAGGED, label: UNTAGGED, children: [], operations: untagged }]
    : roots;
  return prune(all);
}

const size = (node: TagNode): number =>
  node.operations.length + node.children.reduce((n, c) => n + size(c), 0);

function prune(nodes: TagNode[]): TagNode[] {
  return nodes.filter((n) => size(n) > 0).map((n) => ({ ...n, children: prune(n.children) }));
}

const SCHEMA_REF = "#/schemas/";

/** The schema-table name a `$ref` points at, if the schema is a link. */
export function refName(schema: unknown): string | null {
  if (typeof schema !== "object" || schema === null) return null;
  const ref = (schema as Record<string, unknown>)["$ref"];
  return typeof ref === "string" && ref.startsWith(SCHEMA_REF)
    ? ref.slice(SCHEMA_REF.length)
    : null;
}

const asRecord = (value: unknown): Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};

/** A short type label: `string (uuid)`, `array of Pet`, `string | null`, `oneOf (2)`. */
export function typeLabel(schema: unknown): string {
  const ref = refName(schema);
  if (ref !== null) return ref;
  const s = asRecord(schema);
  if (schema === true) return "any";
  for (const key of ["oneOf", "anyOf", "allOf"]) {
    const parts = s[key];
    if (Array.isArray(parts)) return `${key} (${String(parts.length)})`;
  }
  const type = Array.isArray(s.type)
    ? s.type.join(" | ")
    : typeof s.type === "string"
      ? s.type
      : "";
  if (type === "array" || (type === "" && "items" in s)) {
    return `array of ${typeLabel(s.items ?? true)}`;
  }
  if ("const" in s) return `const ${JSON.stringify(s.const)}`;
  const format = typeof s.format === "string" ? ` (${s.format})` : "";
  const base = type || ("properties" in s ? "object" : "any");
  return `${base}${format}`;
}

/** Child rows of a schema in the tree: properties, items, combinations. */
export function schemaChildren(
  schema: unknown,
): { name: string; schema: unknown; required: boolean }[] {
  const s = asRecord(schema);
  const required = Array.isArray(s.required) ? (s.required as unknown[]) : [];
  const props = Object.entries(asRecord(s.properties)).map(([name, sub]) => ({
    name,
    schema: sub,
    required: required.includes(name),
  }));
  const items =
    "items" in s && typeof s.items === "object"
      ? [{ name: "[items]", schema: s.items, required: false }]
      : [];
  const prefix = Array.isArray(s.prefixItems)
    ? (s.prefixItems as unknown[]).map((p, i) => ({
        name: `[${String(i)}]`,
        schema: p,
        required: true,
      }))
    : [];
  const combos = ["allOf", "oneOf", "anyOf"].flatMap((key) =>
    Array.isArray(s[key])
      ? (s[key] as unknown[]).map((p, i) => ({
          name: `${key}[${String(i)}]`,
          schema: p,
          required: false,
        }))
      : [],
  );
  const extra =
    typeof s.additionalProperties === "object" && s.additionalProperties !== null
      ? [{ name: "{key}", schema: s.additionalProperties, required: false }]
      : [];
  return [...props, ...prefix, ...items, ...combos, ...extra];
}

/** Notes worth showing beside a type: enum values, default, constraints. */
export function schemaNotes(schema: unknown): string[] {
  const s = asRecord(schema);
  const notes: string[] = [];
  if (Array.isArray(s.enum))
    notes.push(`one of ${s.enum.map((v) => JSON.stringify(v)).join(", ")}`);
  if ("default" in s) notes.push(`default ${JSON.stringify(s.default)}`);
  for (const key of [
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "pattern",
    "minItems",
    "maxItems",
  ]) {
    if (key in s) notes.push(`${key} ${JSON.stringify(s[key])}`);
  }
  if (s.readOnly === true) notes.push("read-only");
  if (s.writeOnly === true) notes.push("write-only");
  if (s.deprecated === true) notes.push("deprecated");
  return notes;
}

export const descriptionOf = (schema: unknown): string | null => {
  const d = asRecord(schema).description;
  return typeof d === "string" ? d : null;
};
