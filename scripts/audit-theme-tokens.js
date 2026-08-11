import assert from 'node:assert/strict';
import fs from 'node:fs';

const source = fs.readFileSync('src/styles/foundation.css', 'utf8');

const extractBlock = (selector, fromIndex = 0) => {
  const selectorIndex = source.indexOf(selector, fromIndex);
  assert.notEqual(selectorIndex, -1, `Missing theme selector: ${selector}`);
  const open = source.indexOf('{', selectorIndex);
  let depth = 1;
  for (let index = open + 1; index < source.length; index += 1) {
    if (source[index] === '{') depth += 1;
    if (source[index] === '}') depth -= 1;
    if (depth === 0) return source.slice(open + 1, index);
  }
  throw new Error(`Unclosed theme selector: ${selector}`);
};

const tokensIn = (block) => Object.fromEntries(
  [...block.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)]
    .map((match) => [match[1], match[2].trim()]),
);

const commonRootIndex = source.lastIndexOf('  :root {');
const common = tokensIn(extractBlock('  :root {', commonRootIndex));
const dark = tokensIn(extractBlock(':root, html.dark'));
const cool = tokensIn(extractBlock('html:is(.cool, .warm, .theme-2894, .theme-sauced)'));
const themeOverrides = {
  Dark: dark,
  Cool: cool,
  Warm: tokensIn(extractBlock('html.warm')),
  '2894': tokensIn(extractBlock('html.theme-2894')),
  Sauced: tokensIn(extractBlock('html.theme-sauced')),
  Vampire: tokensIn(extractBlock('html.vampire')),
  Flux: tokensIn(extractBlock('html.flux')),
  '808': tokensIn(extractBlock('html.theme-808')),
};

const clamp = (value) => Math.max(0, Math.min(255, value));
const parseHex = (value) => {
  const expanded = value.length === 4
    ? `#${value.slice(1).split('').map((part) => part.repeat(2)).join('')}`
    : value;
  const numeric = Number.parseInt(expanded.slice(1), 16);
  return { r: (numeric >> 16) & 255, g: (numeric >> 8) & 255, b: numeric & 255 };
};
const mix = (foreground, background, weight) => ({
  r: clamp(foreground.r * weight + background.r * (1 - weight)),
  g: clamp(foreground.g * weight + background.g * (1 - weight)),
  b: clamp(foreground.b * weight + background.b * (1 - weight)),
});

const resolveColor = (value, tokens, seen = new Set()) => {
  const trimmed = value.trim();
  if (/^#[0-9a-f]{3}(?:[0-9a-f]{3})?$/i.test(trimmed)) return parseHex(trimmed);
  const variable = trimmed.match(/^var\((--[a-z0-9-]+)\)$/i);
  if (variable) {
    assert(!seen.has(variable[1]), `Circular theme token: ${variable[1]}`);
    assert(tokens[variable[1]], `Undefined theme token: ${variable[1]}`);
    return resolveColor(tokens[variable[1]], tokens, new Set([...seen, variable[1]]));
  }
  const colorMix = trimmed.match(/^color-mix\(in srgb,\s*(.+?)\s+(\d+(?:\.\d+)?)%,\s*(.+)\)$/i);
  if (colorMix) {
    return mix(
      resolveColor(colorMix[1], tokens, seen),
      resolveColor(colorMix[3], tokens, seen),
      Number(colorMix[2]) / 100,
    );
  }
  throw new Error(`Unsupported contrast color: ${trimmed}`);
};

const luminance = ({ r, g, b }) => {
  const linear = [r, g, b].map((component) => {
    const value = component / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
};
const contrast = (foreground, background) => {
  const values = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (values[0] + 0.05) / (values[1] + 0.05);
};

const checks = [
  ['Primary text on app', '--text-main', '--bg-app', 4.5],
  ['Muted text on surface', '--text-muted', '--bg-surface', 4.5],
  ['Title on selected navigation', '--text-title', '--bg-nav-selected', 4.5],
  ['Title on navigation hover', '--text-title', '--bg-nav-hover', 4.5],
  ['Primary button label', '--accent-primary-contrast', '--accent-primary', 4.5],
  ['Primary button hover label', '--accent-primary-contrast', 'color-mix(in srgb, var(--accent-primary) 94%, var(--text-title))', 4.5],
  ['Selected content', '--text-selected', '--bg-card-selected', 4.5],
  ['Code content', '--code-text', '--code-surface', 4.5],
  ['Success icon on sidebar', '--status-success', '--bg-sidebar', 3],
  ['Note icon on card', '--note-accent', '--bg-card', 3],
];

let failures = 0;
console.log('\nLive theme-token contrast audit');
console.log('-------------------------------');
for (const [name, overrides] of Object.entries(themeOverrides)) {
  const tokens = { ...common, ...(['Warm', '2894', 'Sauced'].includes(name) ? cool : {}), ...overrides };
  for (const [label, foregroundToken, backgroundToken, minimum] of checks) {
    const tokenColor = (tokenOrColor) => tokenOrColor.startsWith('--')
      ? tokens[tokenOrColor]
      : tokenOrColor;
    const ratio = contrast(
      resolveColor(tokenColor(foregroundToken), tokens),
      resolveColor(tokenColor(backgroundToken), tokens),
    );
    const passed = ratio >= minimum;
    console.log(`${passed ? 'PASS' : 'FAIL'} [${name}] ${label}: ${ratio.toFixed(2)}:1`);
    if (!passed) failures += 1;
  }
}

assert.equal(failures, 0, `${failures} live theme-token contrast checks failed.`);
console.log('Live theme-token contrast audit passed.');
