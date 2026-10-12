const invoke = window.__TAURI__?.core?.invoke;
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
const historySamples = [];
const metricNames = { cpu: "", gpu: "" };
const defaults = {
  theme: themes[0][0],
  brightness: 80,
  rotation: 0,
  mirror: false,
  accentColor: "#35d7e9",
  cpuCustom: "",
  gpuCustom: "",
  ramCustom: "",
  sensor: "CPU (Tctl/Tdie)",
  network: "Detecção automática",
  city: "",
  startWithWindows: false,
};
let settings = { ...defaults };
let selectedTheme = settings.theme;
let metricsRequestRunning = false;

function formatGb(bytes) {
  return (bytes / (1024 ** 3)).toFixed(1);
}

function formatUptime(seconds) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${days}d ${hours}h ${minutes}m`;
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
  settings.theme = selectedTheme;
  $("#quick-theme").value = selectedTheme;
  document.querySelectorAll(".theme-card").forEach((card) => card.classList.toggle("selected", card.dataset.theme === selectedTheme));
  const theme = themes.find((item) => item[0] === selectedTheme);
  const screen = $(".screen");
  screen.style.backgroundImage = `linear-gradient(180deg,#07101aaa,#07101acc),url("./assets/${encodeURIComponent(theme[1])}")`;
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
    image.style.backgroundImage = `linear-gradient(180deg,transparent,#08101dcc),url("./assets/${encodeURIComponent(file)}")`;
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

function setPercentage(element, value) {
  element.replaceChildren();
  element.append(document.createTextNode(value === null ? "N/D" : Math.round(value).toString()));
  if (value !== null) {
    const suffix = document.createElement("small");
    suffix.textContent = "%";
    element.append(suffix);
  }
}

function updateCustomInfo() {
  $("#cpu-custom-preview").textContent = settings.cpuCustom || metricNames.cpu || "USO DO PROCESSADOR";
  $("#cpu-custom-preview").title = metricNames.cpu || "Processador";
  $("#gpu-custom-preview").textContent = settings.gpuCustom || metricNames.gpu || "USO DA PLACA DE VÍDEO";
  $("#gpu-custom-preview").title = metricNames.gpu || "Placa de vídeo";
  $("#ram-custom-preview").textContent = settings.ramCustom || "MEMÓRIA EM USO";
}

function setProgress(element, value) {
  element.style.width = Number.isFinite(value) ? `${Math.max(0, Math.min(100, value))}%` : "0%";
}

function updateMetrics(metrics) {
  if (!metrics.timestamp_unix_ms) return;
  const cpuUsage = metrics.cpu_usage_percent === null ? null : Math.max(0, Math.min(100, metrics.cpu_usage_percent));
  setPercentage($("#cpu-usage"), cpuUsage);
  setPercentage($("#home-cpu-usage"), cpuUsage);
  if (cpuUsage !== null) setProgress($("#home-cpu-bar"), cpuUsage);

  const frequency = metrics.cpu_frequency_mhz === null ? "N/D" : `${(metrics.cpu_frequency_mhz / 1000).toFixed(1)} GHz`;
  $("#cpu-frequency").textContent = frequency;
  $("#home-cpu-frequency").textContent = frequency;
  metricNames.cpu = metrics.cpu_name || "";
  metricNames.gpu = metrics.gpu_name || "";
  $("#home-cpu-name").textContent = metricNames.cpu || "Nome indisponível";

  const ramUsage = metrics.ram_total_bytes > 0
    ? metrics.ram_used_bytes / metrics.ram_total_bytes * 100
    : null;
  setPercentage($("#ram-usage"), ramUsage);
  if (ramUsage === null) {
    $("#ram-total").textContent = "N/D";
    $("#home-ram-total").textContent = "Memória indisponível";
  } else {
    $("#ram-total").textContent = `${formatGb(metrics.ram_used_bytes)} / ${formatGb(metrics.ram_total_bytes)} GB`;
    $("#home-ram-total").textContent = $("#ram-total").textContent;
    setProgress($("#home-ram-bar"), ramUsage);
    updateBars(".memory-bars", ramUsage);
  }
  setPercentage($("#home-ram-usage"), ramUsage);

  const gpuUsage = metrics.gpu_usage_percent;
  setPercentage($("#gpu-usage"), gpuUsage);
  setPercentage($("#home-gpu-usage"), gpuUsage);
  if (gpuUsage === null) {
    setProgress($("#home-gpu-bar"), NaN);
  } else {
    setProgress($("#home-gpu-bar"), gpuUsage);
    updateBars(".gpu-bars", gpuUsage);
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

  historySamples.push({
    cpu: cpuUsage,
    ram: ramUsage,
    gpu: gpuUsage,
    timestamp: metrics.timestamp_unix_ms,
  });
  if (historySamples.length > 120) historySamples.shift();
  $("#history-range").textContent = `${historySamples.length} amostras · últimos ${Math.max(1, Math.round(historySamples.length / 60))} min`;
  drawHistory();
  $("#metrics-status").textContent = `Dados do PC atualizados às ${new Date(metrics.timestamp_unix_ms).toLocaleTimeString("pt-BR")}`;
  $("#metrics-status").classList.add("metrics-live");
  updateCustomInfo();
  const alert = $("#notice");
  if (alert.classList.contains("metrics-error")) alert.hidden = true;
}

function drawHistory() {
  const canvas = $("#history-chart");
  const context = canvas.getContext("2d");
  const bounds = canvas.getBoundingClientRect();
  if (!context || bounds.width === 0) return;
  const scale = window.devicePixelRatio || 1;
  canvas.width = Math.round(bounds.width * scale);
  canvas.height = Math.round(bounds.height * scale);
  context.setTransform(scale, 0, 0, scale, 0, 0);
  const width = bounds.width;
  const height = bounds.height;
  context.clearRect(0, 0, width, height);
  context.font = "10px Segoe UI, sans-serif";
  context.fillStyle = "#8190a5";
  context.strokeStyle = "#253247";
  context.lineWidth = 1;
  for (let level = 0; level <= 100; level += 25) {
    const y = height - 18 - (height - 28) * level / 100;
    context.beginPath();
    context.moveTo(32, y);
    context.lineTo(width, y);
    context.stroke();
    context.fillText(`${level}%`, 0, y + 3);
  }
  for (const [key, color] of [["cpu", "#44ebba"], ["ram", "#d789f5"], ["gpu", "#38d9fa"]]) {
    context.strokeStyle = color;
    context.lineWidth = 2;
    context.beginPath();
    let started = false;
    historySamples.forEach((sample, index) => {
      if (sample[key] === null) {
        started = false;
        return;
      }
      const x = historySamples.length < 2 ? 32 : 32 + (width - 32) * index / (historySamples.length - 1);
      const y = height - 18 - (height - 28) * Math.max(0, Math.min(100, sample[key])) / 100;
      if (!started) {
        context.moveTo(x, y);
        started = true;
      } else {
        context.lineTo(x, y);
      }
    });
    context.stroke();
  }
}

async function refreshStatus() {
  try {
    updateStatus(await invoke("get_device_status"));
  } catch (error) {
    $("#device-name").textContent = "Serviço local indisponível";
    $("#device-detail").textContent = "Não foi possível consultar o dispositivo.";
    notice(`Falha ao consultar o dispositivo: ${error}`, "error");
  }
}

async function refreshMetrics() {
  if (metricsRequestRunning) return;
  metricsRequestRunning = true;
  try {
    updateMetrics(await invoke("get_metrics"));
  } catch (error) {
    $("#metrics-status").textContent = "MÉTRICAS INDISPONÍVEIS";
    $("#metrics-status").classList.remove("metrics-live");
    notice(`Falha ao ler métricas do computador: ${error}`, "error");
    $("#notice").classList.add("metrics-error");
  } finally {
    metricsRequestRunning = false;
  }
}

async function togglePort() {
  const closing = $("#connect").textContent === "Fechar serial";
  try {
    const data = closing
      ? await invoke("disconnect_serial")
      : await invoke("connect_serial", {
        port: $("#port-select").value,
        baudRate: Number($("#baud").value),
      });
    updateStatus(data);
    notice(closing
      ? "Porta serial fechada."
      : "Porta serial aberta. Nenhum comando foi enviado ao display.");
  } catch (error) {
    notice(String(error), "error");
  }
}

function readSettings() {
  return {
    theme: selectedTheme,
    brightness: Number($("#brightness").value),
    rotation: Number($("#rotation").value),
    mirror: $("#mirror").checked,
    accentColor: $("#bar-color").value,
    cpuCustom: $("#cpu-custom").value.trim(),
    gpuCustom: $("#gpu-custom").value.trim(),
    ramCustom: $("#ram-custom").value.trim(),
    sensor: $("#sensor").value,
    network: $("#network").value,
    city: $("#city").value.trim(),
    startWithWindows: $("#start-with-windows").checked,
  };
}

function applySettings(config) {
  settings = { ...defaults, ...config };
  selectedTheme = settings.theme;
  $("#quick-theme").value = selectedTheme;
  $("#brightness").value = String(settings.brightness);
  $("#brightness-value").value = `${settings.brightness}%`;
  $("#rotation").value = String(settings.rotation);
  $("#mirror").checked = settings.mirror;
  $("#bar-color").value = settings.accentColor;
  $("#cpu-custom").value = settings.cpuCustom;
  $("#gpu-custom").value = settings.gpuCustom;
  $("#ram-custom").value = settings.ramCustom;
  $("#sensor").value = settings.sensor;
  $("#network").value = settings.network;
  $("#city").value = settings.city;
  $("#start-with-windows").checked = settings.startWithWindows;
  document.documentElement.style.setProperty("--accent", settings.accentColor);
  $(".screen").style.transform = settings.rotation === 180 || settings.mirror ? "rotate(180deg)" : "";
  chooseTheme(selectedTheme);
  updateCustomInfo();
}

async function restoreHistory() {
  const samples = await invoke("get_history");
  for (const metrics of samples) {
    if (!metrics.timestamp_unix_ms) continue;
    historySamples.push({
      cpu: metrics.cpu_usage_percent,
      ram: metrics.ram_total_bytes > 0
        ? metrics.ram_used_bytes / metrics.ram_total_bytes * 100
        : null,
      gpu: metrics.gpu_usage_percent,
      timestamp: metrics.timestamp_unix_ms,
    });
  }
  if (historySamples.length > 120) historySamples.splice(0, historySamples.length - 120);
  if (historySamples.length) {
    $("#history-range").textContent = `${historySamples.length} amostras · últimos ${Math.max(1, Math.round(historySamples.length / 60))} min`;
    drawHistory();
  }
}

function tick() {
  const now = new Date();
  $("#clock").textContent = new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(now);
  $("#screen-date").textContent = new Intl.DateTimeFormat("pt-BR", {
    weekday: "short",
    day: "2-digit",
    month: "short",
    year: "numeric",
  }).format(now).replaceAll(".", "").toUpperCase();
}

document.querySelectorAll(".nav-item").forEach((el) => el.addEventListener("click", () => setPage(el.dataset.page)));
document.querySelectorAll("[data-go]").forEach((el) => el.addEventListener("click", () => setPage(el.dataset.go)));
$("#quick-theme").addEventListener("change", (event) => chooseTheme(event.target.value));
$("#refresh").addEventListener("click", refreshStatus);
$("#connect").addEventListener("click", togglePort);
$("#brightness").addEventListener("input", (event) => { $("#brightness-value").value = `${event.target.value}%`; });
$("#rotation").addEventListener("change", (event) => {
  $(".screen").style.transform = Number(event.target.value) === 180 || $("#mirror").checked ? "rotate(180deg)" : "";
});
$("#mirror").addEventListener("change", (event) => {
  $(".screen").style.transform = event.target.checked || Number($("#rotation").value) === 180 ? "rotate(180deg)" : "";
});
$("#bar-color").addEventListener("input", (event) => document.documentElement.style.setProperty("--accent", event.target.value));
["cpu-custom", "gpu-custom", "ram-custom"].forEach((id) => $(`#${id}`).addEventListener("input", () => {
  settings.cpuCustom = $("#cpu-custom").value.trim();
  settings.gpuCustom = $("#gpu-custom").value.trim();
  settings.ramCustom = $("#ram-custom").value.trim();
  updateCustomInfo();
}));
$("#search-city").addEventListener("click", () => {
  const city = $("#city").value.trim();
  $("#city-result").textContent = city
    ? `“${city}” anotada; a previsão exige um provedor meteorológico.`
    : "Digite o nome de uma cidade.";
});
$("#save").addEventListener("click", async () => {
  try {
    const saved = await invoke("save_config", { config: readSettings() });
    applySettings(saved);
    notice("Preferências salvas no computador.");
  } catch (error) {
    notice(`Não foi possível salvar as preferências: ${error}`, "error");
  }
});
$("#export-logs").addEventListener("click", async () => {
  try {
    const path = await invoke("export_logs");
    notice(`Histórico CSV exportado para ${path}.`);
  } catch (error) {
    notice(`Não foi possível exportar o histórico: ${error}`, "error");
  }
});
window.addEventListener("resize", drawHistory);

async function start() {
  if (!invoke) {
    notice("A interface precisa ser executada pelo aplicativo UsbMonitorRust.", "error");
    return;
  }
  renderThemes();
  tick();
  try {
    applySettings(await invoke("load_config"));
  } catch (error) {
    notice(`Não foi possível carregar as preferências: ${error}`, "error");
  }
  try {
    await restoreHistory();
  } catch (error) {
    notice(`Não foi possível carregar o histórico: ${error}`, "error");
  }
  refreshStatus();
  refreshMetrics();
  setInterval(refreshStatus, 5000);
  setInterval(refreshMetrics, 1000);
  setInterval(tick, 1000);
}

start();
