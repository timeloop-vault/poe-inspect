const isIntRegex = /^(-|\+)?[0-9]*$/;
const isFloatRegex = /^(-|\+)?[0-9]*\.[0-9]*$/;

export function parseNumber(value: string): number | null {
  if (value.match(isFloatRegex)) {
    return parseFloat(value);
  } else if (value.match(isIntRegex)) {
    return parseInt(value, 10);
  }
  return null;
}
