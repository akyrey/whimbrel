import type { Response, StreamEvent } from "../../ipc";

const sent: { onEvent?: (e: StreamEvent) => void; finish?: (r: Response) => void } = {};

vi.mock("../../ipc", async (original) => ({
  ...(await original<typeof import("../../ipc")>()),
  requestSend: (_args: unknown, onEvent: (e: StreamEvent) => void) =>
    new Promise<Response>((resolve) => {
      sent.onEvent = onEvent;
      sent.finish = resolve;
    }),
}));

const { useTabs } = await import("./tabsStore");

const event = (index: number): StreamEvent => ({
  index,
  event: null,
  id: null,
  data: `e${String(index)}`,
});

it("collects stream events while sending and keeps them with the response", async () => {
  useTabs.setState({ tabs: [], activeId: null });
  useTabs.getState().newScratch();
  const id = useTabs.getState().activeId ?? "";

  const sending = useTabs.getState().send(id);
  sent.onEvent?.(event(0));
  sent.onEvent?.(event(1));
  const midway = useTabs.getState().tabs[0]?.status;
  sent.finish?.({ status: 200 } as Response);
  await sending;

  expect(midway).toEqual({ kind: "sending", events: [event(0), event(1)] });
  expect(useTabs.getState().tabs[0]?.status).toMatchObject({
    kind: "done",
    events: [event(0), event(1)],
  });
});
