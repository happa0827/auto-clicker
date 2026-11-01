import type { FieldError } from "./draft";
import type { RoundsError } from "./rounds";

export function describeFieldError(error: FieldError | RoundsError | undefined): string | undefined {
  switch (error) {
    case undefined:
      return undefined;
    case "empty":
      return "入力してください。";
    case "not_a_number":
      return "数値で入力してください。";
    case "not_an_integer":
      return "整数で入力してください。";
    case "too_short":
      return "1 ミリ秒以上にしてください。";
    case "too_small":
      return "1 以上にしてください。";
    case "too_large":
      return "大きすぎます。";
  }
}
