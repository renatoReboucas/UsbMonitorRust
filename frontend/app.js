const themes = [
  ["3.5inchTheme1", "deep space.png", "INCLUSO"],
  ["3.5inchTheme2", "deep blue.png", "INCLUSO"],
  ["MagicBlue", "magic blue intel.png", "TECH"],
  ["LandscapeTechnology", "tech intel.png", "TECH"],
  ["LandscapeMagicBlue", "landscape magic blue intel.png", "PAISAGEM"],
  ["LandscapeDeepSpace", "deep space.png", "PAISAGEM"],
  ["AORUS", "tech amd.png", "TECH"],
  ["LandscapeSimpleWhite", "wihte.png", "MINIMAL"],
];
const $ = (selector) => document.querySelector(selector);
let selectedTheme = localStorage.getItem("turing-theme") || themes[0][0];
const cpuHistory = [];
const metricNames = { cpu: "", gpu: "" };

function formatGb(bytes) {
  return (bytes / (1024 ** 3)).toFixed(1);
}

function formatUptime(seconds) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${days}d ${hours}h ${minutes}m`;
}

function setProgress(element, value) {
  const valid = Number.isFinite(value);
  element.style.width = valid ? `${Math.max(0, Math.min(100, value))}%` : "0%";
}

function notice(message, type = "") {
  const el = $("#notice");
  el.textContent = message;
  el.className = `notice ${type}`;
  el.hidden = false;
  clearTimeout(notice.timer);
  notice.timer = setTimeout(() => { el.hidden = true; }, 6500);
}

function setPage(name) {
  document.querySelectorAll(".page").forEach((el) => el.classList.toggle("active", el.id === `page-${name}`));
  document.querySelectorAll(".nav-item").forEach((el) => el.classList.toggle("active", el.dataset.page === name));
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function chooseTheme(name) {
  selectedTheme = themes.some((theme) => theme[0] === name) ? name : themes[0][0];
  localStorage.setItem("turing-theme", selectedTheme);
  $("#quick-theme").value = selectedTheme;
  document.querySelectorAll(".theme-card").forEach((card) => card.classList.toggle("selected", card.dataset.theme === selectedTheme));
  const theme = themes.find((item) => item[0] === selectedTheme);
  const screen = $(".screen");
  $("#screen-theme").textContent = theme[0].toUpperCase();
  screen.style.backgroundImage = `linear-gradient(180deg,#07101aaa,#07101acc),url("/assets/${encodeURIComponent(theme[1])}")`;
  screen.style.backgroundSize = "cover";
  screen.style.backgroundPosition = "center";
}

function renderThemes() {
  $("#quick-theme").replaceChildren(...themes.map(([name]) => new Option(name, name)));
  const grid = $("#theme-grid");
  for (const [name, file, category] of themes) {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "theme-card";
    card.dataset.theme = name;
    const image = document.createElement("span");
    image.className = "theme-image";
    image.style.backgroundImage = `linear-gradient(180deg,transparent,#08101dcc),url("/assets/${encodeURIComponent(file)}")`;
    const badge = document.createElement("span");
    badge.className = "badge";
    badge.textContent = category;
    image.append(badge);
    const title = document.createElement("strong");
    title.textContent = name;
    const hint = document.createElement("small");
    hint.textContent = "Selecionar tema";
    card.append(image, title, hint);
    card.addEventListener("click", () => chooseTheme(name));
    grid.append(card);
  }
  chooseTheme(selectedTheme);
}

function updateStatus(data) {
  const connected = data.connected;
  $("#side-dot").classList.toggle("online", connected);
  $("#main-dot").classList.toggle("online", connected);
  $("#side-status").textContent = data.port_open ? "Porta serial aberta" : connected ? "Display detectado" : "Display não encontrado";
  $("#side-port").textContent = data.port || "USB · VID_1A86";
  $("#main-status").textContent = data.port_open ? "Porta serial aberta" : connected ? "Conectado ao USB" : "Aguardando display";
  $("#device-name").textContent = data.description || (connected ? "Display Turing 3,5″" : "Display não encontrado");
  $("#device-detail").textContent = data.port ? `${data.port} · VID_1A86 · PID_5722` : "Conecte o display USB (VID_1A86 · PID_5722)";
  $("#connect").disabled = !connected && !data.port_open;
  $("#connect").textContent = data.port_open ? "Fechar serial" : "Abrir serial";
  const select = $("#port-select");
  select.replaceChildren(new Option(data.port || "Dispositivo não encontrado", data.port || ""));
  select.disabled = !connected || data.port_open;
}

function updateBars(selector, percentage) {
  const bars = document.querySelectorAll(`${selector} i`);
  const activeBars = Math.round(Math.max(0, Math.min(100, percentage)) / 100 * bars.length);
  bars.forEach((bar, index) => bar.classList.toggle("active", index < activeBars));
}

