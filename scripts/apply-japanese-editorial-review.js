import fs from 'node:fs';
import assert from 'node:assert/strict';

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const path = 'src/locales/ja-JP.json';
const japanese = JSON.parse(fs.readFileSync(path, 'utf8'));

const overrides = {
  'component.analyticsView.sourceCountPercent': { one: '{count}件のクリップ（{percent}%）', other: '{count}件のクリップ（{percent}%）' },
  'component.binModal.openEmojiPickerShortcut': 'クリックして絵文字ピッカーを開きます（{shortcut}）。',
  'component.builtinLifecycleManagerDialog.captureStableReferenceUsage': '安定参照は、APIと共有ライブラリでこの機能を識別します。{command}で一覧を表示できます。',
  'component.builtinLifecycleManagerDialog.stableReferenceUsage': '安定参照は、CLIとAPIでこの{kind}を識別します。{command}で使用してください。',
  'component.clipCard.queuePosition': 'キューの{position}番目',
  'component.clipPreview.noteCount': { one: 'メモ（{count}）', other: 'メモ（{count}）' },
  'component.contentExtractorManagerDialog.customCommandProtocolDescription': '保存時に、クリップの内容を渡さずに{versionFlag}を実行し、選択した実行ファイルを確認します。抽出時は{requestCommand}を受け取り、文字列またはnullの{outputField}フィールドを含むJSONオブジェクトを標準出力に書き込みます。処理はローカルで実行され、制限時間は60秒です。',
  'component.deleteBinDialog.binContentsQuestion': { one: 'このコレクションにはクリップが{count}件あります。どうしますか？', other: 'このコレクションにはクリップが{count}件あります。どうしますか？' },
  'component.deleteBinDialog.deleteNamedBin': '「{name}」を削除しますか？',
  'component.deleteTransformationAssetDialog.removesKindFromLibrary': 'この{kind}をライブラリから削除します。すでに作成または変更されたクリップには影響しません。',
  'component.helpView.classifierRescanDescription': '分類ルールの編集は、新しいテキストクリップに反映されます。{action}を実行すると現在の分類順序が明示的に再適用され、コンテンツタイプ、スマートコレクションへの所属、機密コンテンツのマスキングが変わる場合があります。',
  'component.helpView.commandReferenceDescription': 'レコードや変更内容を返すコマンドでは、表示されている箇所で{flag}を使用できます。無効な機能に関連するコマンドは、動作を暗黙に変えるのではなくエラーになります。',
  'component.helpView.customExtractorProtocolDescription': 'カスタムコマンドは、制限付きの{protocol}プロトコルを通じて画像データやファイル参照を検索可能なテキストに変換できます。',
  'component.helpView.dismissHudShortcutDescription': '{key}を押すと、HUDまたは開いているメニューをすぐに閉じます。',
  'component.helpView.hudNumberShortcutDescription': 'HUDで{start}から{end}の数字キーを押すと、{start}から{end}番目の項目をすぐに貼り付けます。',
  'component.helpView.ocrStatusDescription': 'OCRを無効にすると、バックグラウンド処理がキャンセルされ、遅れて届いた結果は破棄されます。完了済みのテキストは保持されます。進行状況は{command}で確認できます。',
  'component.helpView.openHudShortcutDescription': '{shortcut}を押すと、ポインターの近くにコンパクトなクリップボードウィンドウが開きます。矢印キーで移動し、Returnキーで貼り付けます。',
  'component.helpView.permanentDeleteShortcutDescription': '{modifier}キーを押している間、ゴミ箱アイコンが赤い{symbol}ボタンに変わり、ゴミ箱を経由せずに項目を完全に削除できます。',
  'component.helpView.permanentDeletionDescription': '削除時にOptionまたはAltキーを押している場合、ゴミ箱を空にした場合、またはゴミ箱を無効にした場合は完全に削除されます。',
  'component.helpView.protectionDescription': '保護を解除するまで、クリップの削除と自動保持を防ぎます。',
  'component.helpView.restoreDescription': '復元すると、クリップがゴミ箱から履歴に戻ります。',
  'component.helpView.restoreTrashedClipsDescription': '設定 › 一般 › ゴミ箱の「ゴミ箱のクリップを復元」を実行すると、すべてのクリップが履歴に戻ります。',
  'component.pinnedClipShelf.morePinnedCount': 'ほか{count}件をピン留め',
  'component.sequentialQueueBar.bufferCount': { one: 'バッファ内: {count}件', other: 'バッファ内: {count}件' },
  'component.settingsGeneralPanel.unlimitedRevisionHistoryDescription': '編集、OCR、変換、復元の完全なテキストスナップショットを保持します。変換を自動実行すると、無制限の履歴は急速に増える可能性があります。',
  'component.settingsHotkeysPanel.accessibilityInstructions': '{settingsPath}で{app}を許可してください。',
  'component.settingsHotkeysPanel.developmentAccessibilityInstructions': '開発中は、{settingsPath}で使用中のIDEまたはターミナルを許可してください。',
  'component.settingsSyncPanel.exportFileSummary': { one: '{extension}ファイルを{count}件作成します。', other: '{extension}ファイルを{count}件作成します。' },
  'component.settingsSyncPanel.fullBackupFileSummary': '{extension}形式のSQLiteスナップショットを1件作成します。',
  'format.characterCount': { one: '{count}文字', other: '{count}文字' },
  'format.fileCount': { one: '{count}ファイル', other: '{count}ファイル' },
  'format.versionCount': { one: '{count}バージョン', other: '{count}バージョン' },
  'app.deleteSelectedPermanently': '選択項目を完全に削除',
  'app.deselect': '選択を解除',
  'app.emptyClip': '空のクリップ',
  'app.ignoredApp': '除外中: {name}',
  'app.imageClip': '画像クリップ',
  'app.moveSelectedToTrash': '選択項目をゴミ箱に移動',
  'app.pinSelected': '選択項目をピン留め',
  'app.resultCount': { one: '{count}件の結果', other: '{count}件の結果' },
  'app.searchResultCount': { one: '{count}件の検索結果', other: '{count}件の検索結果' },
  'app.unpinSelected': '選択項目のピン留めを解除',
  'action.addNote': 'メモを追加',
  'action.editNote': 'メモを編集',
  'action.testInPlayground': 'プレイグラウンドでテスト',
  'action.unpin': 'ピン留めを解除',
  'collection.bin': 'コレクション',
  'collection.history': '履歴',
  'collection.noted': 'メモ付き',
  'collection.pinned': 'ピン留め',
  'collection.protected': '保護済み',
  'collection.queue': 'キュー',
  'collection.search': '検索',
  'collection.trashed': 'ゴミ箱',
  'common.automatic': '自動',
  'common.back': '戻る',
  'common.cancel': 'キャンセル',
  'common.close': '閉じる',
  'common.custom': 'カスタム',
  'common.default': 'デフォルト',
  'common.delete': '削除',
  'common.description': '説明',
  'common.dismiss': '閉じる',
  'common.done': '完了',
  'common.edit': '編集',
  'common.enabled': '有効',
  'common.name': '名前',
  'common.new': '新規',
  'common.noBin': 'コレクションなし',
  'common.reset': 'リセット',
  'common.retry': '再試行',
  'common.save': '保存',
  'common.saved': '保存済み',
  'common.system': 'システム',
  'common.unknownSource': '不明なソース',
  'component.analyticsView.noExtension': '拡張子なし',
  'component.activityLogView.hudPasted': 'HUDから貼り付け',
  'component.activityLogView.queuePasted': 'キューから貼り付け',
  'component.clipTransformBar.applyAndSaveRevision': '適用して変更履歴を保存',
  'component.settingsFeaturesPanel.functionality': '機能',
  'component.settingsAnalysisPanel.resetShippedAnalysisDefinitions': '同梱の分析定義をリセットしますか？',
  'component.settingsAnalysisPanel.resetShippedClassifierDefinitions': '同梱の分類器定義をリセットしますか？',
  'component.settingsAnalysisPanel.shippedAnalysisDefaultsRestoredCustomDefinitionsWerePreserved': '同梱の分析設定を初期状態に戻しました。カスタム定義は保持されています。',
  'component.settingsAnalysisPanel.shippedContentTypesContentTypeGroupsAndClassifiersReturnToTheirDefaults': '同梱のコンテンツタイプ、コンテンツタイプグループ、分類器を初期状態に戻します。',
  'component.settingsAnalysisPanel.shippedExtractorsClassifiersContentTypesAndContentTypeGroupsReturnToTheir': '同梱の抽出ツール、分類器、コンテンツタイプ、コンテンツタイプグループを初期状態に戻します。',
  'component.settingsTabs.about': 'Pastedについて',
  'component.settingsTabs.analysis': '分析',
  'component.settingsTabs.appExclusions': 'アプリの除外',
  'component.settingsTabs.functionality': '機能',
  'component.settingsTabs.general': '一般',
  'component.settingsTabs.hotkeys': 'ホットキー',
  'component.settingsTabs.intelligence': 'インテリジェンス',
  'component.settingsTabs.notifications': '通知',
  'component.settingsTabs.security': 'セキュリティ',
  'component.settingsTabs.settingsSections': '設定セクション',
  'component.settingsTabs.storage': 'ストレージ',
  'component.sidebar.bins': 'コレクション',
  'component.sidebar.clips': 'クリップ',
  'component.sidebar.contentTypes': 'コンテンツタイプ',
  'component.sidebar.sources': 'ソース',
  'component.sidebar.tools': 'ツール',
  'destination.activity': 'アクティビティ',
  'destination.help': 'ヘルプ',
  'destination.insights': 'インサイト',
  'destination.operations': '操作',
  'destination.playground': 'プレイグラウンド',
  'destination.settings': '設定',
  'destination.transformations': '変換',
  'feature.preset.custom': 'カスタム',
  'feature.preset.full': 'すべて',
  'feature.preset.simple': 'シンプル',
  'feature.activityLog.label': 'アクティビティ',
  'feature.bins.description': 'スマートコレクションを使って、クリップを手動または自動で整理します。',
  'feature.bins.label': 'コレクション',
  'feature.cli.description': 'Pasted CLIでクリップボードのワークフローを自動化します。',
  'feature.revisions.label': '変更履歴',
  'native.clips.bins': 'コレクション',
  'native.clips.noted': 'メモ付き',
  'native.clips.protected': '保護済み',
  'native.clips.trashed': 'ゴミ箱',
  'native.clips.title': 'クリップ',
  'native.file.newBin': '新規コレクション…',
  'native.tools.playground': 'プレイグラウンド',
  'component.helpView.exportClipsCurrentlyInHistoryForExternalAnalysis': '外部分析用に、現在の履歴にあるクリップをエクスポートします。',
  'component.helpView.listABoundedPageFromHistoryTrashABinOrPinnedClips': '履歴、ゴミ箱、コレクション、またはピン留めしたクリップから、件数を制限したページを一覧表示します。',
  'component.clearHistoryDialog.moveAllUnpinnedAndUnprotectedClipboardHistoryIntoTrashPinnedClipsProtected': 'ピン留めも保護もされていないクリップボード履歴をすべてゴミ箱に移動しますか？ピン留めしたクリップ、保護したクリップ、コレクションの定義は保持されます。',
  'component.clearHistoryDialog.permanentlyDeleteAllUnpinnedAndUnprotectedClipboardHistoryPinnedClipsProtectedClips': 'ピン留めも保護もされていないクリップボード履歴をすべて完全に削除しますか？ピン留めしたクリップ、保護したクリップ、コレクションの定義は保持されます。',
  'component.helpView.rightClickAClipForQueuePinProtectNoteBinTransformAnd': 'クリップを右クリックすると、キュー、ピン留め、保護、メモ、コレクション、変換、ゴミ箱の操作を使用できます。',
  'component.settingsSyncPanel.addsTrashBinsTransformsOperationsContentTypesClassifiersAndOcr': 'ゴミ箱、コレクション、変換、操作、コンテンツタイプ、分類器、OCRを追加します。',
  'component.welcomeSetup.morePastedFeatures': 'Pastedのその他の機能',
  'format.clipCount': { one: '{count}件のクリップ', other: '{count}件のクリップ' },
  'format.dayCount': { one: '{count}日', other: '{count}日' },
  'format.entryCount': { one: '{count}件', other: '{count}件' },
  'format.fileSummaryMore': '{name}、ほか{count}件',
};

for (const [key, value] of Object.entries(overrides)) {
  assert.ok(key in english, `Unknown Japanese editorial key: ${key}`);
  japanese[key] = value;
}

for (const [key, source] of Object.entries(english)) {
  if (typeof source !== 'string' || typeof japanese[key] !== 'string') continue;
  if (/\bPlayground\b/.test(source)) japanese[key] = japanese[key].replaceAll('遊び場', 'プレイグラウンド');
  if (/\bBins?\b/.test(source)) {
    japanese[key] = japanese[key]
      .replaceAll('スマート・ビン', 'スマートコレクション')
      .replaceAll('スマートビン', 'スマートコレクション')
      .replaceAll('Bins', 'コレクション')
      .replaceAll('Bin', 'コレクション')
      .replaceAll('ビン', 'コレクション');
    if (!/\bTrash\b/.test(source)) {
      japanese[key] = japanese[key]
        .replaceAll('ゴミ箱', 'コレクション')
        .replaceAll('フォルダ', 'コレクション')
        .replaceAll('バイン', 'コレクション');
    }
  }
}

const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, japanese[key]]));
fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
console.log(`Applied ${Object.keys(overrides).length} reviewed Japanese overrides.`);
