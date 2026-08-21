import fs from 'node:fs';

const readRustFiles = (directory) => fs.readdirSync(directory, { withFileTypes: true })
  .sort((left, right) => left.name.localeCompare(right.name))
  .flatMap((entry) => {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) return readRustFiles(path);
    return entry.name.endsWith('.rs') ? [fs.readFileSync(path, 'utf8')] : [];
  });

export const readRustSourceTree = (directory) => readRustFiles(directory).join('\n');

export const readRustModuleTree = (rootFile, moduleDirectory) => [
  fs.readFileSync(rootFile, 'utf8'),
  ...readRustFiles(moduleDirectory),
].join('\n');
