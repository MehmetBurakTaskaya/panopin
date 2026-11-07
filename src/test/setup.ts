// Tests switch languages synchronously, as the settings do once a language is loaded: every
// language is loaded up front.
import { LANGUAGES, loadLanguage } from '../i18n';

await Promise.all(LANGUAGES.map(({ code }) => loadLanguage(code)));
