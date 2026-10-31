/** @file Compares and combines CAD decimal coordinates without binary rounding. */

interface DecimalParts {
  /** Sign of the normalized decimal value. */
  sign: -1 | 0 | 1;
  /** Whole-number digits without leading zeroes. */
  whole: string;
  /** Fractional digits without trailing zeroes. */
  fraction: string;
}

/** Chooses the exact decimal minimum without converting coordinates to binary floats.
 * @param values Exact decimal coordinates.
 * @returns Original spelling of the smallest valid decimal, or undefined if invalid.
 */
export function exactMinimum(values: readonly string[]): string | undefined {
  return exactExtreme(values, -1);
}

/** Chooses the exact decimal maximum without converting coordinates to binary floats.
 * @param values Exact decimal coordinates.
 * @returns Original spelling of the largest valid decimal, or undefined if invalid.
 */
export function exactMaximum(values: readonly string[]): string | undefined {
  return exactExtreme(values, 1);
}

/** Adds two plain exact decimal values.
 * @param left First exact decimal value.
 * @param right Second exact decimal value.
 * @returns Exact decimal sum, or undefined for unsupported syntax.
 */
export function exactAdd(left: string, right: string): string | undefined {
  return exactArithmetic(left, right, 1);
}

/** Subtracts one plain exact decimal value from another.
 * @param left Exact decimal minuend.
 * @param right Exact decimal subtrahend.
 * @returns Exact decimal difference, or undefined for unsupported syntax.
 */
export function exactSubtract(left: string, right: string): string | undefined {
  return exactArithmetic(left, right, -1);
}

/** Compares exact decimal values without binary rounding.
 * @param left First exact decimal.
 * @param right Second exact decimal.
 * @returns Negative, zero, or positive according to numeric ordering.
 */
export function compareExactDecimal(left: string, right: string): number | undefined {
  const first = decimalParts(left);
  const second = decimalParts(right);
  if (first === undefined || second === undefined) return undefined;
  if (first.sign !== second.sign) return Math.sign(first.sign - second.sign);
  if (first.sign === 0) return 0;
  const wholeOrder =
    first.whole.length - second.whole.length || first.whole.localeCompare(second.whole);
  const fractionLength = Math.max(first.fraction.length, second.fraction.length);
  const fractionOrder = first.fraction
    .padEnd(fractionLength, "0")
    .localeCompare(second.fraction.padEnd(fractionLength, "0"));
  const magnitude = wholeOrder || fractionOrder;
  return first.sign === -1 ? -Math.sign(magnitude) : Math.sign(magnitude);
}

/** Selects one decimal extreme using exact digit-wise comparison.
 * @param values Exact decimal coordinates.
 * @param direction Negative for minimum and positive for maximum.
 * @returns Original spelling of the requested extreme.
 */
function exactExtreme(values: readonly string[], direction: -1 | 1): string | undefined {
  if (values.length === 0 || values.some((value) => decimalParts(value) === undefined)) {
    return undefined;
  }
  return values.reduce((best, value) =>
    (compareExactDecimal(value, best) ?? 0) * direction > 0 ? value : best,
  );
}

/** Parses a plain exact decimal into normalized digit strings.
 * @param value Coordinate spelling.
 * @returns Decimal components or undefined for unsupported syntax.
 */
function decimalParts(value: string): DecimalParts | undefined {
  const match = /^([+-]?)(\d*)(?:\.(\d*))?$/.exec(value);
  const whole = match?.[2];
  if (match === null || whole === undefined || (whole === "" && match[3] === undefined)) {
    return undefined;
  }
  const integer = whole.replace(/^0+/, "") || "0";
  const fraction = (match[3] ?? "").replace(/0+$/, "");
  const zero = integer === "0" && fraction === "";
  const sign = zero ? 0 : match[1] === "-" ? -1 : 1;
  return { sign, whole: integer, fraction };
}

/** Adds or subtracts decimals using scaled integer arithmetic.
 * @param left Exact decimal left operand.
 * @param right Exact decimal right operand.
 * @param direction Positive to add and negative to subtract.
 * @returns Exact normalized decimal result or undefined for invalid syntax.
 */
function exactArithmetic(left: string, right: string, direction: -1 | 1): string | undefined {
  const first = decimalParts(left);
  const second = decimalParts(right);
  if (first === undefined || second === undefined) return undefined;
  const scale = Math.max(first.fraction.length, second.fraction.length);
  const leftInteger = scaledInteger(first, scale);
  const rightInteger = scaledInteger(second, scale) * BigInt(direction);
  return formatScaled(leftInteger + rightInteger, scale);
}

/** Converts decimal parts into a signed scaled integer.
 * @param parts Normalized decimal components.
 * @param scale Number of fractional digits in the target scale.
 * @returns Signed integer with the requested decimal scale.
 */
function scaledInteger(parts: DecimalParts, scale: number): bigint {
  const digits = parts.whole + parts.fraction.padEnd(scale, "0");
  return BigInt(digits) * BigInt(parts.sign);
}

/** Formats a scaled integer as a normalized exact decimal string.
 * @param value Signed integer decimal payload.
 * @param scale Number of fractional digits represented by the payload.
 * @returns Exact decimal spelling without redundant trailing zeroes.
 */
function formatScaled(value: bigint, scale: number): string {
  if (value === 0n) return "0";
  const sign = value < 0n ? "-" : "";
  const digits = String(value < 0n ? -value : value).padStart(scale + 1, "0");
  if (scale === 0) return sign + digits;
  const split = digits.length - scale;
  const whole = digits.slice(0, split);
  const fraction = digits.slice(split).replace(/0+$/, "");
  return fraction === "" ? sign + whole : `${sign}${whole}.${fraction}`;
}
