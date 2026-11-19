import { clearMocks } from "@tauri-apps/api/mocks";

import { DEMO_ROOT, installMockBackend } from "../../test/mockBackend";
import { useTabs } from "./tabsStore";

beforeEach(() => {
  installMockBackend();
  useTabs.setState({ tabs: [], activeId: null });
});

afterEach(() => {
  clearMocks();
});

describe("tabs store", () => {
  it("opens a file once even when two opens race (a restore run twice)", async () => {
    const file = { root: DEMO_ROOT, path: "ping.req.toml" };

    await Promise.all([useTabs.getState().openFile(file), useTabs.getState().openFile(file)]);

    expect(useTabs.getState().tabs).toHaveLength(1);
  });

  it("closes to the neighbour and keeps another active tab", () => {
    const { newScratch, activate, close } = useTabs.getState();
    newScratch();
    newScratch();
    newScratch();
    const [a, b, c] = useTabs.getState().tabs.map((t) => t.id);

    close(c ?? "");
    expect(useTabs.getState().activeId).toBe(b);
    activate(a ?? "");
    close(b ?? "");
    expect(useTabs.getState().activeId).toBe(a);
  });
});

describe("saving", () => {
  it("keeps the edits when the file changed meanwhile, and puts back a deleted file", async () => {
    const { externalEdit } = await import("../../test/mockBackend");
    const file = { root: DEMO_ROOT, path: "ping.req.toml" };
    await useTabs.getState().openFile(file);
    const id = useTabs.getState().activeId ?? "";
    useTabs.getState().edit(id, (d) => ({ ...d, name: "Mine" }));
    await externalEdit("ping.req.toml", (s) => ({ ...s, order: 99 }));

    await expect(useTabs.getState().save(id)).rejects.toMatchObject({ code: "conflict" });

    await externalEdit("ping.req.toml", null);
    await useTabs.getState().changedOnDisk(DEMO_ROOT, ["ping.req.toml"]);
    expect(useTabs.getState().tabs[0]?.missing).toBe(true);
    await useTabs.getState().save(useTabs.getState().activeId ?? "");
    const tab = useTabs.getState().tabs[0];
    expect(tab?.missing).toBe(false);
    expect(tab?.draft.name).toBe("Mine");
  });
});

describe("a restart", () => {
  it("brings back unsaved edits and scratch tabs", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    localStorage.clear();
    const file = { root: DEMO_ROOT, path: "ping.req.toml" };
    await useTabs.getState().openFile(file);
    const fileId = useTabs.getState().activeId ?? "";
    useTabs.getState().edit(fileId, (d) => ({ ...d, name: "Ping (edited)" }));
    useTabs.getState().newScratch();
    const scratchId = useTabs.getState().activeId ?? "";
    useTabs
      .getState()
      .edit(scratchId, (d) => ({ ...d, request: { ...d.request, url: "http://x/draft" } }));
    vi.advanceTimersByTime(1000);
    vi.useRealTimers();

    useTabs.setState({ tabs: [], activeId: null });
    await useTabs.getState().restore();

    const tabs = useTabs.getState().tabs;
    expect(tabs.find((t) => t.id === fileId)?.draft.name).toBe("Ping (edited)");
    expect(tabs.find((t) => t.id === scratchId)?.draft.request.url).toBe("http://x/draft");
    expect(useTabs.getState().activeId).toBe(scratchId);
    useTabs.getState().newScratch();
    expect(useTabs.getState().activeId).not.toBe(scratchId);
  });
});
