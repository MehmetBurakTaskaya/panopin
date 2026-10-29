/* Sample shelf content for the concept mockup. All text and artwork are original placeholders. */
(function () {
  'use strict';
  const svg = (w, h, body) =>
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" preserveAspectRatio="xMidYMid meet" aria-hidden="true">${body}</svg>`;

  // "Code editor" screenshot built from rounded bars.
  const codeLines = [
    [0, [[14, '#b48ef0'], [22, '#6fb3f2'], [6, '#cfd6e4']]],
    [1, [[30, '#f0c36a'], [40, '#7fd48a']]],
    [1, [[18, '#b48ef0'], [12, '#cfd6e4'], [26, '#ef8a7a']]],
    [2, [[44, '#7fd48a']]],
    [1, [[8, '#cfd6e4']]],
    [0, [[6, '#cfd6e4']]],
    [0, []],
    [0, [[16, '#b48ef0'], [30, '#6fb3f2']]],
    [1, [[24, '#f0c36a'], [18, '#cfd6e4'], [20, '#7fd48a']]],
  ];
  let code = '';
  codeLines.forEach(([indent, parts], i) => {
    const y = 17 + i * 9;
    code += `<rect x="8" y="${y}" width="7" height="3.2" rx="1.6" fill="#4a5263"/>`;
    let x = 28 + indent * 9;
    parts.forEach(([w, c]) => { code += `<rect x="${x}" y="${y}" width="${w}" height="3.4" rx="1.7" fill="${c}"/>`; x += w + 3; });
  });

  const ART = {
    code: svg(160, 100,
      '<rect width="160" height="100" fill="#1f2430"/><rect width="160" height="11" fill="#171b24"/>' +
      '<circle cx="7" cy="5.5" r="2" fill="#ef8a7a"/><circle cx="13" cy="5.5" r="2" fill="#f0c36a"/><circle cx="19" cy="5.5" r="2" fill="#7fd48a"/>' +
      '<rect y="11" width="22" height="89" fill="#1a1f29"/>' + code),
    chart: svg(120, 90,
      '<rect width="120" height="90" fill="#fbfcfe"/><rect x="10" y="9" width="46" height="5" rx="2.5" fill="#2c3e50"/>' +
      '<rect x="10" y="17" width="28" height="3" rx="1.5" fill="#9aa7b4"/><path d="M14 78H110" stroke="#cfd6de"/>' +
      [22, 34, 28, 46, 40, 56].map((h, i) => `<rect x="${18 + i * 15}" y="${78 - h}" width="10" height="${h}" rx="2" fill="${i === 5 ? '#1f8a83' : '#7cc4bd'}"/>`).join('') +
      '<polyline points="23,60 38,50 53,54 68,38 83,42 98,26" fill="none" stroke="#f08a5d" stroke-width="2" stroke-linejoin="round"/>'),
    sunset: svg(150, 100,
      '<defs><linearGradient id="artSky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#6b5aa6"/><stop offset=".55" stop-color="#f28c6a"/><stop offset="1" stop-color="#ffd08a"/></linearGradient></defs>' +
      '<rect width="150" height="100" fill="url(#artSky)"/><circle cx="98" cy="62" r="14" fill="#fff1c2" opacity=".95"/>' +
      '<path d="M0 70Q30 52 60 64T120 60T150 62V100H0Z" fill="#4b3f73"/><path d="M0 82Q40 66 80 78T150 74V100H0Z" fill="#32295a"/>' +
      '<path d="M0 92Q50 82 100 90T150 88V100H0Z" fill="#221c40"/>'),
    wire: svg(100, 100,
      '<rect width="100" height="100" fill="#e9edf2"/><rect x="12" y="14" width="76" height="72" rx="6" fill="#fff" stroke="#d3d9e0"/>' +
      '<circle cx="26" cy="30" r="7" fill="#cfd8e3"/><rect x="38" y="25" width="36" height="4" rx="2" fill="#5a6676"/><rect x="38" y="32" width="24" height="3" rx="1.5" fill="#a9b3bf"/>' +
      '<rect x="20" y="46" width="60" height="3" rx="1.5" fill="#c4ccd6"/><rect x="20" y="53" width="52" height="3" rx="1.5" fill="#c4ccd6"/>' +
      '<rect x="20" y="61" width="12" height="6" rx="3" fill="#2f7de1"/><circle cx="29" cy="64" r="2.2" fill="#fff"/><rect x="36" y="62.5" width="26" height="3" rx="1.5" fill="#c4ccd6"/>' +
      '<rect x="46" y="72" width="16" height="8" rx="3" fill="#e3e8ee"/><rect x="64" y="72" width="16" height="8" rx="3" fill="#2f7de1"/>'),
    phone: svg(72, 156,
      '<rect width="72" height="156" rx="8" fill="#f4f6f8"/><rect width="72" height="20" fill="#2f6f6a"/>' +
      '<circle cx="12" cy="10" r="5" fill="#cde7e4"/><rect x="21" y="7" width="26" height="3.5" rx="1.75" fill="#fff"/>' +
      '<rect x="6" y="28" width="40" height="12" rx="5" fill="#fff"/><rect x="26" y="45" width="40" height="18" rx="5" fill="#d5efe4"/>' +
      '<rect x="6" y="68" width="34" height="12" rx="5" fill="#fff"/><rect x="18" y="86" width="48" height="12" rx="5" fill="#d5efe4"/>' +
      '<rect x="6" y="104" width="44" height="18" rx="5" fill="#fff"/><rect x="5" y="140" width="62" height="10" rx="5" fill="#fff" stroke="#dde3e8"/>'),
  };

  window.MOCK = {
    ART,
    items: [
      { id: 'img-editor', kind: 'image', ar: 1.6, art: 'code', title: 'Ekran görüntüsü 14.12.05', ocr: 'fn main println Merhaba, pano!', pinned: true, age: '2 dk' },
      { id: 'img-chart', kind: 'image', ar: 4 / 3, art: 'chart', title: 'Satış grafiği', ocr: 'Aylık satışlar 2026 Ocak Şubat Mart', age: '18 dk' },
      { id: 'img-sunset', kind: 'image', ar: 1.5, art: 'sunset', title: 'Gün batımı', age: '1 sa' },
      { id: 'img-dialog', kind: 'image', ar: 1, art: 'wire', title: 'Ayarlar penceresi taslağı', ocr: 'Bildirimler Kaydet İptal', age: '3 sa' },
      { id: 'img-phone', kind: 'image', ar: .56, art: 'phone', title: 'Telefon ekranı', ocr: "Akşam İstanbul'da buluşalım mı?", age: 'dün' },

      { id: 'txt-meeting', kind: 'text', body: 'Toplantı Perşembe 14:00, B-204\nGündem: bütçe, yol haritası, raf sesleri', pinned: true, age: '5 dk' },
      { id: 'txt-code', kind: 'text', code: true, body: 'fn main() {\n    println!("Merhaba, pano!");\n}', age: '9 dk' },
      { id: 'txt-address', kind: 'text', body: 'Moda Cad. No: 12 D: 4\nKadıköy / İstanbul', age: '25 dk' },
      { id: 'txt-light', kind: 'text', body: 'IŞIK ŞİDDETİ: 450 lux\nÖlçüm 3, pencere kenarı', age: '1 sa' },
      { id: 'txt-seeds', kind: 'text', body: 'Balkon için çiçek tohumları: lavanta, fesleğen, kadife', age: '2 sa' },
      { id: 'txt-para', kind: 'text', body: 'Kopyaladığın her şey sessizce panoya bantlanır. Saklamak istediklerine raptiye saplarsın; gerisi bir süre sonra kendiliğinden temizlenir. Hepsi bu bilgisayarda kalır.', age: 'dün' },

      { id: 'lnk-repo', kind: 'link', title: 'tauri-apps/tauri · GitHub', domain: 'github.com', fav: ['G', '#24292f'], age: '4 dk' },
      { id: 'lnk-wiki', kind: 'link', title: 'Mantar meşesi — Vikipedi', domain: 'tr.wikipedia.org', fav: ['V', '#5b6670'], thumb: 'linear-gradient(135deg,#7f9c5a,#3f5f2e)', pinned: true, age: '30 dk' },
      { id: 'lnk-video', kind: 'link', title: 'Mantar pano nasıl yapılır? | Hafta sonu atölyesi', domain: 'youtube.com', fav: ['▶', '#3d3d3d'], thumb: 'linear-gradient(135deg,#c98f55,#7a4f2a)', video: true, age: '2 sa' },
      { id: 'lnk-site', kind: 'link', title: 'Mehmet Burak Taşkaya', domain: 'mehmetburaktaskaya.com', fav: ['M', '#1f8a83'], age: 'dün' },
      { id: 'lnk-mdn', kind: 'link', title: 'Web Audio API — MDN Web Docs', domain: 'developer.mozilla.org', fav: ['M', '#1b1b1b'], thumb: 'linear-gradient(135deg,#2b4a7a,#162a4a)', age: '2 gün' },
    ],
  };
})();
