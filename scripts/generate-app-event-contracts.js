import assert from 'node:assert/strict';
import fs from 'node:fs';

const manifest = JSON.parse(fs.readFileSync('contracts/app-events.json', 'utf8'));
const rust = `${Object.entries(manifest)
  .map(([constant, event]) => `pub const ${constant}: &str = "${event.name}";`)
  .join('\n')}\n`;
const typescript = `export const APP_EVENTS = {\n${Object.values(manifest)
  .map((event) => `  ${event.typescript}: '${event.name}',`)
  .join('\n')}\n} as const;\n`;
const outputs = [
  ['src-tauri/src/app_event_names.rs', rust],
  ['src/utils/appEvents.generated.ts', typescript],
];

if (process.argv.includes('--check')) {
  for (const [path, expected] of outputs) {
    assert.equal(fs.readFileSync(path, 'utf8'), expected,
      `${path} is stale; run node scripts/generate-app-event-contracts.js`);
  }
  console.log(`Generated app-event contracts are current (${Object.keys(manifest).length} events).`);
} else {
  for (const [path, content] of outputs) fs.writeFileSync(path, content);
  console.log(`Generated ${outputs.length} app-event contract files.`);
}
