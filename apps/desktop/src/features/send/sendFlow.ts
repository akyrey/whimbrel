import { create } from "zustand";

import { activeEnvironment } from "../environments/environmentsStore";
import { rootOf, useTabs } from "../tabs/tabsStore";

type Pending = { tabId: string; environment: string } | null;

/** A send waiting for confirmation because its environment is protected (FR-4.6). */
export const useSendConfirm = create<{ pending: Pending }>()(() => ({ pending: null }));

/** Sends the tab, asking first when its environment is protected. */
export function sendTab(tabId: string): void {
  const tab = useTabs.getState().tabs.find((t) => t.id === tabId);
  if (!tab || tab.status.kind === "sending") return;
  const environment = activeEnvironment(rootOf(tab));
  if (environment?.protected) {
    useSendConfirm.setState({
      pending: { tabId, environment: environment.name ?? environment.id },
    });
    return;
  }
  void useTabs.getState().send(tabId);
}

export function confirmSend(): void {
  const pending = useSendConfirm.getState().pending;
  useSendConfirm.setState({ pending: null });
  if (pending) void useTabs.getState().send(pending.tabId);
}

export function cancelSend(): void {
  useSendConfirm.setState({ pending: null });
}
