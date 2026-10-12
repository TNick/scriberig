/** @file Verifies incremental Server-Sent Event parsing. */

import { describe, expect, it } from "vitest";
import { SseParser } from "../src";

describe("SseParser", () => {
  it("preserves frames across arbitrary chunk boundaries", () => {
    const parser = new SseParser();
    expect(parser.push("id: 7\neve")).toEqual([]);
    expect(parser.push('nt: protocol\ndata: {"a":')).toEqual([]);
    expect(parser.push("1}\n\n: heartbeat\n\n")).toEqual([
      { id: "7", event: "protocol", data: '{"a":1}' },
    ]);
  });

  it("joins repeated data fields and ignores comments", () => {
    const parser = new SseParser();
    expect(parser.push(": keepalive\r\ndata: first\r\ndata: second\r\n\r\n")).toEqual([
      { data: "first\nsecond" },
    ]);
  });

  it("flushes a final frame without a trailing blank line", () => {
    const parser = new SseParser();
    parser.push("data: final");
    expect(parser.flush()).toEqual([{ data: "final" }]);
  });
});
/** @file Verifies incremental Server-Sent Event parsing. */
