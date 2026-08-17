import fs from 'node:fs';
import assert from 'node:assert/strict';

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const path = 'src/locales/ar.json';
const arabic = JSON.parse(fs.readFileSync(path, 'utf8'));

const count = (zero, one, two, few, many, other) => ({ zero, one, two, few, many, other });
const overrides = {
  'common.automatic': 'تلقائي',
  'common.back': 'رجوع',
  'common.cancel': 'إلغاء',
  'common.close': 'إغلاق',
  'common.custom': 'مخصص',
  'common.default': 'افتراضي',
  'common.delete': 'حذف',
  'common.description': 'الوصف',
  'common.discardChangesQuestion': 'تجاهل التغييرات؟',
  'common.dismiss': 'إغلاق',
  'common.done': 'تم',
  'common.duplicate': 'تكرار',
  'common.edit': 'تعديل',
  'common.enabled': 'مفعّل',
  'common.english': 'الإنجليزية',
  'common.errorMessage': 'خطأ: {error}',
  'common.input': 'الإدخال',
  'common.name': 'الاسم',
  'common.new': 'جديد',
  'common.noBin': 'بلا مجموعة',
  'common.output': 'الإخراج',
  'common.priority': 'الأولوية',
  'common.remove': 'إزالة',
  'common.reset': 'إعادة تعيين',
  'common.resetToDefault': 'إعادة التعيين إلى الافتراضي',
  'common.retry': 'إعادة المحاولة',
  'common.save': 'حفظ',
  'common.saved': 'تم الحفظ',
  'common.saving': 'جارٍ الحفظ…',
  'common.stableId': 'المعرّف الثابت',
  'common.stableReference': 'المرجع الثابت',
  'common.system': 'النظام',
  'common.technicalDetails': 'تفاصيل تقنية',
  'common.thisActionCannotBeUndone': 'لا يمكن التراجع عن هذا الإجراء.',
  'common.unknown': 'غير معروف',
  'common.unknownSource': 'مصدر غير معروف',
  'collection.bin': 'مجموعة',
  'collection.activeClipboardHistory': 'سجل الحافظة النشط',
  'collection.addANoteToAClip': 'أضف ملاحظة إلى أي مقتطف لتوضيحه والعثور عليه هنا لاحقًا.',
  'collection.addTextClipsToQueue': 'أضف مقتطفات نصية أو سجّل عمليات النسخ للصقها بالتسلسل.',
  'collection.clipsMovedToTrash': 'تبقى المقتطفات المنقولة إلى سلة المهملات هنا حتى إفراغها.',
  'collection.copySomethingInAnyApp': 'انسخ أي شيء في أي تطبيق. سيظهر هنا تلقائيًا.',
  'collection.dragClipsHere': 'اسحب المقتطفات إلى هنا أو اختر هذه المجموعة من أحد المقتطفات.',
  'collection.history': 'السجل',
  'collection.noted': 'ذات ملاحظات',
  'collection.pinned': 'المثبّتة',
  'collection.protected': 'المحمية',
  'collection.queue': 'قائمة الانتظار',
  'collection.search': 'البحث',
  'collection.trashed': 'سلة المهملات',
  'collection.noClipsYet': 'لا توجد مقتطفات بعد',
  'collection.noClipsInBin': 'لا توجد مقتطفات في {name}',
  'collection.noFacetClips': 'لا توجد مقتطفات من فئة {label}',
  'collection.noMatchingClips': 'لا توجد مقتطفات مطابقة',
  'collection.noNotedClips': 'لا توجد مقتطفات ذات ملاحظات',
  'collection.noPinnedClips': 'لا توجد مقتطفات مثبّتة',
  'collection.noProtectedClips': 'لا توجد مقتطفات محمية',
  'collection.pinAClip': 'ثبّت مقتطفًا لإبقائه في الأعلى والعثور عليه هنا.',
  'collection.protectAClip': 'احمِ مقتطفًا لمنع التنظيف التلقائي من حذفه.',
  'collection.queueIsEmpty': 'قائمة الانتظار فارغة',
  'collection.searchActiveAndTrashedClips': 'ابحث في المقتطفات النشطة والمحذوفة.',
  'collection.searchYourClips': 'ابحث في مقتطفاتك',
  'collection.smartBinMatchesAppearAutomatically': 'ستظهر المقتطفات المطابقة لقواعد {name} هنا تلقائيًا.',
  'collection.sourceClipsAppearAutomatically': 'ستظهر المقتطفات المنسوخة من {label} هنا تلقائيًا.',
  'collection.thisBin': 'هذه المجموعة',
  'collection.thisBinIsEmpty': 'هذه المجموعة فارغة',
  'collection.trashIsEmpty': 'سلة المهملات فارغة',
  'collection.tryAnotherSearchOrFilter': 'جرّب بحثًا أو عامل تصفية آخر.',
  'collection.typeClipsAppearAutomatically': 'ستظهر المقتطفات من نوع {label} هنا تلقائيًا.',
  'component.externalHistoryImport.couldNotCheckForExistingClipboardHistory': 'تعذّر التحقق من وجود سجل حافظة سابق.',
  'component.helpView.chooseHistoryACollectionOrABinFromTheLeftSidebar': 'اختر السجل أو عرضًا محددًا مسبقًا أو مجموعة من الشريط الجانبي.',
  'component.helpView.previewCopyOrganizeOrTransformItInTheRightColumn': 'عاين المقتطف أو انسخه أو نظّمه أو حوّله في عمود المعاينة.',
  'component.helpView.titleCaseCamelcase': '• حالة العنوان / camelCase',
  'component.helpView.trimWhitespace': '• إزالة المسافات البيضاء الزائدة',
  'component.helpView.uppercaseLowercase': '• أحرف كبيرة / أحرف صغيرة',
  'component.helpView.urlEncodeDecode': '• ترميز URL / فك ترميزه',
  'component.sidebar.bins': 'المجموعات',
  'component.sidebar.alreadyInThisBin': 'موجود بالفعل في هذه المجموعة',
  'component.sidebar.clearSearch': 'مسح البحث',
  'component.sidebar.clearSearch2': 'مسح البحث',
  'component.sidebar.clips': 'المقتطفات',
  'component.sidebar.collapseSidebar': 'طي الشريط الجانبي',
  'component.sidebar.contentTypes': 'أنواع المحتوى',
  'component.sidebar.deleteBin': 'حذف المجموعة',
  'component.sidebar.editBin': 'تعديل المجموعة',
  'component.sidebar.expandSidebar': 'توسيع الشريط الجانبي',
  'component.sidebar.newBin': 'مجموعة جديدة',
  'component.sidebar.searchAllClips': 'البحث في كل المقتطفات',
  'component.sidebar.searchFilters': 'عوامل تصفية البحث',
  'component.sidebar.searchFilters2': 'عوامل تصفية البحث',
  'component.sidebar.smartBinAutomatic': 'مجموعة ذكية — تلقائية',
  'component.sidebar.smartBinCountMatches': 'مجموعة ذكية · {count} مطابقة',
  'component.sidebar.sources': 'المصادر',
  'component.sidebar.tools': 'الأدوات',
  'component.sidebar.toggleBins': 'إظهار المجموعات أو إخفاؤها',
  'component.sidebar.toggleClips': 'إظهار المقتطفات أو إخفاؤها',
  'component.sidebar.toggleTools': 'إظهار الأدوات أو إخفاؤها',
  'component.settingsTabs.about': 'حول التطبيق',
  'component.settingsTabs.analysis': 'التحليل',
  'component.settingsTabs.appExclusions': 'استثناءات التطبيقات',
  'component.settingsTabs.functionality': 'الوظائف',
  'component.settingsTabs.general': 'عام',
  'component.settingsTabs.hotkeys': 'اختصارات لوحة المفاتيح',
  'component.settingsTabs.intelligence': 'الذكاء',
  'component.settingsTabs.notifications': 'الإشعارات',
  'component.settingsTabs.security': 'الأمان',
  'component.settingsTabs.settingsSections': 'أقسام الإعدادات',
  'component.settingsTabs.storage': 'التخزين',
  'component.transformWorkspaceHeader.library': 'المكتبة',
  'component.transformWorkspaceHeader.transformationWorkspace': 'مساحة عمل التحويلات',
  'native.file.title': 'ملف',
  'native.clips.noted': 'ذات ملاحظات',
  'native.file.newBin': 'مجموعة جديدة…',
  'component.settingsGeneralPanel.resetsTheLeftSidebarAndMiddleHistoryListPanelWidthsToTheir': 'يعيد عرض الشريط الجانبي وقائمة السجل إلى القيم الافتراضية.',
  'settings.activity.language.label': 'اللغة',
  'settings.activity.language.system': 'تلقائي',
  'settings.general.language.ariaLabel': 'لغة الواجهة',
  'settings.general.language.automaticDetail': 'لغة النظام',
  'settings.general.language.description': 'استخدم لغة النظام عند توفر ترجمة، أو اختر لغة.',
  'settings.general.language.englishDetail': 'الإنجليزية',
  'settings.general.language.label': 'اللغة',
  'app.resultCount': count('لا نتائج', 'نتيجة واحدة', 'نتيجتان', '{count} نتائج', '{count} نتيجة', '{count} نتيجة'),
  'app.searchResultCount': count('لا توجد نتائج بحث', 'نتيجة بحث واحدة', 'نتيجتا بحث', '{count} نتائج بحث', '{count} نتيجة بحث', '{count} نتيجة بحث'),
  'component.clipBinPicker.selectedBins': count('لا توجد مجموعات', 'مجموعة واحدة', 'مجموعتان', '{count} مجموعات', '{count} مجموعة', '{count} مجموعة'),
  'component.contentTypeGroupManagerDialog.customGroupUsage': count('يمكن أرشفة المجموعات المخصصة عندما تكون فارغة. لا تستخدم أي أنواع هذه المجموعة حاليًا.', 'يمكن أرشفة المجموعات المخصصة عندما تكون فارغة. يستخدم نوع واحد هذه المجموعة حاليًا.', 'يمكن أرشفة المجموعات المخصصة عندما تكون فارغة. يستخدم نوعان هذه المجموعة حاليًا.', 'يمكن أرشفة المجموعات المخصصة عندما تكون فارغة. تستخدم {count} أنواع هذه المجموعة حاليًا.', 'يمكن أرشفة المجموعات المخصصة عندما تكون فارغة. يستخدم {count} نوعًا هذه المجموعة حاليًا.', 'يمكن أرشفة المجموعات المخصصة عندما تكون فارغة. يستخدم {count} نوع هذه المجموعة حاليًا.'),
  'component.externalHistoryImport.importedCount': count('لم يُستورد شيء', 'تم استيراد عنصر واحد', 'تم استيراد عنصرين', 'تم استيراد {count} عناصر', 'تم استيراد {count} عنصرًا', 'تم استيراد {count} عنصر'),
  'component.externalHistoryImport.skippedCount': count('لم يتم تخطي شيء', 'تم تخطي عنصر واحد', 'تم تخطي عنصرين', 'تم تخطي {count} عناصر', 'تم تخطي {count} عنصرًا', 'تم تخطي {count} عنصر'),
  'component.pinnedClipShelf.stackedPinnedClips': count('لا توجد مقتطفات مثبّتة مكدّسة', 'مقتطف مثبّت واحد مكدّس', 'مقتطفان مثبّتان مكدّسان', '{count} مقتطفات مثبّتة مكدّسة', '{count} مقتطفًا مثبّتًا مكدّسًا', '{count} مقتطف مثبّت مكدّس'),
  'component.settingsHotkeysPanel.conflicts': count('لا تعارضات', 'تعارض واحد', 'تعارضان', '{count} تعارضات', '{count} تعارضًا', '{count} تعارض'),
  'component.settingsOcrPanel.eligibleImages': count('لا توجد صور قابلة للمسح لإنشاء نص قابل للبحث.', 'يمكن مسح صورة واحدة لإنشاء نص قابل للبحث.', 'يمكن مسح صورتين لإنشاء نص قابل للبحث.', 'يمكن مسح {count} صور لإنشاء نص قابل للبحث.', 'يمكن مسح {count} صورة لإنشاء نص قابل للبحث.', 'يمكن مسح {count} صورة لإنشاء نص قابل للبحث.'),
  'format.clipCount': count('لا مقتطفات', 'مقتطف واحد', 'مقتطفان', '{count} مقتطفات', '{count} مقتطفًا', '{count} مقتطف'),
  'format.dayCount': count('لا أيام', 'يوم واحد', 'يومان', '{count} أيام', '{count} يومًا', '{count} يوم'),
  'format.entryCount': count('لا إدخالات', 'إدخال واحد', 'إدخالان', '{count} إدخالات', '{count} إدخالًا', '{count} إدخال'),
  'component.analyticsView.sourceCountPercent': count('لا مقتطفات ({percent}٪)', 'مقتطف واحد ({percent}٪)', 'مقتطفان ({percent}٪)', '{count} مقتطفات ({percent}٪)', '{count} مقتطفًا ({percent}٪)', '{count} مقتطف ({percent}٪)'),
  'component.clipPreview.noteCount': count('ملاحظات (٠)', 'ملاحظة واحدة', 'ملاحظتان', 'ملاحظات ({count})', 'ملاحظات ({count})', 'ملاحظات ({count})'),
  'component.deleteBinDialog.binContentsQuestion': count('هذه المجموعة فارغة. ماذا يجب أن يحدث؟', 'تحتوي هذه المجموعة على مقتطف واحد. ماذا يجب أن يحدث له؟', 'تحتوي هذه المجموعة على مقتطفين. ماذا يجب أن يحدث لهما؟', 'تحتوي هذه المجموعة على {count} مقتطفات. ماذا يجب أن يحدث لها؟', 'تحتوي هذه المجموعة على {count} مقتطفًا. ماذا يجب أن يحدث لها؟', 'تحتوي هذه المجموعة على {count} مقتطف. ماذا يجب أن يحدث له؟'),
  'component.pipelineEditorModal.stepCount': count('الخطوات (٠)', 'خطوة واحدة', 'خطوتان', 'الخطوات ({count})', 'الخطوات ({count})', 'الخطوات ({count})'),
  'component.sequentialQueueBar.bufferCount': count('المخزن المؤقت فارغ', 'عنصر واحد في المخزن المؤقت', 'عنصران في المخزن المؤقت', '{count} عناصر في المخزن المؤقت', '{count} عنصرًا في المخزن المؤقت', '{count} عنصر في المخزن المؤقت'),
  'component.settingsSyncPanel.exportFileSummary': count('لن يتم إنشاء ملفات {extension}.', 'سيتم إنشاء ملف {extension} واحد.', 'سيتم إنشاء ملفي {extension}.', 'سيتم إنشاء {count} ملفات {extension}.', 'سيتم إنشاء {count} ملفًا من نوع {extension}.', 'سيتم إنشاء {count} ملف من نوع {extension}.'),
  'format.characterCount': count('لا أحرف', 'حرف واحد', 'حرفان', '{count} أحرف', '{count} حرفًا', '{count} حرف'),
  'format.fileCount': count('لا ملفات', 'ملف واحد', 'ملفان', '{count} ملفات', '{count} ملفًا', '{count} ملف'),
  'format.versionCount': count('لا إصدارات', 'إصدار واحد', 'إصداران', '{count} إصدارات', '{count} إصدارًا', '{count} إصدار'),
};

