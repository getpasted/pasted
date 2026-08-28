import assert from 'node:assert/strict';
import { findExistingAnnouncement, validateAnnouncement } from './publish-release-announcement.js';

const valid = {
  repository: 'getpasted/pasted',
  tag: 'v1.2.3',
  title: 'Pasted 1.2.3. Copy irresponsibly.',
  body: 'A release happened.\n\n<!-- pasted-release:v1.2.3 -->\n',
};

assert.equal(validateAnnouncement(valid), '<!-- pasted-release:v1.2.3 -->');
assert.throws(
  () => validateAnnouncement({ ...valid, tag: 'v1.2.3-rc.1' }),
  /stable release tag/,
);
assert.throws(
  () => validateAnnouncement({ ...valid, body: '<!-- pasted-release:v1.2.2 -->' }),
  /exactly one/,
);
assert.throws(
  () => validateAnnouncement({ ...valid, body: `${valid.body}${valid.body}` }),
  /exactly one/,
);

const discussion = { number: 12, url: 'https://example.test/discussions/12', body: valid.body };
assert.equal(
  findExistingAnnouncement([discussion], '<!-- pasted-release:v1.2.3 -->'),
  discussion,
);
assert.equal(findExistingAnnouncement([], '<!-- pasted-release:v1.2.3 -->'), null);
assert.throws(
  () => findExistingAnnouncement([discussion, discussion], '<!-- pasted-release:v1.2.3 -->'),
  /Multiple Discussions/,
);

console.log('Release announcement checks passed.');

