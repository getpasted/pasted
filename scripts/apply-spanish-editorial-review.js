import assert from 'node:assert/strict';
import fs from 'node:fs';

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const path = 'src/locales/es.json';
const spanish = JSON.parse(fs.readFileSync(path, 'utf8'));

const overrides = {
  'app.emptyTrashConfirmation': '¿Vaciar la papelera?',
  'app.emptyTrashDescription': 'Elimina permanentemente todos los clips sin protección de la papelera. Los clips protegidos se conservarán.',
  'app.emptyTrashEllipsis': 'Vaciar la papelera…',
  'component.settingsSearchHistoryPanel.clearAllEllipsis': 'Borrar todo…',
  'component.settingsGeneralPanel.chooseHowMuchSuccessfulSearchHistoryToKeep': 'Elige cuánto historial de búsquedas correctas conservar.',
  'component.settingsGeneralPanel.keepSearchesFor': 'Conservar búsquedas durante',
  'component.settingsGeneralPanel.olderSearchesAreRemovedAutomatically': 'Las búsquedas más antiguas se eliminan automáticamente.',
  'component.settingsGeneralPanel.maximumSearchAge': 'Antigüedad máxima de las búsquedas',
  'component.settingsGeneralPanel.maximumSearches': 'Máximo de búsquedas',
  'component.settingsGeneralPanel.theOldestSearchesAreRemovedFirst': 'Las búsquedas más antiguas se eliminan primero.',
  'component.settingsGeneralPanel.maximumSearchesRetained': 'Máximo de búsquedas conservadas',
  'component.settingsGeneralPanel.bothSearchLimitsApplyUnlimitedAndForeverDisableAutomaticRemoval': 'Se aplican ambos límites. Ilimitado y Para siempre desactivan la eliminación automática.',
  'component.settingsGeneralPanel.valueSearchesDefault': '{value} búsquedas (Predeterminado)',
  'component.welcomeBackupRestore.chooseAPastedFullBackupFile': 'Elige una copia de seguridad completa (.pastedbackup).',
  'component.welcomeBackupRestore.chooseBackup': 'Elegir copia de seguridad…',
  'component.welcomeBackupRestore.pastedFullBackup': 'Copia de seguridad completa',
  'component.welcomeBackupRestore.restoreTheCompleteWorkspaceFromAPastedbackupFile': 'Restaura todo el espacio de trabajo desde un archivo .pastedbackup.',
  'component.welcomeSetup.restoreAPastedFullBackupOrImportClipboardHistory': 'Restaura una copia de seguridad completa o importa el historial del portapapeles desde casi cualquier lugar.',
  'component.helpView.activityKeepsAPrivacySafeLocalAuditTrailInsightsSummarizesTheActive': 'Actividad mantiene un registro de auditoría local que protege la privacidad; Estadísticas resume la biblioteca activa sin telemetría.',
  'component.helpView.organizeClipsAcrossSeveralManualBinsAndUseSearchVisualLabelsInsights': 'Organiza clips en varias colecciones manuales y usa Búsqueda, Etiquetas visuales, Estadísticas y Actividad para encontrarlos y comprenderlos.',
  'component.helpView.pageResetActionsPreviewExactSettingChangesAndDoNotResetClips': 'Los restablecimientos de cada página muestran los cambios exactos y no restablecen clips ni páginas no relacionadas. El restablecimiento de fábrica sigue siendo una acción destructiva independiente.',
  'component.helpView.privateBrowserExclusionBlocksCaptureFromSupportedPrivateOrIncognitoWindowsWhen': 'La exclusión de navegadores privados bloquea las ventanas privadas o de incógnito detectadas. Cuando no se puede detectar el estado, elige si quieres continuar capturando o excluir ese navegador por completo.',
  'component.helpView.ocrStatusDescription': 'Deshabilitar OCR cancela el trabajo en segundo plano y descarta los resultados tardíos, pero conserva el texto completado. Seleccione un recuento de estado distinto de cero para abrir los clips correspondientes en Buscar, o inspeccione el progreso con {command}.',
  'component.helpView.visualLabels': 'Etiquetas visuales',
  'component.helpView.visualLabelsDescription': 'Apple Vision Labels en macOS y las etiquetas opcionales de llama.cpp en otras plataformas encuentran sujetos y objetos buscables sin reemplazar la imagen original.',
  'component.helpView.visualLabelFilteringDescription': 'Las etiquetas detectadas se pueden editar en el Inspector del clip. Los Extractores de etiquetas pueden aplicar el posprocesamiento compartido de confianza mínima antes de que las etiquetas aceptadas se puedan buscar.',
  'app.emptyClip': 'Clip vacío',
  'app.imageClip': 'Clip de imagen',
  'app.pinSelected': 'Fijar selección',
  'app.unpinSelected': 'Desfijar selección',
  'collection.bin': 'Colección',
  'collection.history': 'Historial',
  'collection.noted': 'Con notas',
  'collection.pinned': 'Fijados',
  'collection.protected': 'Protegidos',
  'collection.queue': 'Cola',
  'collection.search': 'Buscar',
  'collection.searchActiveAndTrashedClips': 'Buscar clips activos y enviados a la Papelera.',
  'collection.thisBin': 'esta colección',
  'collection.thisBinIsEmpty': 'Esta colección está vacía',
  'collection.trashed': 'En la Papelera',
  'common.automatic': 'Automático',
  'common.back': 'Atrás',
  'common.dismiss': 'Cerrar',
  'common.done': 'Listo',
  'common.duplicate': 'Duplicar',
  'common.enabled': 'Activado',
  'common.noBin': 'Sin colección',
  'common.reset': 'Restablecer',
  'common.resetToDefault': 'Restablecer valores predeterminados',
  'common.saved': 'Guardado',
  'component.analyticsView.sourceCountPercent': { one: '{count} clip ({percent} %)', other: '{count} clips ({percent} %)' },
  'component.activityLogView.pinningChanged': 'Cambio de fijación',
  'component.clearHistoryDialog.moveAllUnpinnedAndUnprotectedClipboardHistoryIntoTrashPinnedClipsProtected': '¿Mover a la Papelera todo el historial del portapapeles que no esté fijado ni protegido? Se conservarán los clips fijados, los clips protegidos y las definiciones de las colecciones.',
  'component.clearHistoryDialog.moveClipboardHistoryToTrash': '¿Mover el historial del portapapeles a la Papelera?',
  'component.clearHistoryDialog.permanentlyDeleteAllUnpinnedAndUnprotectedClipboardHistoryPinnedClipsProtectedClips': '¿Eliminar permanentemente todo el historial del portapapeles que no esté fijado ni protegido? Se conservarán los clips fijados, los clips protegidos y las definiciones de las colecciones.',
  'component.deleteBinDialog.binContentsQuestion': { one: 'Esta colección contiene {count} clip. ¿Qué debe ocurrir con él?', other: 'Esta colección contiene {count} clips. ¿Qué debe ocurrir con ellos?' },
  'component.deleteBinDialog.clipsMatchedByThisSmartBinWillBePreserved': 'Se conservarán los clips que coincidan con esta colección inteligente.',
  'component.deleteBinDialog.deleteBin': 'Eliminar colección',
  'component.deleteBinDialog.deleteBin2': 'Eliminar colección «',
  'component.deleteBinDialog.deleteNamedBin': '¿Eliminar «{name}»?',
  'component.deleteBinDialog.protectedClipsWillBeKeptInNoBin': 'Los clips protegidos se conservarán sin colección.',
  'component.deleteBinDialog.thisBinContains': 'Esta colección contiene',
  'component.deleteBinDialog.thisBinIsEmptyNoClipsWillBeAffected': 'Esta colección está vacía. No se modificará ningún clip.',
  'component.deleteBinDialog.trash': 'Papelera',
  'component.clipPreview.copyClip': 'Copiar clip',
  'component.clipPreview.viewCountClipVersions': 'Ver {count} versiones del clip',
  'component.helpView.assignClipsToOneManualBinOrRemoveTheirManualBin': 'Asignar clips a una colección manual o quitarlos de ella.',
  'component.helpView.listABoundedPageFromHistoryTrashABinOrPinnedClips': 'Mostrar una página limitada del Historial, la Papelera, una colección o los clips fijados.',
  'component.helpView.rightClickAClipForQueuePinProtectNoteBinTransformAnd': 'Hacer clic con el botón derecho en un clip para acceder a Cola, Fijar, Proteger, Nota, Colección, Transformación y Papelera.',
  'component.settingsSyncPanel.addsTrashBinsTransformsOperationsContentTypesClassifiersAndOcr': 'Añade la Papelera, las colecciones, las transformaciones, las operaciones, los tipos de contenido, los clasificadores y el OCR.',
  'component.settingsSyncPanel.exportFileSummary': { one: 'Se creará {count} archivo {extension}.', other: 'Se crearán {count} archivos {extension}.' },
  'component.settingsSyncPanel.fullBackupFileSummary': 'Se creará 1 instantánea de SQLite {extension}.',
  'component.settingsSyncPanel.historyAndOrganizationMergedProcessedCountClips': 'Historial y organización combinados. Se procesaron {count} clips.',
  'component.settingsAnalysisPanel.cardChecksum': 'Suma de comprobación de la tarjeta',
  'component.settingsGeneralPanel.alwaysShowDockAndMenuBar': 'Mostrar siempre el Dock y la barra de menús',
  'component.settingsGeneralPanel.autoHideDockIcon': 'Ocultar automáticamente el icono del Dock',
  'component.settingsGeneralPanel.dockAndMenuBarIcon': 'Icono del Dock y de la barra de menús',
  'component.settingsGeneralPanel.dockAndMenuBarIconBehavior': 'Comportamiento del icono del Dock y de la barra de menús',
  'component.settingsSyncPanel.valueClipsValue2BinsValue3TransformsValue4Operations': '{value} clips · {value2} colecciones · {value3} transformaciones · {value4} operaciones',
  'component.sidebar.alreadyInThisBin': 'Ya está en esta colección',
  'component.sidebar.bins': 'Colecciones',
  'component.sidebar.clips': 'Clips',
  'component.sidebar.contentTypes': 'Tipos de contenido',
  'component.sidebar.deleteBin': 'Eliminar colección',
  'component.sidebar.editBin': 'Editar colección',
  'component.sidebar.newBin': 'Nueva colección',
  'component.sidebar.smartBinAutomatic': 'Colección inteligente — Automática',
  'component.sidebar.smartBinCountMatches': 'Colección inteligente · {count} coincidencias',
  'component.sidebar.sources': 'Fuentes',
  'component.sidebar.toggleBins': 'Mostrar u ocultar colecciones',
  'component.sidebar.toggleClips': 'Mostrar u ocultar clips',
  'component.sidebar.toggleTools': 'Mostrar u ocultar herramientas',
  'component.sidebar.tools': 'Herramientas',
  'destination.activity': 'Actividad',
  'destination.help': 'Ayuda',
  'destination.insights': 'Estadísticas',
  'destination.operations': 'Operaciones',
  'destination.playground': 'Playground',
  'destination.settings': 'Configuración',
  'destination.transformations': 'Transformaciones',
  'feature.activityLog.label': 'Actividad',
  'feature.analytics.label': 'Estadísticas',
  'feature.bins.label': 'Colecciones',
  'feature.bins.description': 'Organizar clips manualmente o automáticamente con colecciones inteligentes.',
  'feature.hud.label': 'HUD',
  'feature.ocr.label': 'OCR',
  'feature.pinning.label': 'Fijación',
  'feature.revisions.label': 'Historial de versiones',
  'native.app.settings': 'Configuración…',
  'native.clips.bins': 'Colecciones',
  'native.clips.history': 'Historial',
  'native.clips.noted': 'Con notas',
  'native.clips.pinned': 'Fijados',
  'native.clips.protected': 'Protegidos',
  'native.clips.queue': 'Cola',
  'native.clips.title': 'Clips',
  'native.clips.trashed': 'En la Papelera',
  'native.edit.pin': 'Fijar o desfijar',
  'native.file.newBin': 'Nueva colección…',
  'native.tools.insights': 'Estadísticas',
  'native.tools.playground': 'Playground',
  'native.tools.savedTransforms': 'Transformaciones guardadas',
  'native.tray.startQueue': 'Iniciar pegado secuencial',
  'native.tray.toggleHud': 'Mostrar u ocultar el HUD',
  'native.view.toggleSidebar': 'Mostrar u ocultar la barra lateral',
  'native.window.fullscreen': 'Activar o salir de pantalla completa',
  'collection.pinAClip': 'Fijar un clip para mantenerlo en la parte superior y encontrarlo aquí.',
  'format.characterCount': { one: '{count} carácter', other: '{count} caracteres' },
  'format.clipCount': { one: '{count} clip', other: '{count} clips' },
  'format.dayCount': { one: '{count} día', other: '{count} días' },
  'format.entryCount': { one: '{count} entrada', other: '{count} entradas' },
  'format.fileCount': { one: '{count} archivo', other: '{count} archivos' },
  'format.versionCount': { one: '{count} versión', other: '{count} versiones' },
};