for (const [key, value] of Object.entries(overrides)) {
  assert.ok(key in english, `Unknown Arabic editorial key: ${key}`);
  arabic[key] = value;
}

for (const [key, source] of Object.entries(english)) {
  if (typeof source !== 'string' || typeof arabic[key] !== 'string') continue;
  if (/\bBins?\b/.test(source)) {
    arabic[key] = arabic[key]
      .replaceAll('الحاويات الذكية', 'المجموعات الذكية')
      .replaceAll('الحاوية الذكية', 'المجموعة الذكية')
      .replaceAll('حاويات ذكية', 'مجموعات ذكية')
      .replaceAll('حاوية ذكية', 'مجموعة ذكية')
      .replaceAll('سلات المهملات الذكية', 'المجموعات الذكية')
      .replaceAll('سلة المهملات الذكية', 'المجموعة الذكية')
      .replaceAll('سلات المهملات', 'المجموعات')
      .replaceAll('الحاويات', 'المجموعات')
      .replaceAll('الحاوية', 'المجموعة')
      .replaceAll('حاويات', 'مجموعات')
      .replaceAll('حاوية', 'مجموعة')
      .replaceAll('المجلدات', 'المجموعات')
      .replaceAll('المجلد', 'المجموعة')
      .replaceAll('مجلدات', 'مجموعات')
      .replaceAll('مجلد', 'مجموعة')
      .replaceAll('الصناديق', 'المجموعات')
      .replaceAll('الصندوق', 'المجموعة');
  }
  if (/\bClips?\b/.test(source)) {
    arabic[key] = arabic[key]
      .replaceAll('مقاطع الفيديو', 'المقتطفات')
      .replaceAll('مقطع فيديو', 'مقتطف')
      .replaceAll('شرائط التقطيع', 'المقتطفات')
      .replaceAll('شريط التقطيع', 'مقتطف')
      .replaceAll('اللقطات', 'المقتطفات')
      .replaceAll('اللقطة', 'المقتطف')
      .replaceAll('المشابك', 'المقتطفات')
      .replaceAll('المشبك', 'المقتطف')
      .replaceAll('المقاطع', 'المقتطفات')
      .replaceAll('المقطع', 'المقتطف')
      .replaceAll('مقاطع', 'مقتطفات')
      .replaceAll('مقطع', 'مقتطف');
  }
  if (/\bHistory\b/.test(source)) {
    arabic[key] = arabic[key]
      .replaceAll('سجل الأحداث', 'السجل')
      .replaceAll('سجل الحدث', 'السجل');
  }
}

const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, arabic[key]]));
fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
console.log(`Applied ${Object.keys(overrides).length} reviewed Arabic overrides.`);
