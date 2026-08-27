import assert from 'node:assert/strict';
import fs from 'node:fs';

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const path = 'src/locales/pt-BR.json';
const portuguese = JSON.parse(fs.readFileSync(path, 'utf8'));

const overrides = {
  'app.emptyTrashConfirmation': 'Limpar a lixeira?',
  'app.emptyTrashDescription': 'Exclui permanentemente todos os clipes não protegidos da lixeira. Os clipes protegidos serão mantidos.',
  'app.emptyTrashEllipsis': 'Limpar a lixeira…',
  'component.settingsSearchHistoryPanel.clearAllEllipsis': 'Limpar tudo…',
  'component.settingsGeneralPanel.chooseHowMuchSuccessfulSearchHistoryToKeep': 'Escolha quanto histórico de pesquisas bem-sucedidas manter.',
  'component.settingsGeneralPanel.keepSearchesFor': 'Manter pesquisas por',
  'component.settingsGeneralPanel.olderSearchesAreRemovedAutomatically': 'Pesquisas mais antigas são removidas automaticamente.',
  'component.settingsGeneralPanel.maximumSearchAge': 'Idade máxima das pesquisas',
  'component.settingsGeneralPanel.maximumSearches': 'Máximo de pesquisas',
  'component.settingsGeneralPanel.theOldestSearchesAreRemovedFirst': 'As pesquisas mais antigas são removidas primeiro.',
  'component.settingsGeneralPanel.maximumSearchesRetained': 'Máximo de pesquisas mantidas',
  'component.settingsGeneralPanel.bothSearchLimitsApplyUnlimitedAndForeverDisableAutomaticRemoval': 'Os dois limites se aplicam. Ilimitado e Para sempre desativam a remoção automática.',
  'component.settingsGeneralPanel.valueSearchesDefault': '{value} pesquisas (Padrão)',
  'component.welcomeBackupRestore.chooseAPastedFullBackupFile': 'Escolha um backup completo (.pastedbackup).',
  'component.welcomeBackupRestore.chooseBackup': 'Escolher backup…',
  'component.welcomeBackupRestore.pastedFullBackup': 'Backup completo',
  'component.welcomeBackupRestore.restoreTheCompleteWorkspaceFromAPastedbackupFile': 'Restaure todo o espaço de trabalho de um arquivo .pastedbackup.',
  'component.welcomeSetup.restoreAPastedFullBackupOrImportClipboardHistory': 'Restaure um backup completo ou importe o histórico da área de transferência de praticamente qualquer lugar.',
  'component.helpView.activityKeepsAPrivacySafeLocalAuditTrailInsightsSummarizesTheActive': 'Atividade mantém uma trilha de auditoria local com privacidade; Insights resume a biblioteca ativa sem telemetria.',
  'component.helpView.organizeClipsAcrossSeveralManualBinsAndUseSearchVisualLabelsInsights': 'Organize clipes em várias coleções manuais e use Pesquisa, Rótulos Visuais, Insights e Atividade para encontrá-los e entendê-los.',
  'component.helpView.pageResetActionsPreviewExactSettingChangesAndDoNotResetClips': 'As redefinições de cada página mostram as mudanças exatas e não redefinem clipes nem páginas não relacionadas. A redefinição de fábrica continua sendo uma ação destrutiva separada.',
  'component.helpView.privateBrowserExclusionBlocksCaptureFromSupportedPrivateOrIncognitoWindowsWhen': 'A exclusão de navegador privado bloqueia janelas privadas ou anônimas detectadas. Quando a detecção não está disponível, escolha entre continuar capturando ou excluir o navegador inteiro.',
  'component.helpView.ocrStatusDescription': 'Desativar OCR cancela o trabalho em segundo plano e descarta resultados atrasados, preservando o texto concluído. Selecione uma contagem de status diferente de zero para abrir os Clips correspondentes na Busca ou inspecione o progresso com {command}.',
  'component.helpView.visualLabels': 'Rótulos visuais',
  'component.helpView.visualLabelsDescription': 'O Apple Vision Labels no macOS e os Rótulos do llama.cpp opcionais em outras plataformas encontram assuntos e objetos pesquisáveis sem substituir a imagem original.',
  'component.helpView.visualLabelFilteringDescription': 'Os rótulos detectados podem ser editados no Inspetor do Clip. Os Extratores de rótulos podem aplicar o pós-processamento compartilhado de confiança mínima antes que os rótulos aceitos se tornem pesquisáveis.',
  'app.emptyClip': 'Clipe vazio',
  'app.imageClip': 'Clipe de imagem',
  'app.pinSelected': 'Fixar seleção',
  'app.unpinSelected': 'Desafixar seleção',
  'collection.bin': 'Coleção',
  'collection.history': 'Histórico',
  'collection.noted': 'Com notas',
  'collection.pinned': 'Fixados',
  'collection.protected': 'Protegidos',
  'collection.queue': 'Fila',
  'collection.search': 'Pesquisar',
  'collection.searchActiveAndTrashedClips': 'Pesquisar clipes ativos e movidos para a Lixeira.',
  'collection.thisBin': 'esta coleção',
  'collection.thisBinIsEmpty': 'Esta coleção está vazia',
  'collection.trashed': 'Na Lixeira',
  'common.back': 'Voltar',
  'common.dismiss': 'Fechar',
  'common.done': 'Concluído',
  'common.duplicate': 'Duplicar',
  'common.noBin': 'Sem coleção',
  'common.reset': 'Redefinir',
  'common.resetToDefault': 'Redefinir para o padrão',
  'common.saved': 'Salvo',
  'component.analyticsView.sourceCountPercent': { one: '{count} clipe ({percent}%)', other: '{count} clipes ({percent}%)' },
  'collection.clipsMovedToTrash': 'Os clipes movidos para a Lixeira permanecerão aqui até ela ser esvaziada.',
  'component.clearHistoryDialog.moveAllUnpinnedAndUnprotectedClipboardHistoryIntoTrashPinnedClipsProtected': 'Mover para a Lixeira todo o histórico da área de transferência que não esteja fixado nem protegido? Os clipes fixados, os clipes protegidos e as definições das coleções serão preservados.',
  'component.clearHistoryDialog.moveClipboardHistoryToTrash': 'Mover o histórico da área de transferência para a Lixeira?',
  'component.clearHistoryDialog.permanentlyDeleteAllUnpinnedAndUnprotectedClipboardHistoryPinnedClipsProtectedClips': 'Excluir permanentemente todo o histórico da área de transferência que não esteja fixado nem protegido? Os clipes fixados, os clipes protegidos e as definições das coleções serão preservados.',
  'component.deleteBinDialog.binContentsQuestion': { one: 'Esta coleção contém {count} clipe. O que deve acontecer com ele?', other: 'Esta coleção contém {count} clipes. O que deve acontecer com eles?' },
  'component.deleteBinDialog.clipsMatchedByThisSmartBinWillBePreserved': 'Os clipes correspondentes a esta coleção inteligente serão preservados.',
  'component.deleteBinDialog.deleteBin': 'Excluir coleção',
  'component.deleteBinDialog.deleteBin2': 'Excluir coleção “',
  'component.deleteBinDialog.deleteNamedBin': 'Excluir “{name}”?',
  'component.deleteBinDialog.protectedClipsWillBeKeptInNoBin': 'Os clipes protegidos serão mantidos sem coleção.',
  'component.deleteBinDialog.thisBinContains': 'Esta coleção contém',
  'component.deleteBinDialog.thisBinIsEmptyNoClipsWillBeAffected': 'Esta coleção está vazia. Nenhum clipe será afetado.',
  'component.deleteBinDialog.trash': 'Lixeira',
  'component.clipPreview.copyClip': 'Copiar clipe',
  'component.clipPreview.loadingClipVersionCount': 'Carregando a contagem de versões do clipe',
  'component.clipPreview.openClipWorkflow': 'Abrir fluxo de trabalho do clipe',
  'component.clipPreview.viewCountClipVersions': 'Ver {count} versões do clipe',
  'component.factoryResetDialog.thisPermanentlyDeletesClipsBinsTransformsConnectionsActivityHistoryAndPreferencesFull': 'Isso exclui permanentemente clipes, coleções, transformações, conexões, histórico de atividades e preferências. Os arquivos de Backup completo e os arquivos originais referenciados pelos clipes não são excluídos.',
  'component.helpView.assignClipsToOneManualBinOrRemoveTheirManualBin': 'Atribuir clipes a uma coleção manual ou removê-los dela.',
  'component.helpView.classifierRescanDescription': 'Editar um classificador afeta novos clipes de texto. {action} reaplica explicitamente a ordem atual dos classificadores e pode alterar os tipos de conteúdo, a participação em coleções inteligentes e o mascaramento de conteúdo sensível.',
  'component.helpView.listABoundedPageFromHistoryTrashABinOrPinnedClips': 'Listar uma página limitada do Histórico, da Lixeira, de uma coleção ou dos clipes fixados.',
  'component.helpView.rightClickAClipForQueuePinProtectNoteBinTransformAnd': 'Clicar com o botão direito em um clipe para acessar Fila, Fixar, Proteger, Nota, Coleção, Transformação e Lixeira.',
  'component.helpView.useSettingsStorageToCreateAFullBackupBeforeMajorChangesOr': 'Use Configurações → Armazenamento para criar um Backup completo antes de grandes alterações ou da Redefinição de fábrica. A Restauração completa valida o backup e preserva o estado substituído como um backup de recuperação antes da ativação.',
  'component.settingsSyncPanel.addsTrashBinsTransformsOperationsContentTypesClassifiersAndOcr': 'Adiciona a Lixeira, as coleções, as transformações, as operações, os tipos de conteúdo, os classificadores e o OCR.',
  'component.settingsSyncPanel.exportFileSummary': { one: '{count} arquivo {extension} será criado.', other: '{count} arquivos {extension} serão criados.' },
  'component.settingsSyncPanel.fullBackupFileSummary': 'Será criado 1 snapshot do SQLite {extension}.',
  'component.settingsSyncPanel.historyAndOrganizationMergedProcessedCountClips': 'Histórico e organização combinados. Foram processados {count} clipes.',
  'component.settingsAnalysisPanel.cardChecksum': 'Soma de verificação do cartão',
  'component.settingsGeneralPanel.alwaysShowDockAndMenuBar': 'Sempre mostrar o Dock e a barra de menus',
  'component.settingsGeneralPanel.autoHideDockIcon': 'Ocultar automaticamente o ícone do Dock',
  'component.settingsGeneralPanel.dockAndMenuBarIcon': 'Ícone do Dock e da barra de menus',
  'component.settingsGeneralPanel.dockAndMenuBarIconBehavior': 'Comportamento do ícone do Dock e da barra de menus',
  'component.helpView.rescanClips': 'Reanalisar clipes',
  'component.settingsSyncPanel.valueClipsValue2BinsValue3TransformsValue4Operations': '{value} clipes · {value2} coleções · {value3} transformações · {value4} operações',
  'component.sidebar.alreadyInThisBin': 'Já está nesta coleção',
  'component.sidebar.bins': 'Coleções',
  'component.sidebar.clips': 'Clipes',
  'component.sidebar.contentTypes': 'Tipos de conteúdo',
  'component.sidebar.deleteBin': 'Excluir coleção',
  'component.sidebar.editBin': 'Editar coleção',
  'component.sidebar.newBin': 'Nova coleção',
  'component.sidebar.smartBinAutomatic': 'Coleção inteligente — Automática',
  'component.sidebar.smartBinCountMatches': 'Coleção inteligente · {count} correspondências',
  'component.sidebar.sources': 'Fontes',
  'component.sidebar.toggleBins': 'Mostrar ou ocultar coleções',
  'component.sidebar.toggleClips': 'Mostrar ou ocultar clipes',
  'component.sidebar.toggleTools': 'Mostrar ou ocultar ferramentas',
  'component.sidebar.tools': 'Ferramentas',
  'destination.activity': 'Atividade',
  'destination.help': 'Ajuda',
  'destination.insights': 'Estatísticas',
  'destination.operations': 'Operações',
  'destination.playground': 'Playground',
  'destination.settings': 'Configurações',
  'destination.transformations': 'Transformações',
  'feature.activityLog.label': 'Atividade',
  'feature.analytics.label': 'Estatísticas',
  'feature.bins.label': 'Coleções',
  'feature.bins.description': 'Organizar clipes manualmente ou automaticamente com coleções inteligentes.',
  'feature.hud.label': 'HUD',
  'feature.ocr.label': 'OCR',
  'feature.pinning.label': 'Fixação',
  'feature.revisions.label': 'Histórico de versões',
  'native.app.settings': 'Configurações…',
  'native.clips.bins': 'Coleções',
  'native.clips.history': 'Histórico',
  'native.clips.noted': 'Com notas',
  'native.clips.pinned': 'Fixados',
  'native.clips.protected': 'Protegidos',
  'native.clips.queue': 'Fila',
  'native.clips.title': 'Clipes',
  'native.clips.trashed': 'Na Lixeira',
  'native.edit.pin': 'Fixar ou desafixar',
  'native.file.newBin': 'Nova coleção…',
  'native.tools.insights': 'Estatísticas',
  'native.tools.playground': 'Playground',
  'native.tools.savedTransforms': 'Transformações salvas',
  'native.tray.startQueue': 'Iniciar colagem sequencial',
  'native.tray.toggleHud': 'Mostrar ou ocultar o HUD',
  'native.view.toggleSidebar': 'Mostrar ou ocultar a barra lateral',
  'native.window.fullscreen': 'Ativar ou sair da tela cheia',
  'collection.pinAClip': 'Fixar um clipe para mantê-lo no topo e encontrá-lo aqui.',
  'component.settingsSecurityPanel.approveUnlockFromANearbyPairedAppleWatch': 'Aprovar o desbloqueio usando um Apple Watch emparelhado próximo.',
  'component.settingsSecurityPanel.clipboardHistoryWillOpenWithoutAuthenticationSavedUnlockPreferencesWillNoLonger': 'O histórico da área de transferência será aberto sem autenticação. As preferências de desbloqueio salvas não protegerão mais o acesso.',
  'component.contentExtractorManagerDialog.extractors': 'Extratores',
  'component.settingsAnalysisPanel.test': 'Teste',
  'component.settingsGeneralPanel.clipboard': 'Área de transferência',
  'component.settingsSyncPanel.backup': 'Backup',
  'format.characterCount': { one: '{count} caractere', other: '{count} caracteres' },
  'format.clipCount': { one: '{count} clipe', other: '{count} clipes' },
  'format.dayCount': { one: '{count} dia', other: '{count} dias' },
  'format.entryCount': { one: '{count} entrada', other: '{count} entradas' },
  'format.fileCount': { one: '{count} arquivo', other: '{count} arquivos' },
  'format.versionCount': { one: '{count} versão', other: '{count} versões' },
};

