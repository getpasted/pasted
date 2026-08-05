export interface ColorFormats {
  hex: string;
  rgb: string;
  hsl: string;
  cssVar: string;
  tailwindBg: string;
  tailwindText: string;
  r: number;
  g: number;
  b: number;
  contrastWithWhite: number;
  contrastWithBlack: number;
}

export function parseColor(inputStr: string, allowBareHex = false): ColorFormats | null {
  if (!inputStr) return null;
  const str = inputStr.trim();

  let r = 0, g = 0, b = 0;

  // Bare hex is accepted only for clips that were already classified as colors.
  // This prevents six-digit codes and other short numeric text from becoming swatches.
  const hexMatch = str.match(allowBareHex
    ? /^#?([a-f\d]{3}|[a-f\d]{6})$/i
    : /^#([a-f\d]{3}|[a-f\d]{6})$/i);
  if (hexMatch) {
    let hex = hexMatch[1];
    if (hex.length === 3) {
      hex = hex.split('').map((c) => c + c).join('');
    }
    r = parseInt(hex.substring(0, 2), 16);
    g = parseInt(hex.substring(2, 4), 16);
    b = parseInt(hex.substring(4, 6), 16);
  } else {
    // RGB regex rgb(r, g, b)
    const rgbMatch = str.match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)/i);
    if (rgbMatch) {
      r = parseInt(rgbMatch[1], 10);
      g = parseInt(rgbMatch[2], 10);
      b = parseInt(rgbMatch[3], 10);
    } else {
      // HSL regex hsl(h, s%, l%)
      const hslMatch = str.match(/^hsla?\((\d+),\s*(\d+)%,\s*(\d+)%/i);
      if (hslMatch) {
        const h = parseInt(hslMatch[1], 10) / 360;
        const s = parseInt(hslMatch[2], 10) / 100;
        const l = parseInt(hslMatch[3], 10) / 100;
        const rgb = hslToRgb(h, s, l);
        r = rgb[0];
        g = rgb[1];
        b = rgb[2];
      } else {
        return null;
      }
    }
  }

  // Ensure 0-255 bounds
  r = Math.min(255, Math.max(0, r));
  g = Math.min(255, Math.max(0, g));
  b = Math.min(255, Math.max(0, b));

  const hexVal = `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
  const rgbVal = `rgb(${r}, ${g}, ${b})`;
  const hslVal = rgbToHslStr(r, g, b);

  const lum = getLuminance(r, g, b);
  const whiteLum = 1.0;
  const blackLum = 0.0;

  const contrastWithWhite = Number(((whiteLum + 0.05) / (lum + 0.05)).toFixed(2));
  const contrastWithBlack = Number(((lum + 0.05) / (blackLum + 0.05)).toFixed(2));

  return {
    hex: hexVal,
    rgb: rgbVal,
    hsl: hslVal,
    cssVar: `--color-custom: ${hexVal};`,
    tailwindBg: `bg-[${hexVal}]`,
    tailwindText: `text-[${hexVal}]`,
    r,
    g,
    b,
    contrastWithWhite,
    contrastWithBlack,
  };
}

function getLuminance(r: number, g: number, b: number): number {
  const a = [r, g, b].map((v) => {
    v /= 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  });
  return a[0] * 0.2126 + a[1] * 0.7152 + a[2] * 0.0722;
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  let r: number, g: number, b: number;
  if (s === 0) {
    r = g = b = l;
  } else {
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }
  return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
}

function hue2rgb(p: number, q: number, t: number): number {
  if (t < 0) t += 1;
  if (t > 1) t -= 1;
  if (t < 1 / 6) return p + (q - p) * 6 * t;
  if (t < 1 / 2) return q;
  if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
  return p;
}

function rgbToHslStr(r: number, g: number, b: number): string {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0, s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = (g - b) / d + (g < b ? 6 : 0); break;
      case g: h = (b - r) / d + 2; break;
      case b: h = (r - g) / d + 4; break;
    }
    h /= 6;
  }

  return `hsl(${Math.round(h * 360)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}
