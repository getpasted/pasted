import assert from 'node:assert/strict';
import { createElement, Fragment } from 'react';
import { normalizeMenuDividers } from '../src/utils/menuChildren.ts';

const Divider = () => null;
const Item = ({ name }: { name: string }) => createElement('button', null, name);
const divider = (key: string) => createElement(Divider, { key });
const item = (name: string) => createElement(Item, { key: name, name });

const normalized = normalizeMenuDividers([
  divider('leading'),
  item('copy'),
  createElement(Fragment, null, divider('first'), false, divider('duplicate'), item('restore')),
  divider('trailing'),
], Divider);

assert.deepEqual(
  normalized.map((child) => typeof child === 'object' && child !== null && 'type' in child
    ? child.type === Divider ? 'divider' : 'item'
    : 'other'),
  ['item', 'divider', 'item'],
  'Menus must discard leading, trailing, and consecutive dividers across conditional fragments',
);

console.log('Menu child normalization tests passed.');
