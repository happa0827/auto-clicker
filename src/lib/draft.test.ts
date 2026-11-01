import { describe, expect, it } from "vitest";
import { applyCapture, buildSequence, defaultDraft, draftFromSequence, newStepDraft, type Draft } from "./draft";
import type { Sequence, Step } from "./types";

const draft = (patch: Partial<Draft> = {}): Draft => ({ ...defaultDraft(), ...patch });

describe("buildSequence: current position mode", () => {
  it("makes a one-step sequence that clicks where the cursor is", () => {
    const result = buildSequence(draft({ mode: "current", interval: { text: "200", unit: "milliseconds" } }));
    expect(result).toEqual({
      ok: true,
      sequence: {
        steps: [
          {
            target: { position: { type: "current" }, button: "left", kind: { type: "single" } },
            interval: { value: 200, unit: "milliseconds" },
          },
        ],
      },
    });
  });

  it("ignores the coordinate fields", () => {
    const result = buildSequence(draft({ mode: "current", target: { ...defaultDraft().target, x: "abc", y: "" } }));
    expect(result.ok).toBe(true);
  });
});

describe("buildSequence: fixed position mode", () => {
  it("reads integer coordinates, including negative ones and full-width digits", () => {
    const result = buildSequence(
      draft({ mode: "fixed", target: { ...defaultDraft().target, x: "-1920", y: "１０８０" } }),
    );
    expect(result.ok && result.sequence.steps[0].target.position).toEqual({ type: "fixed", x: -1920, y: 1080 });
  });

  it("reports each bad coordinate on its own field", () => {
    const result = buildSequence(draft({ mode: "fixed", target: { ...defaultDraft().target, x: "1.5", y: "" } }));
    expect(result).toEqual({ ok: false, errors: { x: "not_an_integer", y: "empty" } });
  });

  it("rejects coordinates outside the 32-bit range", () => {
    const result = buildSequence(draft({ mode: "fixed", target: { ...defaultDraft().target, x: "3000000000", y: "0" } }));
    expect(result).toEqual({ ok: false, errors: { x: "not_an_integer" } });
  });
});

describe("buildSequence: click kind and button", () => {
  it("passes the button and double click through", () => {
    const result = buildSequence(
      draft({ mode: "current", target: { ...defaultDraft().target, button: "right", kind: "double" } }),
    );
    expect(result.ok && result.sequence.steps[0].target).toMatchObject({ button: "right", kind: { type: "double" } });
  });

  it("builds a hold with its own duration", () => {
    const result = buildSequence(
      draft({
        mode: "current",
        target: { ...defaultDraft().target, kind: "hold", holdText: "2", holdUnit: "seconds" },
      }),
    );
    expect(result.ok && result.sequence.steps[0].target.kind).toEqual({
      type: "hold",
      duration: { value: 2, unit: "seconds" },
    });
  });

  it("only checks the hold duration when the kind is hold", () => {
    const single = buildSequence(draft({ mode: "current", target: { ...defaultDraft().target, holdText: "oops" } }));
    expect(single.ok).toBe(true);
    const hold = buildSequence(
      draft({ mode: "current", target: { ...defaultDraft().target, kind: "hold", holdText: "0" } }),
    );
    expect(hold).toEqual({ ok: false, errors: { hold: "too_short" } });
  });
});

describe("buildSequence: interval", () => {
  it("reports an invalid interval", () => {
    expect(buildSequence(draft({ interval: { text: "0.5", unit: "milliseconds" } }))).toEqual({
      ok: false,
      errors: { interval: "too_short" },
    });
    expect(buildSequence(draft({ interval: { text: "", unit: "seconds" } }))).toEqual({
      ok: false,
      errors: { interval: "empty" },
    });
  });

  it("reports every problem at once", () => {
    const result = buildSequence(
      draft({
        mode: "fixed",
        interval: { text: "x", unit: "seconds" },
        target: { ...defaultDraft().target, x: "a", y: "b", kind: "hold", holdText: "" },
      }),
    );
    expect(result).toEqual({
      ok: false,
      errors: { interval: "not_a_number", x: "not_an_integer", y: "not_an_integer", hold: "empty" },
    });
  });
});

