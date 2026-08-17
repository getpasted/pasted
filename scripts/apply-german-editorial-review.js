import fs from 'node:fs';
import assert from 'node:assert/strict';

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const path = 'src/locales/de-DE.json';
const german = JSON.parse(fs.readFileSync(path, 'utf8'));

const overrides = {
  'component.analyticsView.sourceCountPercent': { one: '{count} Clip ({percent} %)', other: '{count} Clips ({percent} %)' },
  'component.binModal.openEmojiPickerShortcut': 'Klicken Sie, um die Emoji-Auswahl zu öffnen ({shortcut}).',
  'component.builtinLifecycleManagerDialog.captureStableReferenceUsage': 'Die stabile Referenz identifiziert diese Funktion in der API und der gemeinsamen Bibliothek. Mit {command} auflisten.',
  'component.builtinLifecycleManagerDialog.stableReferenceUsage': 'Die stabile Referenz identifiziert {kind} in der CLI und der API. Mit {command} verwenden.',
  'component.clipCard.queuePosition': 'Position {position} in der Warteschlange',
  'component.contentExtractorManagerDialog.customCommandProtocolDescription': 'Beim Speichern wird {versionFlag} ohne Clip-Inhalt ausgeführt, um die ausgewählte ausführbare Datei zu prüfen. Zur Extraktion wird {requestCommand} übergeben; die Standardausgabe muss ein JSON-Objekt mit dem Feld {outputField} enthalten, dessen Wert eine Zeichenfolge oder null ist. Der Befehl läuft lokal mit einem Zeitlimit von 60 Sekunden.',
  'component.deleteBinDialog.binContentsQuestion': { one: 'Diese Sammlung enthält {count} Clip. Was soll damit geschehen?', other: 'Diese Sammlung enthält {count} Clips. Was soll damit geschehen?' },
  'component.deleteBinDialog.deleteNamedBin': '„{name}“ löschen?',
  'component.deleteTransformationAssetDialog.removesKindFromLibrary': 'Dadurch wird {kind} aus der Bibliothek entfernt. Bereits damit erstellte oder geänderte Clips bleiben unverändert.',
  'component.helpView.classifierRescanDescription': 'Änderungen an einem Klassifikator wirken sich auf neue Textclips aus. {action} wendet die aktuelle Klassifikatorreihenfolge erneut an und kann Inhaltstypen, die Zugehörigkeit zu intelligenten Sammlungen und die Maskierung sensibler Inhalte ändern.',
  'component.helpView.commandReferenceDescription': 'Befehle, die Datensätze oder Änderungsdetails zurückgeben, unterstützen an den angegebenen Stellen {flag}. Deaktivierte Funktionen lehnen die zugehörigen Befehle ab, statt ihr Verhalten stillschweigend zu ändern.',
  'component.helpView.hudNumberShortcutDescription': 'Drücken Sie im HUD die Zahlen {start} bis {end}, um die Elemente {start} bis {end} sofort einzufügen.',
  'component.helpView.ocrStatusDescription': 'Durch das Deaktivieren von OCR werden Hintergrundarbeiten abgebrochen und verspätete Ergebnisse verworfen; bereits erstellter Text bleibt erhalten. Fortschritt mit {command} prüfen.',
  'component.helpView.openHudShortcutDescription': 'Drücken Sie {shortcut}, um das kompakte Zwischenablagefenster am Mauszeiger zu öffnen. Mit den Pfeiltasten navigieren und mit Return einfügen.',
  'component.helpView.permanentDeleteShortcutDescription': 'Wenn Sie {modifier} gedrückt halten, wird das Papierkorbsymbol zu einer roten {symbol}-Schaltfläche, mit der Elemente unter Umgehung des Papierkorbs endgültig gelöscht werden.',
  'component.helpView.permanentDeletionDescription': 'Endgültig gelöscht wird beim Löschen mit gedrückter Wahl- oder Alt-Taste, beim Leeren des Papierkorbs oder beim Deaktivieren des Papierkorbs.',
  'component.helpView.protectionDescription': 'Der Schutz verhindert das Löschen und die automatische Aufbewahrung, bis der Schutz des Clips aufgehoben wird.',
  'component.helpView.restoreDescription': 'Beim Wiederherstellen wird ein Clip aus dem Papierkorb in den Verlauf zurückgebracht.',
  'component.helpView.restoreTrashedClipsDescription': '„Clips aus dem Papierkorb wiederherstellen“ unter Einstellungen › Allgemein › Papierkorb bringt alle Clips aus dem Papierkorb in den Verlauf zurück.',
  'component.pinnedClipShelf.morePinnedCount': '+{count} weitere angeheftet',
  'component.settingsSyncPanel.exportFileSummary': { one: '{count} {extension}-Datei wird erstellt.', other: '{count} {extension}-Dateien werden erstellt.' },
  'format.characterCount': { one: '{count} Zeichen', other: '{count} Zeichen' },
  'format.versionCount': { one: '{count} Version', other: '{count} Versionen' },
  'app.deleteSelectedPermanently': 'Auswahl endgültig löschen',
  'app.deselect': 'Auswahl aufheben',
  'app.emptyClip': 'Leerer Clip',
  'app.imageClip': 'Bildclip',
  'app.loadingOlderClips': 'Ältere Clips werden geladen…',
  'app.moveSelectedToTrash': 'Auswahl in den Papierkorb verschieben',
  'app.pinSelected': 'Auswahl anheften',
  'app.resultCount': { one: '{count} Ergebnis', other: '{count} Ergebnisse' },
  'app.searchResultCount': { one: '{count} Suchergebnis', other: '{count} Suchergebnisse' },
  'app.unpinSelected': 'Auswahl lösen',
  'common.unknownSource': 'Unbekannte Quelle',
  'component.analyticsView.noExtension': 'Keine Dateiendung',
  'component.appDialog.discardUnsavedChanges': 'Nicht gespeicherte Änderungen verwerfen?',
  'component.binContextMenu.editBin': 'Sammlung bearbeiten…',
  'component.binModal.binIcons': 'Sammlungssymbole',
  'component.binModal.chooseAnIconForThisBin': 'Ein Symbol für diese Sammlung auswählen.',
  'component.binModal.chooseBinIcon': 'Sammlungssymbol auswählen',
  'component.binModal.chooseHowClipsEnterThisBinAndWhatHappensNext': 'Festlegen, wie Clips in diese Sammlung gelangen und was anschließend geschieht.',
  'component.binModal.chooseWhichClipsAutomaticallyEnterThisSmartBin': 'Festlegen, welche Clips automatisch in diese intelligente Sammlung gelangen.',
  'component.binModal.labelBinText': 'Sammlungstext für {label}',
  'component.binModal.newBin': 'Neue Sammlung',
  'component.contentExtractorManagerDialog.chooseALocalWhisperCppGgmlModelFileModelDownloadsAreNot': 'Eine lokale whisper.cpp-GGML-Modelldatei auswählen. Modelle werden nicht automatisch heruntergeladen.',
  'component.contentExtractorManagerDialog.createSearchableRepresentationsFromClipContentTheLowestPriorityNumberRunsFirst': 'Durchsuchbare Darstellungen aus Clip-Inhalten erstellen. Die niedrigste Prioritätsnummer wird zuerst ausgeführt.',
  'component.contentExtractorManagerDialog.engineContract': 'Engine-Vertrag',
  'component.contentExtractorManagerDialog.engineContractsIdentifyTheRuntimeAdapterAndProtocolVersionAndAreManaged': 'Engine-Verträge bestimmen den Laufzeitadapter und die Protokollversion und werden automatisch verwaltet.',
  'component.contentExtractorManagerDialog.nameDeleted': '{name} gelöscht.',
  'component.contentExtractorManagerDialog.resetShippedExtractors': 'Mitgelieferte Extraktoren zurücksetzen?',
  'component.contentExtractorManagerDialog.resources': 'Ressourcen',
  'component.contentExtractorManagerDialog.shippedExtractorsReturnToTheirDefaults': 'Mitgelieferte Extraktoren werden auf ihre Standardwerte zurückgesetzt.',
  'component.clipBinPicker.smartBinMembershipIsManagedAutomatically': 'Die Zugehörigkeit zu intelligenten Sammlungen wird automatisch verwaltet.',
  'component.clipBinPicker.smartBinsAutomatic': 'Intelligente Sammlungen · Automatisch',
  'component.contextMenu.smartBinMembershipIsManagedAutomatically': 'Die Zugehörigkeit zu intelligenten Sammlungen wird automatisch verwaltet.',
  'component.contextMenu.smartBinsAutomatic': 'Intelligente Sammlungen · Automatisch',
  'component.deleteBinDialog.clipsMatchedByThisSmartBinWillBePreserved': 'Clips, die dieser intelligenten Sammlung entsprechen, bleiben erhalten.',
  'component.deleteBinDialog.deleteBin2': 'Sammlung „',
  'component.deleteBinDialog.protectedClipsWillBeKeptInNoBin': 'Geschützte Clips bleiben ohne Sammlung erhalten.',
  'component.deleteBinDialog.thisBinContains': 'Diese Sammlung enthält',
  'component.deleteBinDialog.thisBinIsEmptyNoClipsWillBeAffected': 'Diese Sammlung ist leer. Es sind keine Clips betroffen.',
  'component.helpView.binsAndTransforms': 'Sammlungen und Transformationen',
  'component.helpView.assignClipsToOneManualBinOrRemoveTheirManualBin': 'Clips einer manuellen Sammlung zuweisen oder daraus entfernen.',
  'component.helpView.chooseHistoryACollectionOrABinFromTheLeftSidebar': 'Verlauf, eine vorgegebene Ansicht oder eine Sammlung in der linken Seitenleiste auswählen.',
  'component.helpView.createAManualOrSmartBin': 'Eine manuelle oder intelligente Sammlung erstellen.',
  'component.helpView.deleteABinWithAnExplicitClipDisposition': 'Eine Sammlung löschen und dabei festlegen, was mit ihren Clips geschieht.',
  'component.helpView.duplicateABinAndItsAttachedTransform': 'Eine Sammlung und ihre verknüpfte Transformation duplizieren.',
  'component.helpView.inspectOneBinAndItsAttachedTransform': 'Eine Sammlung und ihre verknüpfte Transformation anzeigen.',
  'component.helpView.listABinSClipsInPersistentOrder': 'Die Clips einer Sammlung in ihrer gespeicherten Reihenfolge auflisten.',
  'component.helpView.listABoundedPageFromHistoryTrashABinOrPinnedClips': 'Eine begrenzte Seite aus Verlauf, Papierkorb, einer Sammlung oder angehefteten Clips auflisten.',
  'component.helpView.listBinsCountsAndSavedOrdering': 'Sammlungen, Anzahlen und gespeicherte Reihenfolge auflisten.',
  'component.helpView.replaceABinSCompleteSavedClipOrder': 'Die vollständig gespeicherte Clip-Reihenfolge einer Sammlung ersetzen.',
  'component.helpView.rightClickAClipForQueuePinProtectNoteBinTransformAnd': 'Per Rechtsklick stehen für einen Clip Warteschlange, Anheften, Schützen, Notiz, Sammlung, Transformation und Papierkorb zur Verfügung.',
  'component.helpView.setOrClearABinSDefaultTransform': 'Die Standardtransformation einer Sammlung festlegen oder entfernen.',
  'component.helpView.setOrClearABinShortcut': 'Das Tastenkürzel einer Sammlung festlegen oder entfernen.',
  'component.helpView.updateABinDefinition': 'Die Definition einer Sammlung aktualisieren.',
  'component.helpView.restoreShippedClassifiersWithoutRemovingCustomEntries': 'Mitgelieferte Klassifikatoren wiederherstellen, ohne benutzerdefinierte Einträge zu entfernen.',
  'component.helpView.whisperTranscriptionUsesAnInstalledWhisperCppExecutableAndASelectedLocal': 'Die Whisper-Transkription verwendet eine installierte whisper.cpp-Datei und ein ausgewähltes lokales GGML-Modell. Für M4A und AAC ist außerdem FFmpeg erforderlich.',
  'component.settingsHotkeysPanel.customBinHotkeys': 'Benutzerdefinierte Sammlungstastenkürzel (',
  'component.settingsHotkeysPanel.globalShortcutsBinActionsAndTransformTriggers': 'Globale Tastenkürzel, Sammlungsaktionen und Transformationsauslöser.',
  'component.settingsHotkeysPanel.noCustomBinsCreatedYetCreateBinsInTheSidebarToAssign': 'Noch keine benutzerdefinierten Sammlungen. Sammlungen können in der Seitenleiste erstellt und anschließend globalen Tastenkürzeln zugewiesen werden.',
  'component.settingsHotkeysPanel.thatBinShortcutCouldNotBeRegistered': 'Dieses Sammlungstastenkürzel konnte nicht registriert werden.',
  'component.settingsSyncPanel.carryingEveryClipBinTransformAndRevisionToItsNewHome': 'Clips, Sammlungen, Transformationen und Versionen werden an den neuen Speicherort übertragen…',
  'component.settingsSyncPanel.valueClipsValue2BinsValue3TransformsValue4Operations': '{value} Clips · {value2} Sammlungen · {value3} Transformationen · {value4} Vorgänge',
  'component.sidebar.smartBinAutomatic': 'Intelligente Sammlung — automatisch',
  'component.sidebar.smartBinCountMatches': 'Intelligente Sammlung · {count} Treffer',
  'registry.classifier.envBlock.description': 'Zwei oder mehr Zuweisungen von Umgebungsvariablen',
  'registry.classifier.envBlock.name': 'Umgebungsblöcke',
  'registry.classifier.envVariable.description': 'Eine einzelne Zuweisung einer Umgebungsvariablen im Shell-Stil',
  'registry.classifier.macAddress.description': 'Durch Doppelpunkt, Bindestrich oder Punkt getrennte Hardwareadressen',
  'registry.extractor.extractorAppleVisionOcr.description': 'Extrahiert lokal mit Apple Vision durchsuchbaren Text aus Bildern.',
  'registry.extractor.extractorTesseractOcr.description': 'Extrahiert lokal mit Tesseract durchsuchbaren Text aus Bildern.',
  'registry.extractor.extractorWhisperTranscription.description': 'Extrahiert lokal mit whisper.cpp durchsuchbaren Text aus Audiodateien.',
  'registry.operation.cleanUrlTracking.name': 'URL-Tracking entfernen',
  'registry.operation.collapseWhitespace.name': 'Leerzeichen zusammenfassen',
  'registry.operation.dedupeLines.name': 'Doppelte Zeilen entfernen',
  'registry.operation.fileMarkdownLinks.name': 'Markdown-Links für Dateien',
  'registry.operation.htmlDecode.name': 'HTML-Entitäten dekodieren',
  'registry.operation.htmlEncode.name': 'HTML-Entitäten kodieren',
  'registry.operation.numberLines.name': 'Zeilen nummerieren',
  'registry.operation.quoteText.name': 'Text zitieren',
  'registry.operation.reverseLines.name': 'Zeilen umkehren',
  'registry.operation.reverseText.name': 'Text umkehren',
  'registry.operation.sortByLength.name': 'Zeilen nach Länge sortieren',
  'registry.operation.sortLinesAsc.name': 'Zeilen sortieren (A–Z)',
  'registry.operation.sortLinesDesc.name': 'Zeilen sortieren (Z–A)',
  'registry.operation.straightenPunctuation.name': 'Satzzeichen vereinfachen',
  'registry.operation.stripDiacritics.name': 'Diakritische Zeichen entfernen',
  'registry.operation.stripNonAlphanumeric.name': 'Nicht alphanumerische Zeichen entfernen',
  'registry.operation.titlecase.name': 'Titel-Schreibweise',
  'registry.operation.trim.name': 'Äußere Leerzeichen entfernen',
  'component.settingsAnalysisPanel.environmentBlock': 'Umgebungsvariablenblock',
  'component.settingsAnalysisPanel.nameDeletedResetCanRecoverShippedClassifiers': '{name} gelöscht. Mitgelieferte Klassifikatoren können durch Zurücksetzen wiederhergestellt werden.',
  'component.settingsAnalysisPanel.regularExpressions': 'Reguläre Ausdrücke',
  'component.settingsAnalysisPanel.rescannedCountTextClipsCount2Reclassified': '{count} Textclips erneut analysiert; {count2} neu klassifiziert.',
  'component.settingsAnalysisPanel.rescannedCountTextClipsCount2ReclassifiedAndCount3Failed': '{count} Textclips erneut analysiert; {count2} neu klassifiziert und {count3} fehlgeschlagen.',
  'component.settingsAnalysisPanel.rescanning': 'Erneute Analyse läuft…',
  'component.settingsAnalysisPanel.resetShippedAnalysisDefinitions': 'Mitgelieferte Analysedefinitionen zurücksetzen?',
  'component.settingsAnalysisPanel.resetShippedClassifierDefinitions': 'Mitgelieferte Klassifikatordefinitionen zurücksetzen?',
  'component.settingsAnalysisPanel.shippedContentTypesContentTypeGroupsAndClassifiersReturnToTheirDefaults': 'Mitgelieferte Inhaltstypen, Inhaltstypgruppen und Klassifikatoren werden auf ihre Standardwerte zurückgesetzt.',
  'component.settingsAnalysisPanel.shippedExtractorsClassifiersContentTypesAndContentTypeGroupsReturnToTheir': 'Mitgelieferte Extraktoren, Klassifikatoren, Inhaltstypen und Inhaltstypgruppen werden auf ihre Standardwerte zurückgesetzt.',
  'format.fileSummaryMore': '{name} + {count} weitere',
};

