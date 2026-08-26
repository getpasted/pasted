import { getClipFilePaths, getClipOriginKind } from '../../types';
import { clipMatchesSearch, parseClipSearch } from '../../utils/clipSearch';
import { handleActivityBrowserMock } from './activity';
import { handleBackupBrowserMock } from './backup';
import { handleAnalyticsBrowserMock } from './analytics';
import { handleClipBrowserMock } from './clips';
import { handleClipVersionBrowserMock } from './clipVersions';
import { handleBinBrowserMock } from './bins';
import { handleAnalysisBrowserMock } from './analysis';
import { handleQueueBrowserMock } from './queue';
import { handleAppStateBrowserMock } from './appState';
import type { MockBin, MockClip } from './models';
import { getMockFileSearchableText } from './contentRuntime';
import { unhandledValue } from './result';
import {
  getMockConnectionCount,
  getMockSavedTransforms,
  hasMockBinTransform,
  resetMockIntelligence,
} from './intelligenceRuntime';
import {
  handleManualTransformBrowserMock,
  mockManualTransforms,
} from './manualTransforms';
import { createDefaultMockBins } from './defaultBins';
import { handleSearchHistoryBrowserMock, resetMockSearchHistory } from './searchHistory';

let mockClips: MockClip[] = [
  {
    id: 101,
    text_content: 'Sample Clip 1 for Drag Testing',
    content_type: 'text',
    source: 'Safari',
    created_at: new Date().toISOString(),
    char_count: 30,
    word_count: 6,
    line_count: 1,
    is_pinned: 0,
    is_protected: 0,
    pin_order: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
  {
    id: 102,
    text_content: 'Sample Clip 2 for Drag Testing',
    content_type: 'text',
    source: 'VS Code',
    created_at: new Date().toISOString(),
    char_count: 30,
    word_count: 6,
    line_count: 1,
    is_pinned: 0,
    is_protected: 0,
    pin_order: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
];

let mockBins: MockBin[] = createDefaultMockBins();

export const getMockClips = () => mockClips;

function withMockProtection(clip: MockClip) {
  const protectingBinIds = clip.bin_ids.filter((id) => (
    mockBins.find((bin) => bin.id === id)?.protect_clips
  ));
  const explicitlyProtected = Boolean(clip.is_explicitly_protected ?? clip.is_protected);
  const concealingBinIds = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.conceal_clips);
  const explicitlyConcealed = Boolean(clip.is_explicitly_concealed ?? clip.is_concealed);
  const explicitlyRevealed = Boolean(clip.is_explicitly_revealed);
  return {
    ...clip,
    is_explicitly_protected: explicitlyProtected,
    is_protected: explicitlyProtected || Boolean(clip.hotkey) || protectingBinIds.length > 0,
    protecting_bin_ids: protectingBinIds,
    is_concealed: !explicitlyRevealed && (explicitlyConcealed || concealingBinIds.length > 0),
    is_explicitly_concealed: explicitlyConcealed,
    is_explicitly_revealed: explicitlyRevealed,
    concealing_bin_ids: concealingBinIds,
  };
}

function assignMockClips(ids: number[], binId: number | null) {
  for (const clip of mockClips) {
    if (!ids.includes(clip.id) || clip.is_trashed !== 0) continue;
    clip.bin_id = binId;
    const tagIds = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
    clip.bin_ids = binId === null ? tagIds : [...tagIds, binId];
  }
}

function mockSmartActionSuggestions(text: string) {
  const signals: string[] = [];
  if (/https?:\/\/[^\s]+/i.test(text)) signals.push('url');
  let isJson = false;
  try {
    const trimmed = text.trim();
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      JSON.parse(trimmed);
      isJson = true;
      signals.push('json');
    }
  } catch {}
  const hasHtml = /<[a-z][^>]*>.*<\/[a-z][^>]*>|<[a-z][^>]*\/?>/is.test(text);
  if (hasHtml && !isJson) signals.push('html');
  if (/(^|\s)(#{1,6}\s|\*\*|__|```|\[[^\]]+\]\([^\)]+\))/m.test(text) && !hasHtml && !isJson) signals.push('markdown');
  const lineCount = text.length === 0 ? 0 : text.split(/\r?\n/).length - (/\r?\n$/.test(text) ? 1 : 0);
  if (lineCount > 1) signals.push('multi_line');
  if (/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/i.test(text)) signals.push('email');
  if (/(?:^|[^0-9])(?:\+?[0-9]{1,3}[-. ]?)?\(?[0-9]{3}\)?[-. ]?[0-9]{3}[-. ]?[0-9]{4}(?:$|[^0-9])/.test(text)) signals.push('phone');
  const signalPatterns: Record<string, RegExp> = {
    url: /url|link|tracking|clean_url_tracking|extract_urls/,
    json: /json|json_format|json_minify/,
    html: /html|markup|tag|strip_html|wrap_tags/,
    markdown: /markdown|strip_markdown/,
    multi_line: /line|list|sort|dedupe|sort_lines|dedupe_lines/,
    email: /email|extract_emails/,
    phone: /phone|extract_phones/,
  };
  const actions = mockManualTransforms
    .slice(0, 256)
    .flatMap((manualTransform) => {
      const searchable = `${manualTransform.name} ${manualTransform.steps.map((step) => step.operationRef).join(' ')}`.toLowerCase();
      const reasons = signals.filter((signal) => signalPatterns[signal]?.test(searchable));
      return reasons.length ? [{
        transformRef: manualTransform.stableRef,
        transformName: manualTransform.name,
        transformRevision: manualTransform.revision,
        reasons,
      }] : [];
    })
    .slice(0, 12);
  const labels: Record<string, string> = { url: 'URL Link', json: 'JSON Data', html: 'HTML Markup', markdown: 'Markdown Text', multi_line: 'Multiple Lines', email: 'Email Address', phone: 'Phone Number' };
  return { signals, signalLabels: signals.map((signal) => labels[signal]), actions };
}

export async function invokeLibraryBrowserMock<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | typeof unhandledValue> {
  for (const result of [
    handleActivityBrowserMock(cmd),
    handleBackupBrowserMock(cmd),
    handleAnalyticsBrowserMock(cmd, mockClips),
    handleClipBrowserMock(cmd, args, mockClips, withMockProtection),
    handleClipVersionBrowserMock(cmd, args, mockClips),
    handleBinBrowserMock(cmd, mockBins, mockClips),
    handleAnalysisBrowserMock(cmd),
    handleQueueBrowserMock(cmd, args),
    handleAppStateBrowserMock(cmd, args),
    handleManualTransformBrowserMock(cmd, args),
    handleSearchHistoryBrowserMock(cmd, args),
  ]) {
    if (result.matched) return result.value as T;
  }
  switch (cmd) {
    case 'search_clips': {
      const request = (args?.request ?? {}) as Record<string, unknown>;
      const query = String(request.query ?? '');
      const plan = parseClipSearch(query);
      const appendFilters = (target: string[], value: unknown) => {
        if (Array.isArray(value)) target.push(...value.filter((item): item is string => typeof item === 'string').map((item) => item.toLowerCase()));
      };
      appendFilters(plan.clipTypes, request.clipTypes);
      appendFilters(plan.contentTypes, request.contentTypes);
      appendFilters(plan.formats, request.fileFormats);
      appendFilters(plan.sources, request.sources);
      if (Array.isArray(request.clipIds)) {
        plan.clipIds.push(...request.clipIds.filter((id): id is number => Number.isInteger(id) && Number(id) > 0));
      }
      if (request.trash === true) plan.requiresTrashed = true;
      const offset = Math.max(0, Number(request.offset ?? 0));
      const limit = Math.min(500, Math.max(1, Number(request.limit ?? 100)));
      const items = mockClips.filter((clip) => {
        if (Boolean(clip.is_trashed) !== plan.requiresTrashed) return false;
        const searchableText = getMockFileSearchableText(clip.id);
        const protectedClip = withMockProtection(clip);
        const candidate = searchableText
          ? { ...protectedClip, text_content: `${clip.text_content}\n${searchableText}` }
          : protectedClip;
        return clipMatchesSearch(candidate as unknown as import('../../types').ClipItem, plan);
      });
      return {
        items: items.slice(offset, offset + limit).map((clip) => ({
          ...withMockProtection(clip),
          content_types: [...(clip.content_types ?? [])],
          file_formats: [...(clip.file_formats ?? [])],
          bin_ids: [...clip.bin_ids],
        })),
        totalCount: items.length,
        limit,
        offset,
      } as unknown as T;
    }
    case 'factory_reset_app': {
      const report = {
        clipsDeleted: mockClips.length,
        binsDeleted: mockBins.length,
        transformsDeleted: getMockSavedTransforms().length,
        connectionsDeleted: getMockConnectionCount(),
        activityEntriesDeleted: 0,
      };
      mockClips = [];
      mockBins = createDefaultMockBins();
      resetMockSearchHistory();
      resetMockIntelligence();
      return report as unknown as T;
    }
    case 'get_clip_image':
      return null as unknown as T;
    case 'analyze_content': {
      const request = (args?.request ?? {}) as Record<string, unknown>;
      const hasText = typeof request.text === 'string';
      const hasClipId = request.clipId !== undefined;
      if (hasText === hasClipId) throw new Error('Provide exactly one of text or clipId');
      const clip = hasClipId
        ? mockClips.find((item) => item.id === Number(request.clipId))
        : undefined;
      if (hasClipId && !clip) throw new Error('Clip not found');
      const text = hasText ? String(request.text) : String(clip?.text_content ?? '');
      const clipKind = clip?.content_type === 'file' || clip?.content_type === 'image'
        ? clip.content_type
        : 'text';
      const paths = clip ? getClipFilePaths(clip) : [];
      const structure = {
        origin: clip ? getClipOriginKind(clip) : 'command_line',
        byteCount: new TextEncoder().encode(clipKind === 'file' ? paths.join('') : text).length,
        ...(clipKind === 'file'
          ? { files: { itemCount: paths.length, extensions: [] } }
          : clipKind === 'image'
            ? { image: { width: 0, height: 0 } }
            : { text: {
            characterCount: Array.from(text).length,
            wordCount: text.split(/\p{White_Space}+/u).filter(Boolean).length,
            lineCount: text.length === 0 ? 0 : text.split(/\r?\n/).length - (/\r?\n$/.test(text) ? 1 : 0),
          } }),
      };
      const includeSuggestions = request.includeSuggestions !== false
        && (request.policy === undefined || request.policy === 'interactive')
        && clipKind !== 'file'
        && clipKind !== 'image';
      const includeClassifiers = request.includeClassifiers !== false
        && clipKind !== 'file'
        && clipKind !== 'image';
      const suggestions = includeSuggestions ? mockSmartActionSuggestions(text) : null;
      const participants = [
        { stableRef: 'inspector:structure-v1', pass: 'inspect', outcome: 'produced' },
        ...(includeClassifiers ? [{ stableRef: 'analysis:content-classifiers', pass: 'classify', outcome: 'produced' }] : []),
        ...(suggestions ? [{ stableRef: 'suggestion:smart-actions-v1', pass: 'suggest', outcome: 'produced' }] : []),
      ];
      return {
        formatVersion: 1,
        policy: request.policy ?? 'interactive',
        through: request.policy && request.policy !== 'interactive' ? 'classify' : 'suggest',
        result: {
          clipKind,
          structure,
          ...(includeClassifiers ? { classificationMatches: [] } : {}),
          searchableTextAvailable: false,
          ...(suggestions ? { suggestions } : {}),
        },
        participants,
        appliedClipId: null,
        ...(clipKind === 'file' ? { liveFileObservations: { availableCount: 0, fileCount: 0, directoryCount: 0, totalSizeBytes: 0 } } : {}),
      } as unknown as T;
    }
    case 'get_file_clip_previews':
      return [] as unknown as T;
    case 'update_clip_note': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip && clip.is_trashed === 0) clip.note = typeof args?.note === 'string' ? args.note : null;
      return null as unknown as T;
    }
    case 'delete_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip && !clip.is_protected) {
        clip.is_trashed = 1;
        clip.bin_id = null;
        clip.bin_ids = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
      }
      return null as unknown as T;
    }
    case 'batch_trash_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      for (const clip of mockClips) {
        if (ids.includes(clip.id) && !clip.is_protected) {
          clip.is_trashed = 1;
          clip.bin_id = null;
          clip.bin_ids = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
        }
      }
      return null as unknown as T;
    }
    case 'restore_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip) clip.is_trashed = 0;
      return null as unknown as T;
    }
    case 'restore_all_trashed_clips': {
      const restoredIds = mockClips.filter((clip) => clip.is_trashed !== 0).map((clip) => clip.id);
      for (const clip of mockClips) {
        if (clip.is_trashed !== 0) {
          clip.is_trashed = 0;
          clip.trashed_at = null;
        }
      }
      return {
        action: 'restore_all',
        requestedCount: restoredIds.length,
        changedCount: restoredIds.length,
        skippedCount: 0,
        clipIds: restoredIds,
      } as unknown as T;
    }
    case 'purge_clip_permanently':
      mockClips = mockClips.filter((clip) => clip.id !== Number(args?.id) || clip.is_protected);
      return null as unknown as T;
    case 'empty_trash':
      mockClips = mockClips.filter((clip) => clip.is_trashed === 0 || clip.is_protected);
      return null as unknown as T;
    case 'assign_clip_bin': {
      const clipId = Number(args?.clipId);
      const binId = args?.binId === null ? null : Number(args?.binId);
      if (Number.isInteger(clipId) && (binId === null || Number.isInteger(binId))) {
        assignMockClips([clipId], binId);
      }
      const transformed = binId !== null && hasMockBinTransform(binId)
        ? mockClips.find((clip) => clip.id === clipId) ?? null
        : null;
      return (transformed ? { ...transformed, is_transformed: true } : null) as unknown as T;
    }
    case 'batch_assign_bin_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const binId = args?.binId === null ? null : Number(args?.binId);
      if (binId === null || Number.isInteger(binId)) assignMockClips(ids, binId);
      return true as unknown as T;
    }
    case 'create_bin': {
      const id = Math.max(0, ...mockBins.map((bin) => bin.id)) + 1;
      const created = {
        id,
        name: typeof args?.name === 'string' ? args.name : 'Untitled Bin',
        icon: typeof args?.icon === 'string' ? args.icon : '📂',
        color: typeof args?.color === 'string' ? args.color : 'default',
        smart_rule: typeof args?.smartRule === 'string' ? args.smartRule : null,
        bin_type: 'category',
        protect_clips: false,
        conceal_clips: false,
      };
      mockBins.push(created);
      return created as unknown as T;
    }
    case 'update_bin': {
      const bin = mockBins.find((item) => item.id === Number(args?.id));
      if (bin) {
        if (typeof args?.name === 'string') bin.name = args.name;
        if (typeof args?.icon === 'string') bin.icon = args.icon;
        if (typeof args?.color === 'string') bin.color = args.color;
        bin.smart_rule = typeof args?.smartRule === 'string' ? args.smartRule : null;
      }
      return null as unknown as T;
    }
    case 'update_bin_protection': {
      const bin = mockBins.find((item) => item.id === Number(args?.id));
      if (bin && !bin.smart_rule) bin.protect_clips = Boolean(args?.protectClips);
      return null as unknown as T;
    }
    case 'update_bin_concealment': {
      const bin = mockBins.find((item) => item.id === Number(args?.id));
      if (bin && !bin.smart_rule) bin.conceal_clips = Boolean(args?.concealClips);
      return null as unknown as T;
    }
    case 'get_clip_hotkey_assignments':
      return mockClips
        .filter((clip) => Boolean(clip.hotkey))
        .map((clip) => ({ clipId: clip.id, hotkey: clip.hotkey })) as unknown as T;
    case 'update_clip_hotkey': {
      const clip = mockClips.find((item) => item.id === Number(args?.clipId));
      if (clip) {
        clip.hotkey = typeof args?.hotkey === 'string' && args.hotkey.trim()
          ? args.hotkey.trim()
          : null;
        if (clip.hotkey) {
          clip.is_protected = 1;
          clip.is_explicitly_protected = true;
        }
      }
      return clip as unknown as T;
    }
    case 'delete_bin': {
      const id = Number(args?.id);
      const disposition = typeof args?.disposition === 'string' ? args.disposition : 'keep';
      const destinationBinId = Number(args?.destinationBinId);
      mockBins = mockBins.filter((bin) => bin.id !== id);
      for (const clip of mockClips) {
        const belongsToBin = clip.bin_id === id || clip.bin_ids.includes(id);
        if (!belongsToBin) continue;
        clip.bin_ids = clip.bin_ids.filter((binId) => binId !== id);
        if (disposition === 'move' && Number.isFinite(destinationBinId)) {
          clip.bin_ids = clip.bin_ids.filter((binId) => {
            const candidate = mockBins.find((bin) => bin.id === binId);
            return candidate?.bin_type === 'tag';
          });
          clip.bin_ids.push(destinationBinId);
          clip.bin_id = destinationBinId;
        } else if (disposition === 'trash' && !clip.is_protected) {
          clip.bin_ids = [];
          clip.bin_id = null;
          clip.is_trashed = 1;
          clip.trashed_at = new Date().toISOString();
        } else if (clip.bin_id === id) {
          clip.bin_id = null;
        }
      }
      return null as unknown as T;
    }
    case 'toggle_pin_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip) {
        const nextPinned = clip.is_pinned === 0;
        if (nextPinned) {
          for (const item of mockClips) {
            if (item.is_pinned) item.pin_order += 1;
          }
        }
        clip.is_pinned = nextPinned ? 1 : 0;
        clip.pin_order = 0;
      }
      return Boolean(clip?.is_pinned) as unknown as T;
    }
    case 'batch_pin_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const pinState = Boolean(args?.pinState);
      const changedIds = [...new Set(ids)].filter((id) => {
        const clip = mockClips.find((item) => item.id === id);
        return clip && Boolean(clip.is_pinned) !== pinState;
      });
      if (pinState && changedIds.length > 0) {
        for (const clip of mockClips) {
          if (clip.is_pinned) clip.pin_order += changedIds.length;
        }
      }
      changedIds.forEach((id, index) => {
        const clip = mockClips.find((item) => item.id === id);
        if (clip) {
          clip.is_pinned = pinState ? 1 : 0;
          clip.pin_order = pinState ? index : 0;
        }
      });
      return {
        action: pinState ? 'pin' : 'unpin',
        requestedCount: ids.length,
        changedCount: changedIds.length,
        skippedCount: ids.length - changedIds.length,
        clipIds: changedIds,
      } as unknown as T;
    }
    case 'reorder_pinned_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      ids.forEach((id, index) => {
        const clip = mockClips.find((item) => item.id === id);
        if (clip?.is_pinned) clip.pin_order = index;
      });
      return null as unknown as T;
    }
    case 'reorder_bin_clips': {
      const binId = Number(args?.binId);
      const clipIds = Array.isArray(args?.clipIds) ? args.clipIds.map(Number) : [];
      const bin = mockBins.find((item) => item.id === binId);
      if (bin) bin.clip_order = clipIds;
      return null as unknown as T;
    }
    case 'toggle_clip_protected': {
      const clip = mockClips.find((item) => item.id === Number(args?.clipId));
      if (clip) clip.is_protected = clip.is_protected ? 0 : 1;
      return null as unknown as T;
    }
    case 'batch_protect_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      for (const clip of mockClips) {
        if (ids.includes(clip.id)) clip.is_protected = args?.protectedState ? 1 : 0;
      }
      return {
        action: args?.protectedState ? 'protect' : 'unprotect',
        requestedCount: ids.length,
        changedCount: ids.length,
        skippedCount: 0,
        clipIds: ids,
      } as unknown as T;
    }
    case 'toggle_clip_concealed': {
      const clip = mockClips.find((item) => item.id === Number(args?.clipId));
      if (clip && clip.is_trashed === 0) {
        const concealed = !withMockProtection(clip).is_concealed;
        clip.is_concealed = concealed ? 1 : 0;
        clip.is_explicitly_concealed = concealed;
        clip.is_explicitly_revealed = !concealed;
      }
      return Boolean(clip?.is_concealed) as unknown as T;
    }
    case 'batch_conceal_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      mockClips.forEach((clip) => {
        if (ids.includes(clip.id) && clip.is_trashed === 0) {
          const concealed = Boolean(args?.concealedState);
          clip.is_concealed = concealed ? 1 : 0;
          clip.is_explicitly_concealed = concealed;
          clip.is_explicitly_revealed = !concealed;
        }
      });
      return null as unknown as T;
    }
    case 'remove_clip_bin': {
      const clip = mockClips.find(({ id }) => id === Number(args?.clipId));
      const binId = Number(args?.binId);
      if (!clip) throw new Error('Clip was not found');
      clip.bin_ids = clip.bin_ids.filter((id) => id !== binId);
      if (clip.bin_id === binId) clip.bin_id = null;
      return {
        mutation: { action: 'remove_bin', requestedCount: 1, changedCount: 1, skippedCount: 0, clipIds: [clip.id] },
        updatedClips: [{ ...clip }],
      } as unknown as T;
    }
    case 'trash_unpinned_clips':
      mockClips.forEach((clip) => {
        if (!clip.is_pinned && !clip.is_protected) clip.is_trashed = 1;
      });
      return undefined as unknown as T;
    case 'purge_unpinned_clips':
      mockClips = mockClips.filter((clip) => clip.is_pinned || clip.is_protected);
      return undefined as unknown as T;
    case 'update_bin_hotkey': {
      const bin = mockBins.find(({ id }) => id === Number(args?.id));
      if (bin) bin.hotkey = typeof args?.hotkey === 'string' ? args.hotkey : null;
      return undefined as unknown as T;
    }
    case 'export_clips_json':
      return JSON.stringify(mockClips) as unknown as T;
    case 'export_clips_csv':
      return 'id,content_type,text_content,source,created_at\n' as unknown as T;
    case 'transform_text': {
      const input = String(args?.input ?? '');
      const filterType = String(args?.filterType ?? '');
      if (filterType === 'uppercase') return input.toUpperCase() as unknown as T;
      if (filterType === 'lowercase') return input.toLowerCase() as unknown as T;
      if (filterType === 'trim') return input.trim() as unknown as T;
      if (filterType === 'regex') {
        const config = JSON.parse(String(args?.config ?? '{}')) as { pattern?: string; replacement?: string };
        return input.replace(new RegExp(config.pattern ?? '', 'g'), config.replacement ?? '') as unknown as T;
      }
      return input as unknown as T;
    }
    default:
      return unhandledValue;
  }
}
