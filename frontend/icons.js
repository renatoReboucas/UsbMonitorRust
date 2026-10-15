// Ícones Lucide embutidos localmente (lucide-static v0.460.0, licença ISC).
// O aplicativo roda offline: nenhum recurso é buscado na rede em tempo de execução.
// A marcação é o conteúdo interno do SVG 24 x 24 do Lucide; o envelope (viewBox,
// stroke, tamanho) é criado por iconMarkup/hydrateIcons.
const LUCIDE_ICONS = {
  "monitor":
    "<rect width=\"20\" height=\"14\" x=\"2\" y=\"3\" rx=\"2\" /><line x1=\"8\" x2=\"16\" y1=\"21\" y2=\"21\" /><line x1=\"12\" x2=\"12\" y1=\"17\" y2=\"21\" />",
  "layout-dashboard":
    "<rect width=\"7\" height=\"9\" x=\"3\" y=\"3\" rx=\"1\" /><rect width=\"7\" height=\"5\" x=\"14\" y=\"3\" rx=\"1\" /><rect width=\"7\" height=\"9\" x=\"14\" y=\"12\" rx=\"1\" /><rect width=\"7\" height=\"5\" x=\"3\" y=\"16\" rx=\"1\" />",
  "layout-grid":
    "<rect width=\"7\" height=\"7\" x=\"3\" y=\"3\" rx=\"1\" /><rect width=\"7\" height=\"7\" x=\"14\" y=\"3\" rx=\"1\" /><rect width=\"7\" height=\"7\" x=\"14\" y=\"14\" rx=\"1\" /><rect width=\"7\" height=\"7\" x=\"3\" y=\"14\" rx=\"1\" />",
  "settings":
    "<path d=\"M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z\" /><circle cx=\"12\" cy=\"12\" r=\"3\" />",
  "refresh-cw":
    "<path d=\"M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8\" /><path d=\"M21 3v5h-5\" /><path d=\"M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16\" /><path d=\"M8 16H3v5\" />",
  "play":
    "<polygon points=\"6 3 20 12 6 21 6 3\" />",
  "pause":
    "<rect x=\"14\" y=\"4\" width=\"4\" height=\"16\" rx=\"1\" /><rect x=\"6\" y=\"4\" width=\"4\" height=\"16\" rx=\"1\" />",
  "download":
    "<path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\" /><polyline points=\"7 10 12 15 17 10\" /><line x1=\"12\" x2=\"12\" y1=\"15\" y2=\"3\" />",
  "arrow-right":
    "<path d=\"M5 12h14\" /><path d=\"m12 5 7 7-7 7\" />",
  "sun":
    "<circle cx=\"12\" cy=\"12\" r=\"4\" /><path d=\"M12 2v2\" /><path d=\"M12 20v2\" /><path d=\"m4.93 4.93 1.41 1.41\" /><path d=\"m17.66 17.66 1.41 1.41\" /><path d=\"M2 12h2\" /><path d=\"M20 12h2\" /><path d=\"m6.34 17.66-1.41 1.41\" /><path d=\"m19.07 4.93-1.41 1.41\" />",
  "rotate-cw":
    "<path d=\"M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8\" /><path d=\"M21 3v5h-5\" />",
  "usb":
    "<circle cx=\"10\" cy=\"7\" r=\"1\" /><circle cx=\"4\" cy=\"20\" r=\"1\" /><path d=\"M4.7 19.3 19 5\" /><path d=\"m21 3-3 1 2 2Z\" /><path d=\"M9.26 7.68 5 12l2 5\" /><path d=\"m10 14 5 2 3.5-3.5\" /><path d=\"m18 12 1-1 1 1-1 1Z\" />",
  "activity":
    "<path d=\"M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2\" />",
  "sliders-horizontal":
    "<line x1=\"21\" x2=\"14\" y1=\"4\" y2=\"4\" /><line x1=\"10\" x2=\"3\" y1=\"4\" y2=\"4\" /><line x1=\"21\" x2=\"12\" y1=\"12\" y2=\"12\" /><line x1=\"8\" x2=\"3\" y1=\"12\" y2=\"12\" /><line x1=\"21\" x2=\"16\" y1=\"20\" y2=\"20\" /><line x1=\"12\" x2=\"3\" y1=\"20\" y2=\"20\" /><line x1=\"14\" x2=\"14\" y1=\"2\" y2=\"6\" /><line x1=\"8\" x2=\"8\" y1=\"10\" y2=\"14\" /><line x1=\"16\" x2=\"16\" y1=\"18\" y2=\"22\" />",
  "cpu":
    "<rect width=\"16\" height=\"16\" x=\"4\" y=\"4\" rx=\"2\" /><rect width=\"6\" height=\"6\" x=\"9\" y=\"9\" rx=\"1\" /><path d=\"M15 2v2\" /><path d=\"M15 20v2\" /><path d=\"M2 15h2\" /><path d=\"M2 9h2\" /><path d=\"M20 15h2\" /><path d=\"M20 9h2\" /><path d=\"M9 2v2\" /><path d=\"M9 20v2\" />",
  "memory-stick":
    "<path d=\"M6 19v-3\" /><path d=\"M10 19v-3\" /><path d=\"M14 19v-3\" /><path d=\"M18 19v-3\" /><path d=\"M8 11V9\" /><path d=\"M16 11V9\" /><path d=\"M12 11V9\" /><path d=\"M2 15h20\" /><path d=\"M2 7a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v1.1a2 2 0 0 0 0 3.837V17a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-5.1a2 2 0 0 0 0-3.837Z\" />",
  "microchip":
    "<path d=\"M18 12h2\" /><path d=\"M18 16h2\" /><path d=\"M18 20h2\" /><path d=\"M18 4h2\" /><path d=\"M18 8h2\" /><path d=\"M4 12h2\" /><path d=\"M4 16h2\" /><path d=\"M4 20h2\" /><path d=\"M4 4h2\" /><path d=\"M4 8h2\" /><path d=\"M8 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2h-1.5c-.276 0-.494.227-.562.495a2 2 0 0 1-3.876 0C9.994 2.227 9.776 2 9.5 2z\" />",
  "arrow-up":
    "<path d=\"m5 12 7-7 7 7\" /><path d=\"M12 19V5\" />",
  "arrow-down":
    "<path d=\"M12 5v14\" /><path d=\"m19 12-7 7-7-7\" />",
  "droplet":
    "<path d=\"M12 22a7 7 0 0 0 7-7c0-2-1-3.9-3-5.5s-3.5-4-4-6.5c-.5 2.5-2 4.9-4 6.5C6 11.1 5 13 5 15a7 7 0 0 0 7 7z\" />",
  "cloud-sun":
    "<path d=\"M12 2v2\" /><path d=\"m4.93 4.93 1.41 1.41\" /><path d=\"M20 12h2\" /><path d=\"m19.07 4.93-1.41 1.41\" /><path d=\"M15.947 12.65a4 4 0 0 0-5.925-4.128\" /><path d=\"M13 22H7a5 5 0 1 1 4.9-6H13a3 3 0 0 1 0 6Z\" />",
  "cloud":
    "<path d=\"M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z\" />",
  "cloud-fog":
    "<path d=\"M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242\" /><path d=\"M16 17H7\" /><path d=\"M17 21H9\" />",
  "cloud-drizzle":
    "<path d=\"M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242\" /><path d=\"M8 19v1\" /><path d=\"M8 14v1\" /><path d=\"M16 19v1\" /><path d=\"M16 14v1\" /><path d=\"M12 21v1\" /><path d=\"M12 16v1\" />",
  "cloud-rain":
    "<path d=\"M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242\" /><path d=\"M16 14v6\" /><path d=\"M8 14v6\" /><path d=\"M12 16v6\" />",
  "snowflake":
    "<line x1=\"2\" x2=\"22\" y1=\"12\" y2=\"12\" /><line x1=\"12\" x2=\"12\" y1=\"2\" y2=\"22\" /><path d=\"m20 16-4-4 4-4\" /><path d=\"m4 8 4 4-4 4\" /><path d=\"m16 4-4 4-4-4\" /><path d=\"m8 20 4-4 4 4\" />",
  "cloud-lightning":
    "<path d=\"M6 16.326A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 .5 8.973\" /><path d=\"m13 12-3 5h4l-3 5\" />",
  "terminal":
    "<polyline points=\"4 17 10 11 4 5\" /><line x1=\"12\" x2=\"20\" y1=\"19\" y2=\"19\" />",
  "square":
    "<rect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\" />",
  "x":
    "<path d=\"M18 6 6 18\" /><path d=\"m6 6 12 12\" />",
  "loader":
    "<path d=\"M12 2v4\" /><path d=\"m16.2 7.8 2.9-2.9\" /><path d=\"M18 12h4\" /><path d=\"m16.2 16.2 2.9 2.9\" /><path d=\"M12 18v4\" /><path d=\"m4.9 19.1 2.9-2.9\" /><path d=\"M2 12h4\" /><path d=\"m4.9 4.9 2.9 2.9\" />",
};

