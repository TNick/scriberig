/** @file Verifies exact decimal helpers used by CAD projection bounds. */
import { describe, expect, it } from "vitest";
import {
  compareExactDecimal,
  exactAdd,
  exactMaximum,
  exactMinimum,
  exactSubtract,
} from "../src/cad-geometry-decimal";

describe("CAD exact decimal projection bounds", () => {
  it("adds and subtracts coordinates without binary floating point", () => {
    expect(exactAdd("1.20", "0.30")).toBe("1.5");
    expect(exactSubtract("-2.1", "0.50")).toBe("-2.6");
  });

  it("orders large and signed decimal coordinates exactly", () => {
    expect(compareExactDecimal("9007199254740993", "9007199254740992")).toBe(1);
    expect(exactMinimum(["10", "2.25", "-0.5"])).toBe("-0.5");
    expect(exactMaximum(["10", "2.25", "-0.5"])).toBe("10");
  });

  it("rejects exponent and malformed coordinate spellings", () => {
    expect(exactAdd("1e3", "2")).toBeUndefined();
    expect(exactMinimum(["1", "NaN"])).toBeUndefined();
  });
});
