import fs from 'node:fs';
import assert from 'node:assert/strict';

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const path = 'src/locales/fr-FR.json';
const french = JSON.parse(fs.readFileSync(path, 'utf8'));

const overrides = {
  'app.emptyTrashConfirmation': 'Vider la corbeille ?',
  'app.emptyTrashDescription': 'Supprime définitivement tous les clips non protégés de la corbeille. Les clips protégés seront conservés.',
  'app.emptyTrashEllipsis': 'Vider la corbeille…',
  'component.settingsSearchHistoryPanel.clearAllEllipsis': 'Tout effacer…',
  'component.settingsGeneralPanel.chooseHowMuchSuccessfulSearchHistoryToKeep': 'Choisissez la quantité d’historique des recherches réussies à conserver.',
  'component.settingsGeneralPanel.keepSearchesFor': 'Conserver les recherches pendant',
  'component.settingsGeneralPanel.olderSearchesAreRemovedAutomatically': 'Les recherches plus anciennes sont supprimées automatiquement.',
  'component.settingsGeneralPanel.maximumSearchAge': 'Âge maximal des recherches',
  'component.settingsGeneralPanel.maximumSearches': 'Nombre maximal de recherches',
  'component.settingsGeneralPanel.theOldestSearchesAreRemovedFirst': 'Les recherches les plus anciennes sont supprimées en premier.',
  'component.settingsGeneralPanel.maximumSearchesRetained': 'Nombre maximal de recherches conservées',
  'component.settingsGeneralPanel.bothSearchLimitsApplyUnlimitedAndForeverDisableAutomaticRemoval': 'Les deux limites s’appliquent. Illimité et Toujours désactivent la suppression automatique.',
  'component.settingsGeneralPanel.valueSearchesDefault': '{value} recherches (Par défaut)',
  'component.welcomeBackupRestore.chooseAPastedFullBackupFile': 'Choisissez une sauvegarde complète (.pastedbackup).',
  'component.welcomeBackupRestore.chooseBackup': 'Choisir une sauvegarde…',
  'component.welcomeBackupRestore.pastedFullBackup': 'Sauvegarde complète',
  'component.welcomeBackupRestore.restoreTheCompleteWorkspaceFromAPastedbackupFile': 'Restaurez tout l’espace de travail depuis un fichier .pastedbackup.',
  'component.welcomeSetup.restoreAPastedFullBackupOrImportClipboardHistory': 'Restaurez une sauvegarde complète ou importez l’historique du presse-papiers depuis presque partout.',
  'component.helpView.activityKeepsAPrivacySafeLocalAuditTrailInsightsSummarizesTheActive': 'Activité conserve un journal d’audit local respectueux de la vie privée ; Statistiques résume la bibliothèque active sans télémétrie.',
  'component.helpView.organizeClipsAcrossSeveralManualBinsAndUseSearchVisualLabelsInsights': 'Organisez les clips dans plusieurs collections manuelles, puis utilisez Recherche, Étiquettes visuelles, Statistiques et Activité pour les retrouver et les comprendre.',
  'component.helpView.pageResetActionsPreviewExactSettingChangesAndDoNotResetClips': 'Les réinitialisations de page affichent les modifications exactes et ne réinitialisent ni les clips ni les pages sans rapport. La réinitialisation d’usine reste une action destructive distincte.',
  'component.helpView.privateBrowserExclusionBlocksCaptureFromSupportedPrivateOrIncognitoWindowsWhen': 'L’exclusion des navigateurs privés bloque les fenêtres privées ou de navigation privée détectées. Si la détection est indisponible, choisissez de poursuivre la capture ou d’exclure entièrement ce navigateur.',
  'component.analyticsView.sourceCountPercent': { one: '{count} clip ({percent} %)', other: '{count} clips ({percent} %)' },
  'component.builtinLifecycleManagerDialog.captureStableReferenceUsage': 'La référence stable identifie cette fonctionnalité dans l’API et la bibliothèque partagée. Affichez-la avec {command}.',
  'component.builtinLifecycleManagerDialog.stableReferenceUsage': 'La référence stable identifie cet élément ({kind}) dans la CLI et l’API. Utilisez-la avec {command}.',
  'component.contentExtractorManagerDialog.customCommandProtocolDescription': 'Lors de l’enregistrement, {versionFlag} est exécuté sans contenu de clip pour vérifier l’exécutable sélectionné. Pour l’extraction, la commande reçoit {requestCommand} et doit écrire sur la sortie standard un objet JSON dont le champ {outputField} contient une chaîne ou null. Elle s’exécute localement avec une limite de 60 secondes.',
  'component.deleteBinDialog.binContentsQuestion': { one: 'Cette collection contient {count} clip. Que doit-il devenir ?', other: 'Cette collection contient {count} clips. Que doivent-ils devenir ?' },
  'component.deleteBinDialog.deleteNamedBin': 'Supprimer « {name} » ?',
  'component.helpView.classifierRescanDescription': 'La modification d’un classificateur affecte les nouveaux clips de texte. {action} réapplique explicitement l’ordre actuel des classificateurs et peut modifier les types de contenu, l’appartenance aux collections intelligentes et le masquage du contenu sensible.',
  'component.helpView.commandReferenceDescription': 'Les commandes qui renvoient des enregistrements ou les détails d’une modification prennent en charge {flag} aux endroits indiqués. Les fonctionnalités désactivées refusent les commandes associées au lieu de changer silencieusement de comportement.',
  'component.helpView.dismissHudShortcutDescription': 'Appuyez sur {key} pour fermer immédiatement le HUD ou un menu ouvert.',
  'component.helpView.hudNumberShortcutDescription': 'Dans le HUD, appuyez sur {modifier}{start} à {modifier}{end} pour coller immédiatement les éléments {start} à {end}.',
  'component.helpView.ocrStatusDescription': 'La désactivation de l’OCR annule les tâches en arrière-plan et ignore les résultats tardifs tout en conservant le texte déjà produit. Sélectionnez un compteur d’état non nul pour ouvrir les clips correspondants dans la recherche, ou consultez la progression avec {command}.',
  'component.helpView.visualLabels': 'Libellés visuels',
  'component.helpView.visualLabelsDescription': 'Apple Vision Labels sur macOS et les libellés llama.cpp facultatifs sur les autres plateformes trouvent des sujets et objets recherchables sans remplacer l’image d’origine.',
  'component.helpView.visualLabelFilteringDescription': 'Les libellés détectés sont modifiables dans l’Inspecteur du clip. Les Extracteurs de libellés peuvent appliquer le post-traitement partagé de confiance minimale avant que les libellés acceptés deviennent recherchables.',
  'component.helpView.openHudHotkeyDescription': 'Appuyez sur {hotkey} pour ouvrir la fenêtre compacte du presse-papiers près du pointeur. Utilisez les touches fléchées pour naviguer et Retour pour coller.',
  'component.helpView.permanentDeletionDescription': 'La suppression est définitive lorsque vous maintenez Option ou Alt pendant la suppression, videz la corbeille ou désactivez la corbeille.',
  'component.helpView.protectionDescription': 'La protection empêche la suppression et la rétention automatique jusqu’à ce que le clip ne soit plus protégé.',
  'component.helpView.restoreTrashedClipsDescription': '« Restaurer les clips de la corbeille », dans Réglages › Général › Corbeille, renvoie tous les clips de la corbeille vers l’historique.',
  'component.pinnedClipShelf.morePinnedCount': '+{count} autres épinglés',
  'component.settingsHotkeysPanel.accessibilityInstructions': 'Autorisez {app} dans {settingsPath}.',
  'component.settingsHotkeysPanel.developmentAccessibilityInstructions': 'En développement, autorisez l’IDE ou le terminal actif dans {settingsPath}.',
  'component.settingsSyncPanel.exportFileSummary': { one: '{count} fichier {extension} sera créé.', other: '{count} fichiers {extension} seront créés.' },
  'format.characterCount': { one: '{count} caractère', other: '{count} caractères' },
  'format.fileCount': { one: '{count} fichier', other: '{count} fichiers' },
  'format.versionCount': { one: '{count} version', other: '{count} versions' },
  'app.deleteSelectedPermanently': 'Supprimer définitivement la sélection',
  'app.emptyClip': 'Clip vide',
  'app.ignoredApp': 'Application ignorée : {name}',
  'app.imageClip': 'Clip image',
  'app.loadingOlderClips': 'Chargement des anciens clips…',
  'app.moveSelectedToTrash': 'Déplacer la sélection vers la corbeille',
  'app.pinSelected': 'Épingler la sélection',
  'app.resultCount': { one: '{count} résultat', other: '{count} résultats' },
  'app.searchResultCount': { one: '{count} résultat de recherche', other: '{count} résultats de recherche' },
  'app.unpinSelected': 'Désépingler la sélection',
  'action.addNote': 'Ajouter une note',
  'action.editNote': 'Modifier la note',
  'action.testInPlayground': 'Tester dans Playground',
  'action.unpin': 'Désépingler',
  'collection.bin': 'Collection',
  'collection.history': 'Historique',
  'collection.noted': 'Avec notes',
  'collection.pinned': 'Épinglés',
  'collection.protected': 'Protégés',
  'collection.queue': 'File d’attente',
  'collection.search': 'Recherche',
  'collection.trashed': 'Corbeille',
  'common.automatic': 'Automatique',
  'common.back': 'Retour',
  'common.cancel': 'Annuler',
  'common.close': 'Fermer',
  'common.custom': 'Personnalisé',
  'common.default': 'Par défaut',
  'common.delete': 'Supprimer',
  'common.description': 'Description',
  'common.dismiss': 'Fermer',
  'common.done': 'Terminé',
  'common.edit': 'Modifier',
  'common.enabled': 'Activé',
  'common.name': 'Nom',
  'common.new': 'Nouveau',
  'common.noBin': 'Aucune collection',
  'common.reset': 'Réinitialiser',
  'common.retry': 'Réessayer',
  'common.save': 'Enregistrer',
  'common.saved': 'Enregistré',
  'common.system': 'Système',
  'common.unknownSource': 'Source inconnue',
  'component.analyticsView.noExtension': 'Sans extension',
  'component.appDialog.discard': 'Ignorer',
  'component.appDialog.discardUnsavedChanges': 'Ignorer les modifications non enregistrées ?',
  'component.binModal.emoji.archive': 'Archive',
  'component.binModal.emoji.bookmark': 'Signet',
  'component.binModal.emoji.clipboard': 'Presse-papiers',
  'component.binModal.emoji.complete': 'Terminé',
  'component.binModal.emoji.openFolder': 'Dossier ouvert',
  'component.binModal.emoji.pin': 'Épingle',
  'component.binModal.emoji.settings': 'Réglages',
  'component.contentExtractorManagerDialog.createSearchableRepresentationsFromClipContentTheLowestPriorityNumberRunsFirst': 'Créer des représentations interrogeables à partir du contenu des clips. Le numéro de priorité le plus bas s’exécute en premier.',
  'component.contentExtractorManagerDialog.noAdditionalResourcesAreRequired': 'Aucune ressource supplémentaire n’est nécessaire.',
  'component.settingsAnalysisPanel.onePerLineAnyMayMatch': '(une par ligne ; une seule correspondance suffit)',
  'component.settingsAnalysisPanel.phoneGuardrails': 'Garde-fous pour les numéros de téléphone',
  'component.settingsAnalysisPanel.proseGuardrails': 'Garde-fous pour le texte courant',
  'component.settingsAnalysisPanel.rescanCanChangeDerivedOrganization': 'Les types de contenu, l’appartenance aux collections intelligentes et le masquage du contenu sensible peuvent changer. Les images et les fichiers restent inchangés.',
  'component.settingsAnalysisPanel.rescannedCountTextClipsCount2Reclassified': '{count} clips texte réanalysés ; {count2} reclassés.',
  'component.settingsAnalysisPanel.rescannedCountTextClipsCount2ReclassifiedAndCount3Failed': '{count} clips texte réanalysés ; {count2} reclassés et {count3} en échec.',
  'format.clipCount': { one: '{count} clip', other: '{count} clips' },
  'format.dayCount': { one: '{count} jour', other: '{count} jours' },
  'format.entryCount': { one: '{count} entrée', other: '{count} entrées' },
  'format.fileSummaryMore': '{name} + {count} autres',
  'component.activityLogView.hudPasted': 'Collé depuis le HUD',
  'component.activityLogView.queuePasted': 'Collé depuis la file d’attente',
  'component.clearHistoryDialog.moveAllUnpinnedAndUnprotectedClipboardHistoryIntoTrashPinnedClipsProtected': 'Déplacer dans la corbeille tout l’historique du presse-papiers qui n’est ni épinglé ni protégé ? Les clips épinglés, les clips protégés et les définitions des collections seront conservés.',
  'component.clearHistoryDialog.permanentlyDeleteAllUnpinnedAndUnprotectedClipboardHistoryPinnedClipsProtectedClips': 'Supprimer définitivement tout l’historique du presse-papiers qui n’est ni épinglé ni protégé ? Les clips épinglés, les clips protégés et les définitions des collections seront conservés.',
  'component.helpView.rightClickAClipForQueuePinProtectNoteBinTransformAnd': 'Cliquez sur un clip avec le bouton droit pour accéder aux actions File d’attente, Épingler, Protéger, Note, Collection, Transformation et Corbeille.',
  'component.settingsSyncPanel.addsTrashBinsTransformsOperationsContentTypesClassifiersAndOcr': 'Ajoute la corbeille, les collections, les transformations, les opérations, les types de contenu, les classificateurs et l’OCR.',
  'component.settingsFeaturesPanel.functionality': 'Fonctionnalités',
  'component.settingsTabs.about': 'À propos',
  'component.settingsTabs.analysis': 'Analyse',
  'component.settingsTabs.appExclusions': 'Exclusions d’applications',
  'component.settingsTabs.functionality': 'Fonctionnalités',
  'component.settingsTabs.general': 'Général',
  'component.settingsTabs.hotkeys': 'Raccourcis',
  'component.settingsTabs.intelligence': 'Intelligence',
  'component.settingsTabs.notifications': 'Notifications',
  'component.settingsTabs.security': 'Sécurité',
  'component.settingsTabs.settingsSections': 'Sections des réglages',
  'component.settingsTabs.storage': 'Stockage',
  'component.sidebar.bins': 'Collections',
  'component.sidebar.clips': 'Clips',
  'component.sidebar.contentTypes': 'Types de contenu',
  'component.sidebar.alreadyInThisBin': 'Déjà dans cette collection',
  'component.sidebar.deleteBin': 'Supprimer la collection',
  'component.sidebar.editBin': 'Modifier la collection',
  'component.sidebar.newBin': 'Nouvelle collection',
  'component.sidebar.smartBinAutomatic': 'Collection intelligente — automatique',
  'component.sidebar.smartBinCountMatches': 'Collection intelligente · {count} correspondances',
  'component.sidebar.sources': 'Sources',
  'component.sidebar.toggleBins': 'Afficher ou masquer les collections',
  'component.sidebar.toggleClips': 'Afficher ou masquer les clips',
  'component.sidebar.toggleTools': 'Afficher ou masquer les outils',
  'component.sidebar.tools': 'Outils',
  'component.settingsFeaturesPanel.featurePresets': 'Préréglages des fonctionnalités',
  'component.settingsFeaturesPanel.simpleEnablesEssentialClipboardToolsFullEnablesEveryFeatureDisablingAFeature': 'Simple active les outils essentiels du presse-papiers. Complet active toutes les fonctionnalités. La désactivation d’une fonctionnalité masque son interface et arrête les nouveaux comportements tout en conservant les données existantes, sauf indication contraire.',
  'destination.activity': 'Activité',
  'destination.help': 'Aide',
  'destination.insights': 'Statistiques',
  'destination.operations': 'Opérations',
  'destination.playground': 'Playground',
  'destination.settings': 'Réglages',
  'destination.transformations': 'Transformations',
  'feature.activityLog.label': 'Activité',
  'feature.analytics.label': 'Statistiques',
  'feature.appLock.label': 'Verrouillage de l’app',
  'feature.bins.label': 'Collections',
  'feature.bins.description': 'Organiser les clips manuellement ou automatiquement avec des collections intelligentes.',
  'feature.cli.description': 'Utilisez {command} pour automatiser les flux de travail du presse-papiers.',
  'feature.cli.label': 'Interface en ligne de commande',
  'feature.contentClassification.label': 'Classification du contenu',
  'feature.help.label': 'Aide',
  'feature.hud.label': 'HUD',
  'feature.notes.label': 'Notes',
  'feature.notes.description': 'Annoter les clips et parcourir la collection Avec notes.',
  'feature.notifications.label': 'Notifications',
  'feature.ocr.label': 'OCR',
  'feature.pinning.label': 'Épinglage',
  'feature.preset.custom': 'Personnalisé',
  'feature.preset.full': 'Complet',
  'feature.preset.simple': 'Simple',
  'feature.protection.label': 'Protection',
  'feature.queue.label': 'File de copie',
  'feature.revisions.label': 'Historique des versions',
  'feature.sources.label': 'Sources',
  'feature.transcriptions.label': 'Transcription',
  'feature.transformations.label': 'Transformations',
  'feature.trash.label': 'Corbeille',
  'feature.types.label': 'Types de contenu',
  'native.clips.bins': 'Collections',
  'native.clips.history': 'Historique',
  'native.clips.noted': 'Avec notes',
  'native.clips.pinned': 'Épinglés',
  'native.clips.protected': 'Protégés',
  'native.clips.queue': 'File d’attente',
  'native.clips.title': 'Clips',
  'native.clips.trashed': 'Corbeille',
  'native.file.newBin': 'Nouvelle collection…',
  'native.file.title': 'Fichier',
  'native.app.settings': 'Réglages…',
  'native.edit.pin': 'Épingler ou désépingler',
  'native.tools.activity': 'Activité',
  'native.tools.insights': 'Statistiques',
  'native.tools.playground': 'Playground',
  'native.tools.savedTransforms': 'Transformations enregistrées',
  'native.tray.startQueue': 'Démarrer le collage séquentiel',
  'native.tray.toggleHud': 'Afficher ou masquer le HUD',
  'native.view.toggleSidebar': 'Afficher ou masquer la barre latérale',
  'native.window.fullscreen': 'Activer ou quitter le plein écran',
  'native.window.hud': 'HUD',
  'registry.classifier.credential.description': 'Formats connus de clés API et affectations de secrets',
  'registry.classifier.envVariable.description': 'Affectation unique d’une variable d’environnement de style shell',
  'registry.classifier.filePath.description': 'Chemins Unix, relatifs au dossier personnel, UNC et avec lettre de lecteur',
  'registry.classifier.macAddress.description': 'Adresses matérielles séparées par des deux-points, des tirets ou des points',
  'registry.classifier.paymentCard.description': 'Numéros de carte potentiels avec validation de la somme de contrôle',
  'registry.contentType.credential.label': 'Identifiants',
  'registry.contentType.jwt.label': 'Jeton Web JSON',
  'registry.operation.alternatingCase.name': 'Casse alternée',
  'registry.operation.cleanUrlTracking.name': 'Supprimer le suivi des URL',
  'registry.operation.collapseWhitespace.name': 'Regrouper les espaces',
  'registry.operation.fileMarkdownLinks.name': 'Liens Markdown vers les fichiers',
  'registry.operation.numberLines.name': 'Numéroter les lignes',
  'registry.operation.quoteText.name': 'Citer le texte',
  'registry.operation.sentenceCase.name': 'Casse de phrase',
  'registry.operation.smileysToEmoji.name': 'Convertir les smileys en émojis',
  'registry.operation.straightenPunctuation.name': 'Simplifier la ponctuation',
  'registry.operation.stripDiacritics.name': 'Supprimer les signes diacritiques',
  'registry.operation.titlecase.name': 'Casse de titre',
  'registry.operation.trim.name': 'Supprimer les espaces en début et fin',
};