for (const [key, value] of Object.entries(overrides)) {
  assert.ok(key in english, `Unknown Spanish editorial key: ${key}`);
  spanish[key] = value;
}

for (const [key, source] of Object.entries(english)) {
  if (typeof source !== 'string' || typeof spanish[key] !== 'string') continue;
  if (/\bPlayground\b/.test(source)) {
    spanish[key] = spanish[key]
      .replaceAll('Zona de pruebas', 'Playground')
      .replaceAll('Área de pruebas', 'Playground');
  }
  if (/\bBins?\b/.test(source)) {
    spanish[key] = spanish[key]
      .replace(/(?:papeleras|carpetas|contenedores|carritos) inteligentes/gi, 'colecciones inteligentes')
      .replace(/(?:papelera|carpeta|contenedor|carrito) inteligente/gi, 'colección inteligente')
      .replace(/\bSmart Bins\b/g, 'colecciones inteligentes')
      .replace(/\bSmart Bin\b/g, 'colección inteligente')
      .replace(/\bBins\b/g, 'colecciones')
      .replace(/\bBin\b/g, 'colección')
      .replace(/\b(?:carpetas|contenedores|carritos)\b/gi, 'colecciones')
      .replace(/\b(?:carpeta|contenedor|carrito)\b/gi, 'colección');
    if (!/\bTrash\b/.test(source)) {
      spanish[key] = spanish[key]
        .replace(/\bpapeleras\b/gi, 'colecciones')
        .replace(/\bpapelera\b/gi, 'colección');
    }
    spanish[key] = spanish[key]
      .replace(/\bel colección\b/gi, 'la colección')
      .replace(/\bun colección\b/gi, 'una colección')
      .replace(/\beste colección\b/gi, 'esta colección')
      .replace(/\bdel colección\b/gi, 'de la colección')
      .replace(/\bal colección\b/gi, 'a la colección')
      .replace(/\blos colecciones\b/gi, 'las colecciones');
  }
  if (/\bclips?\b/i.test(source) && !/\bclipboard\b/i.test(source)) {
    spanish[key] = spanish[key]
      .replace(/\bfragmentos\b/gi, 'clips')
      .replace(/\bfragmento\b/gi, 'clip')
      .replace(/\brecortes\b/gi, 'clips')
      .replace(/\brecorte\b/gi, 'clip')
      .replace(/\bcintas\b/gi, 'clips')
      .replace(/\bcinta\b/gi, 'clip');
  }
  if (/\b(?:Pin|Pinned|Unpin|Unpinned)\b/.test(source)) {
    spanish[key] = spanish[key]
      .replace(/\bdesmarcar\b/gi, 'desfijar')
      .replace(/\bmarcados\b/gi, 'fijados')
      .replace(/\bmarcado\b/gi, 'fijado')
      .replace(/\banclados\b/gi, 'fijados')
      .replace(/\banclado\b/gi, 'fijado')
      .replace(/\bmarcar\b/gi, 'fijar')
      .replace(/\banclar\b/gi, 'fijar');
  }
}

const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, spanish[key]]));
fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
console.log(`Applied ${Object.keys(overrides).length} reviewed Spanish overrides.`);
