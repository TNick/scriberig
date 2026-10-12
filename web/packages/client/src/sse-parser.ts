/** @file Parses Server-Sent Events across arbitrary fetch chunk boundaries. */

/** One decoded Server-Sent Event frame. */
export interface SseFrame {
  /** Optional resumable stream cursor. */
  readonly id?: string;
  /** Optional semantic event name. */
  readonly event?: string;
  /** Concatenated event data lines. */
  readonly data: string;
}

/**
 * Incrementally decodes SSE framing without interpreting event payloads.
 *
 * Public members
 * - `push`: Decode one arbitrary text chunk.
 * - `flush`: Decode the final buffered frame.
 *
 * Private members
 * - Buffer and frame fields hold the in-progress SSE record.
 */
export class SseParser {
  private buffer = "";
  private data: string[] = [];
  private id: string | undefined;
  private event: string | undefined;

  /** Adds one arbitrary text chunk and returns complete frames. */
  push(chunk: string): SseFrame[] {
    this.buffer += chunk;
    const frames: SseFrame[] = [];
    let boundary = this.lineBoundary();
    while (boundary !== undefined) {
      const line = this.buffer.slice(0, boundary.index);
      this.buffer = this.buffer.slice(boundary.nextIndex);
      this.consumeLine(line, frames);
      boundary = this.lineBoundary();
    }
    return frames;
  }

  /** Flushes a final unterminated event frame, if one exists. */
  flush(): SseFrame[] {
    const frames: SseFrame[] = [];
    if (this.buffer.length > 0) {
      this.consumeLine(this.buffer, frames);
      this.buffer = "";
    }
    this.finish(frames);
    return frames;
  }

  /**
   * Finds the next LF or CRLF boundary.
   * @returns The next line boundary, when one is buffered.
   */
  private lineBoundary(): { index: number; nextIndex: number } | undefined {
    const lf = this.buffer.indexOf("\n");
    if (lf < 0) return undefined;
    return {
      index: lf > 0 && this.buffer[lf - 1] === "\r" ? lf - 1 : lf,
      nextIndex: lf + 1,
    };
  }

  /**
   * Applies one field line or dispatches a blank-frame boundary.
   * @param line One decoded SSE line.
   * @param frames Frames emitted during processing.
   */
  private consumeLine(line: string, frames: SseFrame[]): void {
    if (line === "") {
      this.finish(frames);
      return;
    }
    if (line.startsWith(":")) return;
    const separator = line.indexOf(":");
    const field = separator < 0 ? line : line.slice(0, separator);
    const value = separator < 0 ? "" : line.slice(separator + 1).replace(/^ /, "");
    if (field === "data") this.data.push(value);
    if (field === "id" && !value.includes("\0")) this.id = value;
    if (field === "event") this.event = value;
  }

  /**
   * Emits the current frame and resets its fields.
   * @param frames Frames receiving the completed event.
   */
  private finish(frames: SseFrame[]): void {
    if (this.data.length === 0) {
      this.id = undefined;
      this.event = undefined;
      return;
    }
    frames.push({
      ...(this.id === undefined ? {} : { id: this.id }),
      ...(this.event === undefined ? {} : { event: this.event }),
      data: this.data.join("\n"),
    });
    this.data = [];
    this.id = undefined;
    this.event = undefined;
  }
}