for (const [key, value] of Object.entries(overrides)) {
  assert.ok(key in english, `Unknown French editorial key: ${key}`);
  french[key] = value;
}

for (const [key, source] of Object.entries(english)) {
  if (typeof source !== 'string' || typeof french[key] !== 'string') continue;
  if (/\bPlayground\b/.test(source)) {
    french[key] = french[key]
      .replaceAll('Terrain de jeu', 'Playground')
      .replaceAll('Bac à sable', 'Playground');
  }
  if (/\bBins?\b/.test(source)) {
    french[key] = french[key]
      .replaceAll('Smart Bins', 'collections intelligentes')
      .replaceAll('Smart Bin', 'collection intelligente')
      .replaceAll('Bins', 'collections')
      .replaceAll('Bin', 'collection')
      .replaceAll('bacs intelligents', 'collections intelligentes')
      .replaceAll('bac intelligent', 'collection intelligente')
      .replaceAll('bacs', 'collections')
      .replaceAll('bac', 'collection');
    if (!/\bTrash\b/.test(source)) {
      french[key] = french[key]
        .replaceAll('des dossiers', 'des collections')
        .replaceAll('du dossier', 'de la collection')
        .replaceAll('un dossier', 'une collection')
        .replaceAll('ce dossier', 'cette collection')
        .replaceAll('le dossier', 'la collection')
        .replaceAll('son dossier', 'sa collection')
        .replaceAll('dossiers', 'collections')
        .replaceAll('dossier', 'collection')
        .replaceAll('Dossiers', 'Collections')
        .replaceAll('Dossier', 'Collection')
        .replaceAll('corbeilles', 'collections')
        .replaceAll('corbeille', 'collection')
        .replaceAll('Corbeilles', 'Collections')
        .replaceAll('Corbeille', 'Collection')
        .replaceAll('arborescences', 'collections')
        .replaceAll('arborescence', 'collection')
        .replaceAll('bouteilles', 'collections')
        .replaceAll('bouteille', 'collection');
    }
  }
  if (/\b(?:Pin|Pinned|Unpin|Unpinned)\b/.test(source)) {
    french[key] = french[key]
      .replaceAll('Défaire l’attache', 'Désépingler')
      .replaceAll("Défaire l'attache", 'Désépingler')
      .replaceAll('attacher', 'épingler')
      .replaceAll('Attaché', 'Épinglé')
      .replaceAll('attaché', 'épinglé')
      .replaceAll('fixés', 'épinglés')
      .replaceAll('fixé', 'épinglé');
  }
}

const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, french[key]]));
fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
console.log(`Applied ${Object.keys(overrides).length} reviewed French overrides.`);
