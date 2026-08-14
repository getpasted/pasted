import assert from 'node:assert/strict';
import {
  MAX_RENDERABLE_RASTER_BYTES,
  base64DecodedByteLength,
  decodeSafeRasterDataUrl,
} from '../src/utils/safeRasterImage.ts';

const dataUrl = (mimeType, bytes) => `data:${mimeType};base64,${Buffer.from(bytes).toString('base64')}`;

assert.ok(decodeSafeRasterDataUrl(dataUrl('image/png', [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])));
assert.ok(decodeSafeRasterDataUrl(dataUrl('image/jpeg', [0xff, 0xd8, 0xff, 0xdb])));
assert.ok(decodeSafeRasterDataUrl(dataUrl('image/webp', Buffer.from('RIFF1234WEBP'))));

const activeSvg = Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"/>');
assert.equal(decodeSafeRasterDataUrl(dataUrl('image/svg+xml', activeSvg)), null);
assert.equal(decodeSafeRasterDataUrl(dataUrl('image/png', activeSvg)), null);
assert.equal(decodeSafeRasterDataUrl('data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg=='), null);
assert.equal(decodeSafeRasterDataUrl('javascript:alert(1)'), null);
assert.equal(decodeSafeRasterDataUrl('https://example.invalid/image.png'), null);
assert.equal(decodeSafeRasterDataUrl('data:image/png;base64,%%%'), null);
assert.equal(base64DecodedByteLength('AA=='), 1);
assert.equal(base64DecodedByteLength('AAA='), 2);
assert.equal(base64DecodedByteLength('AAAA'), 3);
assert.equal(base64DecodedByteLength('AAA'), null);
assert.equal(MAX_RENDERABLE_RASTER_BYTES, 128 * 1024 * 1024);

console.log('Safe raster image source tests passed.');
