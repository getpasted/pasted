#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
const readArgument = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};

const version = readArgument('--version');
const sha256 = readArgument('--sha256');
const output = readArgument('--output');

if (!version || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error('Pass a semantic version with --version.');
}
if (!sha256 || !/^[a-fA-F0-9]{64}$/.test(sha256)) {
  throw new Error('Pass the DMG SHA-256 digest with --sha256.');
}
if (!output) {
  throw new Error('Pass a destination with --output.');
}

const cask = `cask "pasted" do
  version "${version}"
  sha256 "${sha256.toLowerCase()}"

  url "https://github.com/pasted-app/pasted/releases/download/v#{version}/Pasted_#{version}_universal.dmg",
      verified: "github.com/pasted-app/pasted/"
  name "Pasted"
  desc "Clipboard history, organization, and transformations"
  homepage "https://github.com/pasted-app/pasted"

  app "Pasted.app"
  binary "#{appdir}/Pasted.app/Contents/MacOS/pasted"
end
`;

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, cask);
console.log(`Rendered Homebrew Cask for Pasted ${version} at ${output}.`);
