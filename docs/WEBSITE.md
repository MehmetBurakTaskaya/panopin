# Website

Two pieces live in `site/` (D-069): Panopin's own page for GitHub Pages, and a download section
for mehmetburaktaskaya.com.

The download links point to `releases/latest/download/<file>`. GitHub serves them only for a
published release (drafts do not count), so they return 404 until v1.0.0 is published.

## Panopin's page on GitHub Pages

`site/index.html` is one static page in English and Turkish. It needs no build step and loads
nothing from other sites.

1. On GitHub, open **Settings → Pages → Build and deployment** and set **Source** to
   **GitHub Actions**. This is needed once.
2. Open **Actions → Website → Run workflow**. The page appears at
   <https://mehmetburaktaskaya.github.io/panopin/>.
3. Run the workflow again after changing `site/`; nothing is published automatically.

To serve it from a subdomain such as `panopin.mehmetburaktaskaya.com` instead, enter it under
**Settings → Pages → Custom domain**, add a `CNAME` record for it pointing to
`mehmetburaktaskaya.github.io` at your DNS provider, and change the `og:image` address at the top
of `site/index.html` to the new domain.

To look at the page locally: `python -m http.server 4318 --directory site`, then open
<http://localhost:4318/>.

## The download section for mehmetburaktaskaya.com

`site/embed/download-section.html` is a single `<section>`. Its styles apply only inside
`.panopin-embed`, the icon is drawn inline, and it has no script and no external files. Text
colour and font come from your page, so it suits light and dark designs. `data-lang="tr"` shows
the Turkish text; anything else shows English.

### On the Astro site

The site is built with Astro, with English pages at the root and Turkish pages under `/tr/`.

1. Create `src/components/PanopinDownload.astro`:

   ```astro
   ---
   const { lang = 'en' } = Astro.props;
   ---
   ```

   Below the closing `---`, paste the whole content of `site/embed/download-section.html`.
2. In the pasted markup, change `data-lang="en"` to `data-lang={lang}`, and `<style>` to
   `<style is:inline>`. The CSS is already scoped by its class, so Astro does not need to process
   it.
3. Put the section where it belongs, for example a "Projects" part of the home page:
   - English page (for example `src/pages/index.astro`):
     `import PanopinDownload from '../components/PanopinDownload.astro';` in the frontmatter, then
     `<PanopinDownload />` in the page.
   - Turkish page (for example `src/pages/tr/index.astro`):
     `import PanopinDownload from '../../components/PanopinDownload.astro';`, then
     `<PanopinDownload lang="tr" />`.
4. Run the site locally (`npm run dev`) and check both languages, on a phone width too, and in
   dark mode if the site has one.
5. Once v1.0.0 is published, click "Download for Windows" once to make sure the file arrives.

### On any other page

Paste the content of `download-section.html` where the section should appear. On Turkish pages,
change `data-lang="en"` to `data-lang="tr"`.

### Changing the text

Every sentence appears twice, in `<span data-l="en">` and `<span data-l="tr">`. Edit both, keep
the file in `site/embed/` up to date, and paste it again.
