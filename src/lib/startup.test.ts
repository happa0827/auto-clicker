import { describe, expect, it } from "vitest";
import { describeAutostartError } from "./startup";

describe("describeAutostartError", () => {
  it("explains that the setting was not changed and shows the reason", () => {
    expect(describeAutostartError("denied")).toBe("自動起動の設定を変更できませんでした: denied");
  });

  it("does not leave a dangling colon when there is no reason", () => {
    expect(describeAutostartError("")).toBe("自動起動の設定を変更できませんでした。");
    expect(describeAutostartError("   ")).toBe("自動起動の設定を変更できませんでした。");
  });
});