function updateCustomInfo() {
  const customCpu = $("#cpu-custom").value.trim();
  const customGpu = $("#gpu-custom").value.trim();
  const customRam = $("#ram-custom").value.trim();
  $("#cpu-custom-preview").textContent = customCpu || metricNames.cpu || "USO DO PROCESSADOR";
  $("#cpu-custom-preview").title = metricNames.cpu || "Processador";
  $("#gpu-custom-preview").textContent = customGpu || metricNames.gpu || "USO DA PLACA DE VÍDEO";
  $("#gpu-custom-preview").title = metricNames.gpu || "Placa de vídeo";
  $("#ram-custom-preview").textContent = customRam || "MEMÓRIA EM USO";
}

function updateMetrics(metrics) {
  if (metrics.cpu_usage_percent !== null) {
    const usage = Math.max(0, Math.min(100, metrics.cpu_usage_percent));
    $("#cpu-usage").innerHTML = `${usage.toFixed(0)}<small>%</small>`;
    $("#home-cpu-usage").textContent = `${usage.toFixed(0)}%`;
    setProgress($("#home-cpu-bar"), usage);
    cpuHistory.push(usage);
    if (cpuHistory.length > 30) cpuHistory.shift();
    const line = cpuHistory.map((sample, index) => {
      const x = cpuHistory.length < 2 ? 0 : index * 120 / (cpuHistory.length - 1);
      return `${index === 0 ? "M" : "L"}${x.toFixed(1)} ${(27 - sample * 0.24).toFixed(1)}`;
    });
    $("#cpu-history").setAttribute("d", line.join(" "));
  } else {
    $("#cpu-usage").innerHTML = "N/D<small></small>";
    $("#home-cpu-usage").textContent = "N/D";
  }

  if (metrics.cpu_frequency_mhz !== null) {
    const frequency = `${(metrics.cpu_frequency_mhz / 1000).toFixed(1)} GHz`;
    $("#cpu-frequency").textContent = frequency;
    $("#home-cpu-frequency").textContent = frequency;
  }
  metricNames.cpu = metrics.cpu_name || "";
  metricNames.gpu = metrics.gpu_name || "";
  $("#home-cpu-name").textContent = metricNames.cpu || "Nome indisponível";

  if (metrics.ram_total_bytes > 0) {
    const usage = metrics.ram_used_bytes / metrics.ram_total_bytes * 100;
    $("#ram-usage").innerHTML = `${usage.toFixed(0)}<small>%</small>`;
    $("#ram-total").textContent = `${formatGb(metrics.ram_used_bytes)} / ${formatGb(metrics.ram_total_bytes)} GB`;
    $("#home-ram-usage").textContent = `${usage.toFixed(0)}%`;
    $("#home-ram-total").textContent = `${formatGb(metrics.ram_used_bytes)} / ${formatGb(metrics.ram_total_bytes)} GB`;
    setProgress($("#home-ram-bar"), usage);
    updateBars(".memory-bars", usage);
  } else {
    $("#ram-usage").innerHTML = "N/D<small></small>";
    $("#ram-total").textContent = "N/D";
    $("#home-ram-usage").textContent = "N/D";
    $("#home-ram-total").textContent = "Memória indisponível";
  }

  if (metrics.gpu_usage_percent !== null) {
    $("#gpu-usage").innerHTML = `${metrics.gpu_usage_percent}<small>%</small>`;
    $("#home-gpu-usage").textContent = `${metrics.gpu_usage_percent}%`;
    setProgress($("#home-gpu-bar"), metrics.gpu_usage_percent);
    updateBars(".gpu-bars", metrics.gpu_usage_percent);
  } else {
    $("#gpu-usage").innerHTML = "N/D<small></small>";
    $("#home-gpu-usage").textContent = "N/D";
    setProgress($("#home-gpu-bar"), NaN);
  }

  const gpuTemperature = metrics.gpu_temperature_c === null ? "N/D" : `${metrics.gpu_temperature_c}°C`;
  $("#gpu-temperature").textContent = `TEMP ${gpuTemperature}`;
  $("#footer-gpu-temperature").textContent = gpuTemperature;
  $("#home-gpu-temperature").textContent = gpuTemperature;
  $("#home-gpu-name").textContent = metricNames.gpu || "GPU NVIDIA indisponível";
  $("#vram").textContent = metrics.vram_used_bytes !== null && metrics.vram_total_bytes
    ? `${formatGb(metrics.vram_used_bytes)}/${formatGb(metrics.vram_total_bytes)} GB`
    : "N/D";
  $("#home-vram-total").textContent = metrics.vram_used_bytes !== null && metrics.vram_total_bytes
    ? `VRAM: ${formatGb(metrics.vram_used_bytes)} / ${formatGb(metrics.vram_total_bytes)} GB`
    : "VRAM: indisponível";
  $("#uptime").textContent = formatUptime(metrics.uptime_seconds);
  $("#upload-speed").textContent = `⬆ ${(metrics.upload_bytes_per_sec / 1048576).toFixed(1)} MB/s`;
  $("#download-speed").textContent = `⬇ ${(metrics.download_bytes_per_sec / 1048576).toFixed(1)} MB/s`;
  $("#network-state").textContent = metrics.network_interfaces > 0 ? "ATIVA" : "OFFLINE";
  if (metrics.computer_name) $("#computer-name").textContent = metrics.computer_name;
  if (metrics.os_version) $("#os-version").textContent = metrics.os_version;
  $("#metrics-status").textContent = `Dados do PC atualizados às ${new Date().toLocaleTimeString("pt-BR")}`;
  $("#metrics-status").classList.add("metrics-live");
  updateCustomInfo();
  const alert = $("#notice");
  if (alert.classList.contains("metrics-error")) alert.hidden = true;
}

