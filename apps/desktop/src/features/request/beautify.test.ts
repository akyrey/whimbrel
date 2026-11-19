import { beautifyJson } from "./beautify";

describe("beautifyJson", () => {
  it("indents compact JSON by two spaces", () => {
    expect(beautifyJson('{"a":1,"b":[true,null]}')).toEqual({
      ok: true,
      text: '{\n  "a": 1,\n  "b": [\n    true,\n    null\n  ]\n}',
    });
  });

  it("keeps variables, inside strings and out", () => {
    const result = beautifyJson('{"id":{{id}},"auth":"Bearer {{token}}","n":[{{a}},{{b}}]}');
    expect(result).toEqual({
      ok: true,
      text: '{\n  "id": {{id}},\n  "auth": "Bearer {{token}}",\n  "n": [\n    {{a}},\n    {{b}}\n  ]\n}',
    });
  });

  it("does not take escaped quotes for the end of a string", () => {
    expect(beautifyJson('{"q":"say \\"{{x}}\\""}')).toEqual({
      ok: true,
      text: '{\n  "q": "say \\"{{x}}\\""\n}',
    });
  });

  it("leaves an empty body alone and reports invalid JSON", () => {
    expect(beautifyJson("  ")).toEqual({ ok: true, text: "  " });
    expect(beautifyJson('{"a":')).toMatchObject({ ok: false });
  });
});
