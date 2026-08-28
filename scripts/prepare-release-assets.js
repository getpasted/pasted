#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const NOTICE_NAME = 'THIRD_PARTY_NOTICES.txt';

const walkFiles = (directory) => fs.readdirSync(directory, { withFileTypes: true })
  .sort((left, right) => left.name.localeCompare(right.name))
  .flatMap((entry) => {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      return walkFiles(entryPath);
    }
    if (!entry.isFile()) {
      throw new Error(`Release asset input must be a regular file: ${entryPath}`);
    }
    return [entryPath];
  });

const canonicalContent = (name, content) => {
  if (name !== NOTICE_NAME) {
    return content;
  }
  return Buffer.from(content.toString('utf8').replaceAll('\r\n', '\n'), 'utf8');
};

export const prepareReleaseAssets = (inputDirectory, outputDirectory) => {
  const input = path.resolve(inputDirectory);
  const output = path.resolve(outputDirectory);
  if (input === output || output.startsWith(`${input}${path.sep}`)) {
    throw new Error('Release asset output must be outside the input directory.');
  }
  if (!fs.statSync(input).isDirectory()) {
    throw new Error(`Release asset input is not a directory: ${input}`);
  }
  if (fs.existsSync(output) && fs.readdirSync(output).length > 0) {
    throw new Error(`Release asset output must be empty: ${output}`);
  }

  const assets = new Map();
  for (const source of walkFiles(input)) {
    const name = path.basename(source);
    const content = canonicalContent(name, fs.readFileSync(source));
    const existing = assets.get(name);
    if (existing) {
      if (existing.content.equals(content)) {
        continue;
      }
      throw new Error(`Conflicting release assets share the name ${name}.`);
    }
    assets.set(name, {
      content,
      mode: fs.statSync(source).mode,
    });
  }

  if (assets.size === 0) {
    throw new Error('No release assets were found.');
  }

  fs.mkdirSync(output, { recursive: true });
  for (const [name, asset] of assets) {
    const destination = path.join(output, name);
    fs.writeFileSync(destination, asset.content, { mode: asset.mode });
  }
  return [...assets.keys()].sort();
};

const isMain = process.argv[1]
  && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const [inputDirectory, outputDirectory] = process.argv.slice(2);
  if (!inputDirectory || !outputDirectory) {
    throw new Error('Usage: node scripts/prepare-release-assets.js <input-directory> <output-directory>');
  }
  const assets = prepareReleaseAssets(inputDirectory, outputDirectory);
  console.log(`Prepared ${assets.length} uniquely named release assets.`);
}