async function refreshStatus() {
  try {
    const response = await fetch("/api/status", { cache: "no-store" });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || `HTTP ${response.status}`);
    updateStatus(data);
  } catch (error) {
    $("#device-name").textContent = "Serviço Rust indisponível";
    $("#device-detail").textContent = "Inicie o aplicativo Rust e atualize a página.";
    notice(`Não foi possível consultar o dispositivo: ${error.message}`, "error");
  }
}

async function refreshMetrics() {
  try {
    const response = await fetch("/api/metrics", { cache: "no-store" });
    const metrics = await response.json();
    if (!response.ok) throw new Error(metrics.error || `HTTP ${response.status}`);
    updateMetrics(metrics);
  } catch (error) {
    $("#metrics-status").textContent = "MÉTRICAS INDISPONÍVEIS";
    $("#metrics-status").classList.remove("metrics-live");
    notice(`Falha ao ler métricas do computador: ${error.message}`, "error");
    $("#notice").classList.add("metrics-error");
  }
}

async function togglePort() {
  const closing = $("#connect").textContent === "Fechar serial";
  try {
    const response = await fetch(closing ? "/api/disconnect" : "/api/connect", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: closing ? "{}" : JSON.stringify({ port: $("#port-select").value, baud_rate: Number($("#baud").value) }),
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || `HTTP ${response.status}`);
    updateStatus(data);
    notice(closing ? "Porta serial fechada." : "Porta serial aberta. As métricas aparecem independentemente; nenhum comando foi enviado ao display.");
  } catch (error) {
    notice(error.message, "error");
  }
}

document.querySelectorAll(".nav-item").forEach((el) => el.addEventListener("click", () => setPage(el.dataset.page)));
document.querySelectorAll("[data-go]").forEach((el) => el.addEventListener("click", () => setPage(el.dataset.go)));
$("#quick-theme").addEventListener("change", (event) => chooseTheme(event.target.value));
$("#refresh").addEventListener("click", refreshStatus);
$("#connect").addEventListener("click", togglePort);
$("#brightness").addEventListener("input", (event) => { $("#brightness-value").value = `${event.target.value}%`; });
$("#rotation").addEventListener("change", (event) => { $(".screen").style.transform = event.target.value === "180" ? "rotate(180deg)" : ""; });
$("#mirror").addEventListener("change", (event) => { $(".screen").style.transform = event.target.checked ? "rotate(180deg)" : ""; });
$("#bar-color").addEventListener("input", (event) => document.documentElement.style.setProperty("--accent", event.target.value));
["cpu-custom", "gpu-custom", "ram-custom"].forEach((id) => $(`#${id}`).addEventListener("input", updateCustomInfo));
$("#search-city").addEventListener("click", () => {
  const city = $("#city").value.trim();
  $("#city-result").textContent = city ? `“${city}” anotada; busca de previsão indisponível sem um provedor meteorológico.` : "Digite o nome de uma cidade.";
});
$("#save").addEventListener("click", () => {
  for (const id of ["cpu-custom", "gpu-custom", "ram-custom", "sensor", "network", "city", "brightness", "rotation", "bar-color"]) {
    localStorage.setItem(`turing-${id}`, $(`#${id}`).value);
  }
  localStorage.setItem("turing-mirror", String($("#mirror").checked));
  updateCustomInfo();
  notice("Preferências locais salvas neste navegador.");
});
function restoreSettings() {
  for (const id of ["cpu-custom", "gpu-custom", "ram-custom", "sensor", "network", "city", "brightness", "rotation", "bar-color"]) {
    const value = localStorage.getItem(`turing-${id}`);
    if (value !== null) $(`#${id}`).value = value;
  }
  const mirror = localStorage.getItem("turing-mirror");
  if (mirror !== null) $("#mirror").checked = mirror === "true";
  $("#brightness-value").value = `${$("#brightness").value}%`;
  document.documentElement.style.setProperty("--accent", $("#bar-color").value);
  const rotation = $("#rotation").value === "180";
  $(".screen").style.transform = rotation || $("#mirror").checked ? "rotate(180deg)" : "";
  updateCustomInfo();
}
function updateCustomInfo() {
  updateCustomInfo();
}
function tick() {
  const now = new Date();
  $("#clock").textContent = new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(now);
  const date = new Intl.DateTimeFormat("pt-BR", { weekday: "short", day: "2-digit", month: "short", year: "numeric" })
    .format(now).replaceAll(".", "").toUpperCase();
  $("#screen-date").textContent = date;
}
renderThemes();
restoreSettings();
tick();
setInterval(tick, 1000);
refreshStatus();
setInterval(refreshStatus, 5000);
refreshMetrics();
setInterval(refreshMetrics, 1000);
