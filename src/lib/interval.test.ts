import { describe, expect, it } from "vitest";
import { parseInterval } from "./interval";

describe("parseInterval", () => {
  it("builds an interval from text and a unit", () => {
    expect(parseInterval("250", "milliseconds")).toEqual({ ok: true, value: { value: 250, unit: "milliseconds" } });
    expect(parseInterval("1.5", "seconds")).toEqual({ ok: true, value: { value: 1.5, unit: "seconds" } });
    expect(parseInterval("2", "minutes")).toEqual({ ok: true, value: { value: 2, unit: "minutes" } });
  });

  it("accepts exactly one millisecond in any unit", () => {
    expect(parseInterval("1", "milliseconds").ok).toBe(true);
    expect(parseInterval("0.001", "seconds").ok).toBe(true);
  });

  it("rejects less than one millisecond after converting the unit", () => {
    expect(parseInterval("0.9", "milliseconds")).toEqual({ ok: false, error: "too_short" });
    expect(parseInterval("0", "seconds")).toEqual({ ok: false, error: "too_short" });
    expect(parseInterval("0.00001", "minutes")).toEqual({ ok: false, error: "too_short" });
  });

  it("rejects empty text", () => {
    expect(parseInterval("", "seconds")).toEqual({ ok: false, error: "empty" });
    expect(parseInterval("   ", "seconds")).toEqual({ ok: false, error: "empty" });
  });

  it("rejects anything that is not a plain non-negative number", () => {
    for (const text of ["abc", "-5", "1e3", "1,5", "1.2.3", "Infinity", "NaN", "5 ms"]) {
      expect(parseInterval(text, "milliseconds"), text).toEqual({ ok: false, error: "not_a_number" });
    }
  });

  it("trims spaces and reads full-width digits", () => {
    expect(parseInterval("  300  ", "milliseconds")).toEqual({ ok: true, value: { value: 300, unit: "milliseconds" } });
    expect(parseInterval("１２０", "milliseconds")).toEqual({ ok: true, value: { value: 120, unit: "milliseconds" } });
    expect(parseInterval("０．５", "seconds")).toEqual({ ok: true, value: { value: 0.5, unit: "seconds" } });
  });
});