for (const [key, value] of Object.entries(overrides)) {
  assert.ok(key in english, `Unknown Brazilian Portuguese editorial key: ${key}`);
  portuguese[key] = value;
}

for (const [key, source] of Object.entries(english)) {
  if (typeof source !== 'string' || typeof portuguese[key] !== 'string') continue;
  if (/\bPlayground\b/.test(source)) {
    portuguese[key] = portuguese[key]
      .replaceAll('Playground (Área de Teste)', 'Playground')
      .replaceAll('Área de teste', 'Playground');
  }
  if (/\bBins?\b/.test(source)) {
    portuguese[key] = portuguese[key]
      .replace(/(?:lixeiras|pastas) inteligentes/gi, 'coleções inteligentes')
      .replace(/(?:lixeira|pasta) inteligente/gi, 'coleção inteligente')
      .replace(/\bSmart Bins\b/g, 'coleções inteligentes')
      .replace(/\bSmart Bin\b/g, 'coleção inteligente')
      .replace(/\bBins\b/g, 'coleções')
      .replace(/\bBin\b/g, 'coleção')
      .replace(/\bpastas\b/gi, 'coleções')
      .replace(/\bpasta\b/gi, 'coleção');
    if (!/\bTrash\b/.test(source)) {
      portuguese[key] = portuguese[key]
        .replace(/\blixeiras\b/gi, 'coleções')
        .replace(/\blixeira\b/gi, 'coleção');
    }
    portuguese[key] = portuguese[key]
      .replace(/\bo coleção\b/gi, 'a coleção')
      .replace(/\bum coleção\b/gi, 'uma coleção')
      .replace(/\beste coleção\b/gi, 'esta coleção')
      .replace(/\bdo coleção\b/gi, 'da coleção')
      .replace(/\bao coleção\b/gi, 'à coleção')
      .replace(/\bos coleções\b/gi, 'as coleções');
  }
  if (/\bclips?\b/i.test(source) && !/\bclipboard\b/i.test(source)) {
    portuguese[key] = portuguese[key]
      .replace(/áreas de transferência/gi, 'clipes')
      .replace(/área de transferência/gi, 'clipe')
      .replace(/\btrechos\b/gi, 'clipes')
      .replace(/\btrecho\b/gi, 'clipe')
      .replace(/\bclips\b/gi, 'clipes')
      .replace(/\bas clipes\b/gi, 'os clipes')
      .replace(/\buma clipe\b/gi, 'um clipe')
      .replace(/\ba clipe\b/gi, 'o clipe');
  }
  portuguese[key] = portuguese[key]
    .replace(/\baplicações\b/gi, 'aplicativos')
    .replace(/\baplicação\b/gi, 'aplicativo')
    .replace(/\bdestravagem\b/gi, 'desbloqueio')
    .replace(/\bdestravar\b/gi, 'desbloquear');
}

const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, portuguese[key]]));
fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
console.log(`Applied ${Object.keys(overrides).length} reviewed Brazilian Portuguese overrides.`);
