/**
 * The OpenAPI commands of the mocked backend (see mockBackend.ts), following
 * src-tauri/src/spec.rs. Any source reads as a small "Pets" spec; a URL containing `remote`
 * refers to a document on another server; one containing `broken` is not OpenAPI.
 */
import type {
  Api,
  AppError,
  SpecImportArgs,
  SpecImported,
  SpecReadArgs,
  SpecSummary,
  StoredSpec,
} from "../ipc/generated";
import { mockInstall, mockStored } from "./mockBackend";

const REMOTE = "http://other.test/schemas.yaml";

export const PETS_API: Api = {
  version: "open_api31",
  versionText: "3.1.0",
  title: "Pets",
  apiVersion: "1.0",
  description: "A small pet store.",
  servers: [{ url: "http://localhost", description: "Local", variables: [] }],
  tags: [{ name: "pets", summary: "Pets", description: null, parent: null, kind: null }],
  operations: [
    {
      id: "listPets",
      hasOperationId: true,
      method: "GET",
      path: "/pets",
      summary: "List pets",
      description: null,
      tags: ["pets"],
      deprecated: false,
      parameters: [
        {
          name: "limit",
          location: "query",
          required: false,
          deprecated: false,
          description: "How many",
          schema: { type: "integer", default: 20 },
          example: null,
          examples: [],
          style: null,
          explode: null,
        },
      ],
      requestBody: null,
      responses: [
        {
          status: "200",
          summary: "The pets",
          description: "The pets",
          content: [
            {
              mediaType: "application/json",
              schema: { type: "array", items: { $ref: "#/schemas/Pet" } },
              itemSchema: null,
              example: [{ name: "Rex" }],
              examples: [],
            },
          ],
        },
      ],
      security: null,
    },
  ],
  webhooks: [
    {
      id: "petAdded",
      hasOperationId: false,
      method: "POST",
      path: "petAdded",
      summary: "A pet was added",
      description: null,
      tags: [],
      deprecated: false,
      parameters: [],
      requestBody: {
        required: true,
        description: null,
        content: [
          {
            mediaType: "application/json",
            schema: { $ref: "#/schemas/Pet" },
            itemSchema: null,
            example: null,
            examples: [
              {
                name: "rex",
                summary: "Rex",
                value: { name: "Rex" },
                serialized: null,
                external: null,
              },
            ],
          },
          {
            mediaType: "text/event-stream",
            schema: null,
            itemSchema: { type: "object", properties: { data: { type: "string" } } },
            example: null,
            examples: [],
          },
        ],
      },
      responses: [{ status: "200", summary: null, description: null, content: [] }],
      security: null,
    },
  ],
  securitySchemes: [
    {
      name: "key",
      description: null,
      deprecated: true,
      kind: { type: "api_key", name: "X-Key", location: "header" },
    },
  ],
  security: [],
  schemas: {
    Pet: {
      type: "object",
      required: ["name"],
      description: "A pet",
      properties: {
        name: { type: "string", minLength: 1 },
        kind: { type: "string", enum: ["cat", "dog"] },
        owner: { $ref: "#/schemas/Pet" },
        tags: { type: "array", items: { type: "string" } },
      },
    },
  },
  warnings: ["A reference leads nowhere: #/nope"],
};

let specs = new Map<string, StoredSpec[]>();

export function resetSpecs(): void {
  specs = new Map();
}

const reject = (code: string, message: string): Promise<never> =>
  Promise.reject({ code, message } satisfies AppError);

function summary(args: SpecReadArgs): Promise<SpecSummary> {
  const location = args.source.kind === "url" ? args.source.url : args.source.path;
  if (location.includes("broken")) {
    return reject("openapi_unknown", "This is not an OpenAPI document: it has neither field");
  }
  return Promise.resolve({
    title: "Pets",
    apiVersion: "1.0",
    versionText: "3.1.0",
    operations: 1,
    webhooks: 0,
    folders: ["Pets"],
    environments: ["Local"],
    warnings: location.includes("warn") ? ["Only implicit flows"] : [],
    remote: location.includes("remote") && !args.allowRemote ? [REMOTE] : [],
  });
}

function install(args: SpecImportArgs): Promise<SpecImported> {
  if (args.target.kind === "new") {
    return reject("invalid_file", "That folder already holds a collection");
  }
  const { root, folderName } = args.target;
  const done = mockInstall(root, folderName, [
    {
      id: "listPets",
      folder: "Pets",
      stored: {
        ...mockStored("List pets", "{{baseUrl}}/pets"),
        variables: { baseUrl: "http://localhost" },
      },
    },
  ]);
  if (!done) return reject("not_open", "Open the collection first");
  const dir = `openapi/pets${specs.get(root)?.length ? `-${String((specs.get(root)?.length ?? 0) + 1)}` : ""}`;
  const location = args.source.kind === "url" ? args.source.url : args.source.path;
  const spec: StoredSpec = {
    dir,
    title: "Pets",
    source: location,
    folder: done.folder,
    operations: done.files,
  };
  specs.set(root, [...(specs.get(root) ?? []), spec]);
  return Promise.resolve({
    root,
    folder: done.folder,
    installed: { requests: done.files, environments: ["local"], specDir: dir, warnings: [] },
  });
}

export function specCommand(
  command: string,
  args: Record<string, unknown>,
): Promise<unknown> | null {
  switch (command) {
    case "spec_read":
      return summary(args as unknown as SpecReadArgs);
    case "spec_import":
      return install(args as unknown as SpecImportArgs);
    case "spec_list":
      return Promise.resolve(specs.get(args.root as string) ?? []);
    case "spec_open":
      return Promise.resolve(PETS_API);
    case "spec_validate": {
      const body = String(args.body);
      const ok = body.trim().startsWith("[");
      return Promise.resolve({
        response: "200",
        mediaType: "application/json",
        checked: true,
        errors: ok ? [] : [{ pointer: "", message: `${body.slice(0, 20)} is not of type "array"` }],
        note: null,
      });
    }
    default:
      return null;
  }
}
