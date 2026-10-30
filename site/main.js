// Language switch and the download button that fits the visitor's system. The page works
// without this script: English, with the Windows download.
(function () {
  var root = document.documentElement;
  var titles = {
    en: 'Panopin — everything you copy, pinned to a cork board',
    tr: 'Panopin — kopyaladığın her şey, mantar panoda raptiyeyle',
  };

  function setLanguage(lang, remember) {
    root.lang = lang;
    root.dataset.lang = lang;
    document.title = titles[lang];
    document.querySelectorAll('[data-set-lang]').forEach(function (button) {
      button.setAttribute('aria-pressed', String(button.dataset.setLang === lang));
    });
    if (remember) {
      try {
        localStorage.setItem('panopin-lang', lang);
      } catch {
        // Storage may be off; the choice then lasts for this visit.
      }
    }
  }

  document.querySelectorAll('[data-set-lang]').forEach(function (button) {
    button.addEventListener('click', function () {
      setLanguage(button.dataset.setLang, true);
    });
  });
  setLanguage(root.dataset.lang === 'tr' ? 'tr' : 'en', false);

  // Windows gets the installer, macOS hears that its version is coming, others see all downloads.
  var ua = navigator.userAgent;
  var platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '';
  var system = /iPhone|iPad|Android/i.test(ua)
    ? 'other'
    : /Win/i.test(platform) || /Windows/.test(ua)
      ? 'windows'
      : /Mac/i.test(platform) || /Mac OS X/.test(ua)
        ? 'mac'
        : 'other';
  var button = document.getElementById('download-main');
  if (system !== 'windows' && button) {
    button.href = 'https://github.com/MehmetBurakTaskaya/panopin/releases/latest';
    button.classList.toggle('muted', system === 'mac');
  }
  document.querySelectorAll('[data-download-label]').forEach(function (label) {
    label.hidden = label.dataset.downloadLabel !== system;
  });
  document.querySelectorAll('[data-download-note]').forEach(function (note) {
    note.hidden = note.dataset.downloadNote !== system;
  });
})();
