import { Button } from "@roost/ui";

import type { Assertion, Capture, StoredRequest } from "../../ipc";
import type { Tab } from "../tabs/model";
import { useTabs } from "../tabs/tabsStore";
import {
  ASSERTION_KINDS,
  type AssertionKind,
  addAssertion,
  addCapture,
  blankAssertion,
  expectedText,
  parseExpected,
  removeAssertion,
  removeCapture,
  setAssertion,
  updateCapture,
} from "./checks";

type Edit = (change: (draft: StoredRequest) => StoredRequest) => void;

function CaptureRow({ capture, index, edit }: { capture: Capture; index: number; edit: Edit }) {
  const n = String(index + 1);
  return (
    <div className="flex flex-wrap items-center gap-2">
      <input
        aria-label={`Capture ${n} variable`}
        placeholder="variable"
        className="input w-32 font-mono"
        value={capture.var}
        onChange={(e) => {
          edit(updateCapture(index, { var: e.target.value.trim() }));
        }}
      />
      <span className="text-zinc-500">from</span>
      <select
        aria-label={`Capture ${n} source`}
        className="input"
        value={capture.from}
        onChange={(e) => {
          edit(updateCapture(index, { from: e.target.value === "header" ? "header" : "body" }));
        }}
      >
        <option value="body">body</option>
        <option value="header">header</option>
      </select>
      <input
        aria-label={`Capture ${n} path`}
        placeholder={capture.from === "body" ? "$.token" : "Location"}
        className="input min-w-0 flex-1 font-mono"
        value={capture.path}
        onChange={(e) => {
          edit(updateCapture(index, { path: e.target.value }));
        }}
      />
      <input
        aria-label={`Capture ${n} regex`}
        placeholder="regex (optional)"
        className="input w-36 font-mono"
        value={capture.regex ?? ""}
        onChange={(e) => {
          edit(updateCapture(index, { regex: e.target.value === "" ? null : e.target.value }));
        }}
      />
      <label className="flex items-center gap-1">
        <input
          type="checkbox"
          aria-label={`Capture ${n} secret`}
          checked={capture.secret}
          onChange={(e) => {
            edit(updateCapture(index, { secret: e.target.checked }));
          }}
        />
        secret
      </label>
      <Button
        aria-label={`Remove capture ${n}`}
        onClick={() => {
          edit(removeCapture(index));
        }}
      >
        ✕
      </Button>
    </div>
  );
}

function AssertionFields({
  assertion,
  n,
  set,
}: {
  assertion: Assertion;
  n: string;
  set: (a: Assertion) => void;
}) {
  const input = (label: string, value: string, change: (v: string) => void, placeholder = "") => (
    <input
      aria-label={`Assertion ${n} ${label}`}
      placeholder={placeholder}
      className="input min-w-0 flex-1 font-mono"
      value={value}
      onChange={(e) => {
        change(e.target.value);
      }}
    />
  );
  const number = (text: string, fallback: number) => {
    const parsed = Number.parseInt(text, 10);
    return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
  };
  switch (assertion.check) {
    case "status":
      return input("status", String(assertion.equals), (v) => {
        set({ ...assertion, equals: number(v, assertion.equals) });
      });
    case "time":
      return input("milliseconds", String(assertion.belowMs), (v) => {
        set({ ...assertion, belowMs: number(v, assertion.belowMs) });
      });
    case "header":
      return (
        <>
          {input(
            "header name",
            assertion.name,
            (v) => {
              set({ ...assertion, name: v });
            },
            "Content-Type",
          )}
          {input(
            "expected",
            assertion.equals ?? "",
            (v) => {
              set({ ...assertion, equals: v === "" ? null : v });
            },
            "exists",
          )}
        </>
      );
    case "json":
      return (
        <>
          {input(
            "path",
            assertion.path,
            (v) => {
              set({ ...assertion, path: v });
            },
            "$.id",
          )}
          {input(
            "expected",
            expectedText(assertion.equals),
            (v) => {
              set({ ...assertion, equals: parseExpected(v) });
            },
            "exists",
          )}
        </>
      );
  }
}

function AssertionRow({
  assertion,
  index,
  edit,
}: {
  assertion: Assertion;
  index: number;
  edit: Edit;
}) {
  const n = String(index + 1);
  const set = (a: Assertion) => {
    edit(setAssertion(index, a));
  };
  return (
    <div className="flex flex-wrap items-center gap-2">
      <select
        aria-label={`Assertion ${n} kind`}
        className="input"
        value={assertion.check}
        onChange={(e) => {
          set(blankAssertion(e.target.value as AssertionKind));
        }}
      >
        {ASSERTION_KINDS.map(([kind, label]) => (
          <option key={kind} value={kind}>
            {label}
          </option>
        ))}
      </select>
      <AssertionFields assertion={assertion} n={n} set={set} />
      <Button
        aria-label={`Remove assertion ${n}`}
        onClick={() => {
          edit(removeAssertion(index));
        }}
      >
        ✕
      </Button>
    </div>
  );
}

/** The request's captures and assertions (FR-6.1, FR-6.2). */
export function ChecksEditor({ tab }: { tab: Tab }) {
  const { captures, assertions } = tab.draft;
  const edit: Edit = (change) => {
    useTabs.getState().edit(tab.id, change);
  };
  return (
    <div className="flex flex-col gap-1">
      {captures.map((c, i) => (
        <CaptureRow key={i} capture={c} index={i} edit={edit} />
      ))}
      {assertions.map((a, i) => (
        <AssertionRow key={i} assertion={a} index={i} edit={edit} />
      ))}
      <div className="flex gap-2">
        <Button
          onClick={() => {
            edit(addCapture);
          }}
        >
          Add capture
        </Button>
        <Button
          onClick={() => {
            edit(addAssertion);
          }}
        >
          Add assertion
        </Button>
      </div>
    </div>
  );
}
