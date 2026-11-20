import { phasesOf } from "./timing";

describe("phasesOf", () => {
  it("splits a new connection into DNS, connect, waiting and download", () => {
    expect(phasesOf({ dnsMs: 2, connectMs: 5, headersMs: 12, totalMs: 15 })).toEqual([
      { phase: "dns", label: "DNS", startMs: 0, ms: 2 },
      { phase: "connect", label: "Connect (TCP + TLS)", startMs: 2, ms: 5 },
      { phase: "waiting", label: "Waiting", startMs: 7, ms: 5 },
      { phase: "download", label: "Download", startMs: 12, ms: 3 },
    ]);
  });

  it("leaves out DNS for an address and both for a reused connection", () => {
    expect(
      phasesOf({ dnsMs: null, connectMs: 4, headersMs: 10, totalMs: 10 }).map((p) => p.phase),
    ).toEqual(["connect", "waiting", "download"]);
    expect(phasesOf({ dnsMs: null, connectMs: null, headersMs: 3, totalMs: 9 })).toEqual([
      { phase: "waiting", label: "Waiting", startMs: 0, ms: 3 },
      { phase: "download", label: "Download", startMs: 3, ms: 6 },
    ]);
  });

  it("never gives a negative phase when rounding makes the parts exceed the whole", () => {
    const waiting = phasesOf({ dnsMs: 3, connectMs: 3, headersMs: 5, totalMs: 5 }).find(
      (p) => p.phase === "waiting",
    );
    expect(waiting?.ms).toBe(0);
  });
});