describe("buildSequence: sequence mode", () => {
  const row = (x: string, y: string, ms = "50") => ({
    target: { ...defaultDraft().target, x, y },
    interval: { text: ms, unit: "milliseconds" as const },
  });

  it("builds every row in order as a fixed-position step", () => {
    const result = buildSequence(draft({ mode: "sequence", steps: [row("1", "2"), row("3", "4", "75")] }));
    expect(result).toEqual({
      ok: true,
      sequence: {
        steps: [
          {
            target: { position: { type: "fixed", x: 1, y: 2 }, button: "left", kind: { type: "single" } },
            interval: { value: 50, unit: "milliseconds" },
          },
          {
            target: { position: { type: "fixed", x: 3, y: 4 }, button: "left", kind: { type: "single" } },
            interval: { value: 75, unit: "milliseconds" },
          },
        ],
      },
    });
  });

  it("needs at least one step", () => {
    expect(buildSequence(draft({ mode: "sequence", steps: [] }))).toEqual({ ok: false, errors: { steps: "empty" } });
  });

  it("reports errors per row, by position, and keeps good rows out of the list", () => {
    const result = buildSequence(
      draft({ mode: "sequence", steps: [row("1", "2"), row("x", "4", "0"), row("5", "6")] }),
    );
    expect(result).toEqual({
      ok: false,
      errors: { stepErrors: { 1: { x: "not_an_integer", interval: "too_short" } } },
    });
  });
});

describe("draftFromSequence", () => {
  it("restores a single current-position step as current mode", () => {
    const built = buildSequence(draft({ mode: "current", interval: { text: "300", unit: "seconds" } }));
    if (!built.ok) throw new Error("setup");
    const restored = draftFromSequence(built.sequence);
    expect(restored.mode).toBe("current");
    expect(restored.interval).toEqual({ text: "300", unit: "seconds" });
    expect(buildSequence(restored)).toEqual(built);
  });

  it("restores a single fixed step as fixed mode with its coordinates and hold", () => {
    const sequence: Sequence = {
      steps: [
        {
          target: {
            position: { type: "fixed", x: -5, y: 40 },
            button: "middle",
            kind: { type: "hold", duration: { value: 1.5, unit: "seconds" } },
          },
          interval: { value: 2, unit: "minutes" },
        },
      ],
    };
    const restored = draftFromSequence(sequence);
    expect(restored.mode).toBe("fixed");
    expect(restored.target).toMatchObject({ x: "-5", y: "40", button: "middle", kind: "hold", holdText: "1.5", holdUnit: "seconds" });
    expect(buildSequence(restored)).toEqual({ ok: true, sequence });
  });

  it("restores several steps as sequence mode, keeping order", () => {
    const at = (x: number): Step => ({
      target: { position: { type: "fixed", x, y: 0 }, button: "left", kind: { type: "double" } },
      interval: { value: 10 * x, unit: "milliseconds" },
    });
    const sequence: Sequence = { steps: [at(1), at(2), at(3)] };
    const restored = draftFromSequence(sequence);
    expect(restored.mode).toBe("sequence");
    expect(restored.steps).toHaveLength(3);
    expect(buildSequence(restored)).toEqual({ ok: true, sequence });
  });

  it("keeps a current-position step inside a multi-step sequence unchanged", () => {
    const current: Step = {
      target: { position: { type: "current" }, button: "left", kind: { type: "single" } },
      interval: { value: 5, unit: "milliseconds" },
    };
    const fixed: Step = {
      target: { position: { type: "fixed", x: 9, y: 9 }, button: "left", kind: { type: "single" } },
      interval: { value: 6, unit: "milliseconds" },
    };
    const sequence: Sequence = { steps: [current, fixed] };
    const restored = draftFromSequence(sequence);
    expect(restored.mode).toBe("sequence");
    expect(buildSequence(restored)).toEqual({ ok: true, sequence });
  });
});

describe("newStepDraft", () => {
  it("makes a row at the captured position with the default click", () => {
    const row = newStepDraft(120, -30);
    expect(row.target).toMatchObject({ position: "fixed", x: "120", y: "-30", button: "left", kind: "single" });
    expect(row.interval).toEqual(defaultDraft().interval);
  });
});

describe("applyCapture", () => {
  it("adds a row in sequence mode and keeps the earlier rows", () => {
    const first = newStepDraft(1, 2);
    const next = applyCapture(draft({ mode: "sequence", steps: [first] }), 30, 40);
    expect(next.steps).toHaveLength(2);
    expect(next.steps[0]).toEqual(first);
    expect(next.steps[1].target).toMatchObject({ x: "30", y: "40", position: "fixed" });
  });

  it("fills in the coordinates in fixed mode", () => {
    const next = applyCapture(draft({ mode: "fixed" }), -7, 99);
    expect(next.target).toMatchObject({ x: "-7", y: "99" });
    expect(next.mode).toBe("fixed");
  });

  it("does nothing in current mode, where no coordinates are used", () => {
    const before = draft({ mode: "current" });
    expect(applyCapture(before, 5, 5)).toEqual(before);
  });

  it("does not modify the draft it was given", () => {
    const before = draft({ mode: "sequence", steps: [] });
    applyCapture(before, 1, 1);
    expect(before.steps).toEqual([]);
  });
});