for (const [key, value] of Object.entries(overrides)) {
  assert.ok(key in english, `Unknown German editorial key: ${key}`);
  german[key] = value;
}

for (const [key, source] of Object.entries(english)) {
  if (typeof source !== 'string' || typeof german[key] !== 'string' || !/\bBins?\b/.test(source)) continue;
  german[key] = german[key]
    .replaceAll('Smart Bins', 'intelligente Sammlungen')
    .replaceAll('Smart Bin', 'intelligente Sammlung')
    .replaceAll('Bins', 'Sammlungen')
    .replaceAll('Bin', 'Sammlung')
    .replaceAll('einen Sammlung', 'eine Sammlung')
    .replaceAll('einem Sammlung', 'einer Sammlung')
    .replaceAll('eines Sammlung', 'einer Sammlung')
    .replaceAll('diesem Sammlung', 'dieser Sammlung')
    .replaceAll('Dieser Sammlung', 'Diese Sammlung')
    .replaceAll('diese Sammlung eingeben', 'in diese Sammlung gelangen')
    .replaceAll('diese intelligente Sammlung eingeben', 'automatisch in diese intelligente Sammlung gelangen');
}

const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, german[key]]));
fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
console.log(`Applied ${Object.keys(overrides).length} reviewed German overrides.`);