const ICON_NAMES = Object.keys(LUCIDE_ICONS);

/// Marcação <svg> completa para um ícone, ou string vazia quando o nome não existe.
function iconMarkup(name) {
  const body = LUCIDE_ICONS[name];
  if (!body) return "";
  return `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${body}</svg>`;
}

/// Reaproveita o elemento que carrega data-lucide e injeta o SVG dentro dele,
/// preservando classes e atributos do host (assim o CSS continua valendo).
function hydrateIcons(root) {
  const scope = root || document;
  for (const host of scope.querySelectorAll("[data-lucide]")) {
    if (host.dataset.iconReady === "true") continue;
    const markup = iconMarkup(host.dataset.lucide);
    if (!markup) continue;
    host.insertAdjacentHTML("afterbegin", markup);
    host.dataset.iconReady = "true";
  }
}

/// Substitui o conteúdo de um elemento pelo ícone informado (para valores que
/// mudam em tempo de execução, como o clima e as setas de rede).
function setIcon(element, name) {
  if (!element) return;
  const markup = iconMarkup(name);
  if (!markup) return;
  element.innerHTML = markup;
}

if (typeof window !== "undefined") {
  window.hydrateIcons = hydrateIcons;
  window.iconMarkup = iconMarkup;
  window.setIcon = setIcon;
}

// O script é carregado com `defer`, então o documento já está analisado quando
// este ponto é alcançado e os marcadores `data-lucide` podem ser hidratados de
// imediato (a checagem cobre o caso de uso sem `defer`).
if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => hydrateIcons());
  } else {
    hydrateIcons();
  }
}
