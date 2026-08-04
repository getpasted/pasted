import assert from 'node:assert/strict';
import fs from 'node:fs';

function readFilesRecursively(directory, extensions) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) return readFilesRecursively(path, extensions);
    return extensions.some((extension) => entry.name.endsWith(extension))
      ? [fs.readFileSync(path, 'utf8')]
      : [];
  });
}

function matches(source, pattern) {
  return new Set([...source.matchAll(pattern)].map((match) => match[1]));
}

const frontendSource = readFilesRecursively('src', ['.ts', '.tsx']).join('\n');
const tauriBridge = fs.readFileSync('src/utils/tauri.ts', 'utf8');
const rustRegistration = fs.readFileSync('src-tauri/src/lib.rs', 'utf8');
const handlerBlock = rustRegistration.match(/generate_handler!\[([\s\S]*?)\]\)/)?.[1];
assert.ok(handlerBlock, 'Could not locate the Tauri generate_handler registration');

const invokedCommands = matches(
  frontendSource,
  /invoke(?:<[^;\n]*?>)?\(\s*['"]([a-zA-Z0-9_]+)['"]/g,
);
const registeredCommands = matches(handlerBlock, /commands::([a-zA-Z0-9_]+)/g);
const mockedCommands = matches(tauriBridge, /case ['"]([a-zA-Z0-9_]+)['"]:/g);
const dynamicInvocations = [...frontendSource.matchAll(/\binvoke(?:<[^;\n]*>)?\((?!\s*['"])/g)];

const unregisteredInvocations = [...invokedCommands]
  .filter((command) => !registeredCommands.has(command))
  .sort();
const staleMocks = [...mockedCommands]
  .filter((command) => !registeredCommands.has(command))
  .sort();
const unusedRegistrations = [...registeredCommands]
  .filter((command) => !invokedCommands.has(command))
  .sort();

assert.deepEqual(
  unregisteredInvocations,
  [],
  `Frontend invokes unregistered Tauri commands: ${unregisteredInvocations.join(', ')}`,
);
assert.deepEqual(
  staleMocks,
  [],
  `Browser mocks contain stale or misspelled Tauri commands: ${staleMocks.join(', ')}`,
);
assert.deepEqual(
  unusedRegistrations,
  [],
  `Tauri exposes commands with no frontend consumer: ${unusedRegistrations.join(', ')}`,
);
assert.equal(
  dynamicInvocations.length,
  0,
  'Tauri command names must be string literals so the IPC contract remains auditable',
);

console.log(
  `IPC contract audit passed (${invokedCommands.size} invoked, ${registeredCommands.size} registered, ${mockedCommands.size} mocked).`,
);
