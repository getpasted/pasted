/**
 * WCAG 2.1 Level AA Automated Contrast Audit Tool for Pasted
 * Verifies color contrast ratios for Dark, Cool, Warm, Vampire, Flux, and 808 themes.
 */

function hexToRgb(hex) {
  hex = hex.replace(/^#/, '');
  if (hex.length === 3) {
    hex = hex.split('').map((c) => c + c).join('');
  }
  const num = parseInt(hex, 16);
  return {
    r: (num >> 16) & 255,
    g: (num >> 8) & 255,
    b: num & 255,
  };
}

function getLuminance({ r, g, b }) {
  const [rs, gs, bs] = [r, g, b].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

function getContrastRatio(hex1, hex2) {
  const lum1 = getLuminance(hexToRgb(hex1));
  const lum2 = getLuminance(hexToRgb(hex2));
  const lighter = Math.max(lum1, lum2);
  const darker = Math.min(lum1, lum2);
  return (lighter + 0.05) / (darker + 0.05);
}

// Test palette tokens for all application schemes.
const testPairs = [
  // COOL SCHEME PAIRS
  { name: 'Cool - Copy Clip Button Text vs BG', fg: '#ffffff', bg: '#0066cc', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Bin Bar Text vs BG', fg: '#1c1c1e', bg: '#e8e8ed', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Add Note Button Text vs BG', fg: '#713f12', bg: '#fef08a', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Notes Header Text vs BG', fg: '#b45309', bg: '#fefce8', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Note Item Text vs Row BG', fg: '#451a03', bg: '#ffffff', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Note Item Text vs Row Hover BG', fg: '#451a03', bg: '#fef9c3', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Note Action Icons vs Row BG', fg: '#b45309', bg: '#ffffff', minRatio: 3.0, scheme: 'Cool' },
  { name: 'Cool - Pin Icon vs Sidebar BG', fg: '#c2410c', bg: '#f3f3f6', minRatio: 3.0, scheme: 'Cool' },
  { name: 'Cool - Selected Nav Text vs BG', fg: '#1c1c1e', bg: '#ffffff', minRatio: 4.5, scheme: 'Cool' },
  { name: 'Cool - Hovered Nav Text vs BG', fg: '#1c1c1e', bg: '#d5e2f2', minRatio: 4.5, scheme: 'Cool' },

  // WARM SCHEME PAIRS
  { name: 'Warm - Primary Text vs App BG', fg: '#3b3026', bg: '#f3eee5', minRatio: 4.5, scheme: 'Warm' },
  { name: 'Warm - Muted Text vs Surface', fg: '#66584b', bg: '#fffdf8', minRatio: 4.5, scheme: 'Warm' },
  { name: 'Warm - Accent Button Text vs BG', fg: '#ffffff', bg: '#9a5b31', minRatio: 4.5, scheme: 'Warm' },
  { name: 'Warm - Selected Text vs BG', fg: '#1f170f', bg: '#e3d5c2', minRatio: 4.5, scheme: 'Warm' },
  { name: 'Warm - Selected Nav Text vs BG', fg: '#2b2118', bg: '#fffaf2', minRatio: 4.5, scheme: 'Warm' },
  { name: 'Warm - Hovered Nav Text vs BG', fg: '#2b2118', bg: '#e3d4c4', minRatio: 4.5, scheme: 'Warm' },

  // VAMPIRE SCHEME PAIRS
  { name: 'Vampire - Primary Text vs App BG', fg: '#eee5f2', bg: '#19131f', minRatio: 4.5, scheme: 'Vampire' },
  { name: 'Vampire - Muted Text vs Surface', fg: '#baabc2', bg: '#211827', minRatio: 4.5, scheme: 'Vampire' },
  { name: 'Vampire - Accent Button Text vs BG', fg: '#ffffff', bg: '#a94168', minRatio: 4.5, scheme: 'Vampire' },
  { name: 'Vampire - Selected Text vs BG', fg: '#ffffff', bg: '#403049', minRatio: 4.5, scheme: 'Vampire' },
  { name: 'Vampire - Selected Nav Text vs BG', fg: '#fff8ff', bg: '#49354f', minRatio: 4.5, scheme: 'Vampire' },

  // FLUX SCHEME PAIRS
  { name: 'Flux - Primary Text vs App BG', fg: '#dff4f6', bg: '#07151d', minRatio: 4.5, scheme: 'Flux' },
  { name: 'Flux - Muted Text vs Surface', fg: '#9abdc2', bg: '#0c222b', minRatio: 4.5, scheme: 'Flux' },
  { name: 'Flux - Accent Button Text vs BG', fg: '#061419', bg: '#45d7e5', minRatio: 4.5, scheme: 'Flux' },
  { name: 'Flux - Selected Text vs BG', fg: '#ffffff', bg: '#1a4350', minRatio: 4.5, scheme: 'Flux' },
  { name: 'Flux - Selected Nav Text vs BG', fg: '#f3fdff', bg: '#1b4a57', minRatio: 4.5, scheme: 'Flux' },

  // 808 SCHEME PAIRS
  { name: '808 - Primary Text vs App BG', fg: '#f1eadf', bg: '#151515', minRatio: 4.5, scheme: '808' },
  { name: '808 - Muted Text vs Surface', fg: '#bdb0a0', bg: '#1d1b19', minRatio: 4.5, scheme: '808' },
  { name: '808 - Accent Button Text vs BG', fg: '#211207', bg: '#ff8a24', minRatio: 4.5, scheme: '808' },
  { name: '808 - Selected Text vs BG', fg: '#ffffff', bg: '#423a31', minRatio: 4.5, scheme: '808' },
  { name: '808 - Selected Nav Text vs BG', fg: '#fffaf0', bg: '#493f35', minRatio: 4.5, scheme: '808' },
  { name: '808 - Hovered Nav Text vs BG', fg: '#fffaf0', bg: '#36291c', minRatio: 4.5, scheme: '808' },

  // DARK SCHEME PAIRS
  { name: 'Dark Mode - Copy Clip Button Text vs BG', fg: '#000000', bg: '#ffffff', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Bin Bar Text vs BG', fg: '#d1d5db', bg: '#171717', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Add Note Button Text vs BG', fg: '#fcd34d', bg: '#451a03', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Notes Header Text vs BG', fg: '#fbbf24', bg: '#171510', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Note Item Text vs Row BG', fg: '#fef3c7', bg: '#171510', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Note Action Icons vs Row BG', fg: '#fbbf24', bg: '#171510', minRatio: 3.0, scheme: 'Dark' },
  { name: 'Dark Mode - Pin Icon vs Sidebar BG', fg: '#f97316', bg: '#212121', minRatio: 3.0, scheme: 'Dark' },
  { name: 'Dark Mode - Selected Nav Text vs BG', fg: '#ffffff', bg: '#34383d', minRatio: 4.5, scheme: 'Dark' },
];

console.log('\n=======================================================');
console.log('  WCAG 2.1 LEVEL AA COLOR CONTRAST AUTOMATED AUDIT');
console.log('=======================================================\n');

let failures = 0;

testPairs.forEach((pair) => {
  const ratio = getContrastRatio(pair.fg, pair.bg);
  const passed = ratio >= pair.minRatio;
  const statusStr = passed ? '✅ PASS' : '❌ FAIL';
  const ratioStr = `${ratio.toFixed(2)}:1 (Min ${pair.minRatio}:1)`;

  console.log(`${statusStr} [${pair.scheme}] ${pair.name.padEnd(46)} -> ${ratioStr}`);

  if (!passed) failures++;
});

console.log('\n-------------------------------------------------------');
if (failures === 0) {
  console.log('✨ ALL THEME COLOR PAIRS PASSED WCAG 2.1 AA COMPLIANCE!');
  console.log('-------------------------------------------------------\n');
  process.exit(0);
} else {
  console.error(`⚠️  ${failures} CONTRAST PAIRS FAILED WCAG 2.1 AA COMPLIANCE.`);
  console.log('-------------------------------------------------------\n');
  process.exit(1);
}
