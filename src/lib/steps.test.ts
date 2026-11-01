import { describe, expect, it } from "vitest";
import { addStep, moveStep, removeStep, updateStep } from "./steps";
import type { Step } from "./types";

const step = (x: number, ms = 100): Step => ({
  target: { position: { type: "fixed", x, y: 0 }, button: "left", kind: { type: "single" } },
  interval: { value: ms, unit: "milliseconds" },
});

const xs = (steps: Step[]) =>
  steps.map((s) => (s.target.position.type === "fixed" ? s.target.position.x : -1));

describe("addStep", () => {
  it("appends to the end without touching the original", () => {
    const original = [step(1)];
    const next = addStep(original, step(2));
    expect(xs(next)).toEqual([1, 2]);
    expect(xs(original)).toEqual([1]);
  });
});

describe("removeStep", () => {
  it("removes by index", () => {
    expect(xs(removeStep([step(1), step(2), step(3)], 1))).toEqual([1, 3]);
  });

  it("can remove the last remaining step, leaving an empty list", () => {
    expect(removeStep([step(1)], 0)).toEqual([]);
  });

  it("ignores an index that is out of range", () => {
    const original = [step(1), step(2)];
    expect(xs(removeStep(original, 5))).toEqual([1, 2]);
    expect(xs(removeStep(original, -1))).toEqual([1, 2]);
  });
});

describe("moveStep", () => {
  it("moves a step forward and backward", () => {
    const list = [step(1), step(2), step(3), step(4)];
    expect(xs(moveStep(list, 0, 2))).toEqual([2, 3, 1, 4]);
    expect(xs(moveStep(list, 3, 1))).toEqual([1, 4, 2, 3]);
  });

  it("keeps the order when the step stays where it is", () => {
    expect(xs(moveStep([step(1), step(2)], 1, 1))).toEqual([1, 2]);
  });

  it("clamps the destination into the list", () => {
    const list = [step(1), step(2), step(3)];
    expect(xs(moveStep(list, 0, 99))).toEqual([2, 3, 1]);
    expect(xs(moveStep(list, 2, -5))).toEqual([3, 1, 2]);
  });

  it("ignores a source index that is out of range", () => {
    expect(xs(moveStep([step(1), step(2)], 7, 0))).toEqual([1, 2]);
  });

  it("does not modify the original", () => {
    const original = [step(1), step(2)];
    moveStep(original, 0, 1);
    expect(xs(original)).toEqual([1, 2]);
  });
});

describe("updateStep", () => {
  it("replaces only the chosen step", () => {
    const next = updateStep([step(1), step(2)], 1, step(9, 500));
    expect(xs(next)).toEqual([1, 9]);
    expect(next[1].interval.value).toBe(500);
  });

  it("ignores an index that is out of range", () => {
    expect(xs(updateStep([step(1)], 3, step(9)))).toEqual([1]);
  });
});
