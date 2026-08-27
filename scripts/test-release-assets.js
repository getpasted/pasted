import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { prepareReleaseAssets } from './prepare-release-assets.js';

const makeRoot = () => fs.mkdtempSync(path.join(os.tmpdir(), 'pasted-release-assets-'));
const write = (root, platform, name, content) => {
  const directory = path.join(root, 'input', platform);
  fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(path.join(directory, name), content);
};

{
  const root = makeRoot();
  write(root, 'macos', 'THIRD_PARTY_NOTICES.txt', 'Pasted notices\n\nDependency A\n');
  write(root, 'windows', 'THIRD_PARTY_NOTICES.txt', 'Pasted notices\r\n\r\nDependency A\r\n');
  write(root, 'linux', 'pasted', Buffer.from([0, 1, 2, 3]));
  write(root, 'macos', 'pasted', Buffer.from([0, 1, 2, 3]));
  write(root, 'windows', 'Pasted-setup.exe', Buffer.from([4, 5, 6]));

  const names = prepareReleaseAssets(path.join(root, 'input'), path.join(root, 'output'));
  assert.deepEqual(names, [
    'Pasted-setup.exe',
    'THIRD_PARTY_NOTICES.txt',
    'pasted',
  ]);
  assert.equal(
    fs.readFileSync(path.join(root, 'output', 'THIRD_PARTY_NOTICES.txt'), 'utf8'),
    'Pasted notices\n\nDependency A\n',
  );
}

{
  const root = makeRoot();
  write(root, 'linux', 'collision.bin', Buffer.from([1]));
  write(root, 'windows', 'collision.bin', Buffer.from([2]));
  assert.throws(
    () => prepareReleaseAssets(path.join(root, 'input'), path.join(root, 'output')),
    /Conflicting release assets share the name collision\.bin/,
  );
  assert.equal(fs.existsSync(path.join(root, 'output')), false);
}

{
  const root = makeRoot();
  write(root, 'macos', 'THIRD_PARTY_NOTICES.txt', 'Dependency A\n');
  write(root, 'windows', 'THIRD_PARTY_NOTICES.txt', 'Dependency B\r\n');
  assert.throws(
    () => prepareReleaseAssets(path.join(root, 'input'), path.join(root, 'output')),
    /Conflicting release assets share the name THIRD_PARTY_NOTICES\.txt/,
  );
}

console.log('Release asset preparation tests passed.');
