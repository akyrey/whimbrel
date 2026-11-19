import { Button } from "@roost/ui";

import { type FormField, type StoredRequest, chooseFile } from "../../ipc";
import { addField, removeField, setFilePath, updateField } from "./editing";

type Edit = (change: (draft: StoredRequest) => StoredRequest) => void;

const ANY_FILE = [{ name: "Any file", extensions: ["*"] }];

function FilePicker({
  label,
  path,
  onPath,
}: {
  label: string;
  path: string;
  onPath: (p: string) => void;
}) {
  return (
    <span className="flex min-w-0 flex-1 items-center gap-2">
      <Button
        aria-label={label}
        onClick={() =>
          void chooseFile("Choose a file to send", ANY_FILE).then((chosen) => {
            if (chosen !== null) onPath(chosen);
          })
        }
      >
        Choose file…
      </Button>
      <input
        aria-label={`${label} path`}
        placeholder="/path/to/file"
        className="input min-w-0 flex-1 font-mono"
        value={path}
        onChange={(e) => {
          onPath(e.target.value);
        }}
      />
    </span>
  );
}

function FieldRow({
  field,
  index,
  multipart,
  edit,
}: {
  field: FormField;
  index: number;
  multipart: boolean;
  edit: Edit;
}) {
  const n = String(index + 1);
  return (
    <div className="flex items-center gap-2">
      <input
        type="checkbox"
        aria-label={`Send field ${n}`}
        checked={field.enabled}
        onChange={(e) => {
          edit(updateField(index, { enabled: e.target.checked }));
        }}
      />
      <input
        aria-label={`Field ${n} name`}
        placeholder="Name"
        className="input w-48 font-mono"
        value={field.name}
        onChange={(e) => {
          edit(updateField(index, { name: e.target.value }));
        }}
      />
      {multipart && (
        <select
          aria-label={`Field ${n} type`}
          className="input"
          value={field.file ? "file" : "text"}
          onChange={(e) => {
            edit(updateField(index, { file: e.target.value === "file", value: "" }));
          }}
        >
          <option value="text">Text</option>
          <option value="file">File</option>
        </select>
      )}
      {field.file ? (
        <FilePicker
          label={`Field ${n} file`}
          path={field.value}
          onPath={(value) => {
            edit(updateField(index, { value }));
          }}
        />
      ) : (
        <input
          aria-label={`Field ${n} value`}
          placeholder="Value"
          className="input min-w-0 flex-1 font-mono"
          value={field.value}
          onChange={(e) => {
            edit(updateField(index, { value: e.target.value }));
          }}
        />
      )}
      <Button
        aria-label={`Remove field ${n}`}
        onClick={() => {
          edit(removeField(index));
        }}
      >
        ✕
      </Button>
    </div>
  );
}

/** The fields of a form-urlencoded or multipart body (FR-2.2). */
export function FieldsEditor({
  fields,
  multipart,
  edit,
}: {
  fields: FormField[];
  multipart: boolean;
  edit: Edit;
}) {
  return (
    <div aria-label="Form fields" role="group" className="flex flex-col gap-1 text-sm">
      {fields.map((field, index) => (
        <FieldRow key={index} field={field} index={index} multipart={multipart} edit={edit} />
      ))}
      <div>
        <Button
          onClick={() => {
            edit(addField);
          }}
        >
          Add field
        </Button>
      </div>
    </div>
  );
}

/** A file sent as the whole body. */
export function FileBodyEditor({ path, edit }: { path: string; edit: Edit }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <FilePicker
        label="Body file"
        path={path}
        onPath={(p) => {
          edit(setFilePath(p));
        }}
      />
    </div>
  );
}
