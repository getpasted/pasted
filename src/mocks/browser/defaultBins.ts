import type { MockBin } from './models';

const DEFAULT_BROWSER_SMART_RULE = JSON.stringify({
  version: 1,
  conditions: [
    { type: 'source', operator: 'contains', value: 'Safari' },
    { type: 'source', operator: 'contains', value: 'Chrome' },
    { type: 'source', operator: 'contains', value: 'Chromium' },
    { type: 'source', operator: 'contains', value: 'Firefox' },
    { type: 'source', operator: 'contains', value: 'Edge' },
    { type: 'source', operator: 'is', value: 'Arc' },
    { type: 'source', operator: 'contains', value: 'Brave' },
    { type: 'source', operator: 'contains', value: 'Vivaldi' },
    { type: 'source', operator: 'contains', value: 'Opera' },
    { type: 'source', operator: 'contains', value: 'Orion' },
    { type: 'source', operator: 'is', value: 'Zen Browser' },
  ],
  match: 'any',
});

export function createDefaultMockBins(): MockBin[] {
  return [
    { id: 1, name: 'Projects', icon: '🗂️', color: '#6b7280', smart_rule: null, bin_type: 'category' },
    { id: 2, name: 'From Browsers', icon: '🌐', color: '#6b7280', smart_rule: DEFAULT_BROWSER_SMART_RULE, bin_type: 'category' },
  ];
}
