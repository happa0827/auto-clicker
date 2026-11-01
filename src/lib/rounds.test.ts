import { describe, expect, it } from "vitest";
import { parseRounds } from "./rounds";
import { describeFieldError } from "./field-errors";

describe("parseRounds", () => {
  it("reads a positive whole number", () => {
    expect(parseRounds("5")).toEqual({ ok: true, count: 5 });
    expect(parseRounds(" １０ ")).toEqual({ ok: true, count: 10 });
    expect(parseRounds("4294967295")).toEqual({ ok: true, count: 4294967295 });
  });

  it("rejects empty text", () => {
    expect(parseRounds("")).toEqual({ ok: false, error: "empty" });
  });

  it("rejects zero, which Rust refuses too", () => {
    expect(parseRounds("0")).toEqual({ ok: false, error: "too_small" });
  });

  it("rejects what is not a whole number", () => {
    for (const text of ["1.5", "-3", "abc", "1e3", "5 回"]) {
      expect(parseRounds(text), text).toEqual({ ok: false, error: "not_an_integer" });
    }
  });

  it("rejects a number that does not fit in 32 bits", () => {
    expect(parseRounds("4294967296")).toEqual({ ok: false, error: "too_large" });
  });
});

describe("describeFieldError", () => {
  it("has a plain sentence for every error", () => {
    expect(describeFieldError("empty")).toBe("入力してください。");
    expect(describeFieldError("not_a_number")).toBe("数値で入力してください。");
    expect(describeFieldError("not_an_integer")).toBe("整数で入力してください。");
    expect(describeFieldError("too_short")).toBe("1 ミリ秒以上にしてください。");
    expect(describeFieldError("too_small")).toBe("1 以上にしてください。");
    expect(describeFieldError("too_large")).toBe("大きすぎます。");
  });

  it("returns nothing when there is no error", () => {
    expect(describeFieldError(undefined)).toBeUndefined();
  });
});
