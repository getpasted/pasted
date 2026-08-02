/**
 * WCAG 2.1 Level AA Automated Contrast Audit Tool for Pasted
 * Verifies color contrast ratios for Dark & Light Mode themes.
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

// Test Palette Tokens for Dark & Light Schemes
const testPairs = [
  // LIGHT SCHEME PAIRS
  { name: 'Light Mode - Copy Clip Button Text vs BG', fg: '#ffffff', bg: '#0066cc', minRatio: 4.5, scheme: 'Light' },
  { name: 'Light Mode - Pasteboard Bar Text vs BG', fg: '#1c1c1e', bg: '#e8e8ed', minRatio: 4.5, scheme: 'Light' },
  { name: 'Light Mode - Add Note Button Text vs BG', fg: '#713f12', bg: '#fef08a', minRatio: 4.5, scheme: 'Light' },
  { name: 'Light Mode - Notes Header Text vs BG', fg: '#b45309', bg: '#fefce8', minRatio: 4.5, scheme: 'Light' },
  { name: 'Light Mode - Note Item Text vs Row BG', fg: '#451a03', bg: '#ffffff', minRatio: 4.5, scheme: 'Light' },
  { name: 'Light Mode - Note Item Text vs Row Hover BG', fg: '#451a03', bg: '#fef9c3', minRatio: 4.5, scheme: 'Light' },
  { name: 'Light Mode - Note Action Icons vs Row BG', fg: '#b45309', bg: '#ffffff', minRatio: 3.0, scheme: 'Light' },
  { name: 'Light Mode - Pin Icon vs Sidebar BG', fg: '#c2410c', bg: '#f3f3f6', minRatio: 3.0, scheme: 'Light' },

  // DARK SCHEME PAIRS
  { name: 'Dark Mode - Copy Clip Button Text vs BG', fg: '#000000', bg: '#ffffff', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Pasteboard Bar Text vs BG', fg: '#d1d5db', bg: '#171717', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Add Note Button Text vs BG', fg: '#fcd34d', bg: '#451a03', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Notes Header Text vs BG', fg: '#fbbf24', bg: '#171510', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Note Item Text vs Row BG', fg: '#fef3c7', bg: '#171510', minRatio: 4.5, scheme: 'Dark' },
  { name: 'Dark Mode - Note Action Icons vs Row BG', fg: '#fbbf24', bg: '#171510', minRatio: 3.0, scheme: 'Dark' },
  { name: 'Dark Mode - Pin Icon vs Sidebar BG', fg: '#f97316', bg: '#212121', minRatio: 3.0, scheme: 'Dark' },
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
