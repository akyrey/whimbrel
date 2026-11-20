/** The light theme mirrors only some Tailwind scales (@roost/ui/theme-light.css, and violet in
 * index.css); a class from any other scale keeps its dark colour in light mode. */
const MIRRORED = new Set(["zinc", "sky", "rose", "amber", "emerald", "violet"]);
const COLOUR_CLASS =
  /\b(?:bg|text|border|ring|outline|fill|stroke|from|to|via|divide|placeholder|accent|caret|decoration)-([a-z]+)-(?:50|[1-9]00|950)\b/g;

const sources = import.meta.glob<string>(["./**/*.{ts,tsx,css}", "!./**/*.test.{ts,tsx}"], {
  query: "?raw",
  import: "default",
  eager: true,
});

it("uses only colour scales the light theme mirrors", () => {
  const offending = Object.entries(sources).flatMap(([file, text]) =>
    [...text.matchAll(COLOUR_CLASS)]
      .filter((match) => !MIRRORED.has(match[1] ?? ""))
      .map((match) => `${file}: ${match[0]}`),
  );
  expect(offending).toEqual([]);
});
