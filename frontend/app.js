const invoke = window.__TAURI__?.core?.invoke;
const templateIdPattern = /^[a-z0-9-]{1,48}$/;
const palettes = {
  latte: {
    label: "Latte",
    base: "#eff1f5", mantle: "#e6e9ef", crust: "#dce0e8", surface0: "#ccd0da",
    text: "#4c4f69", subtext0: "#6c6f85", mauve: "#8839ef", green: "#40a02b",
    pink: "#ea76cb", blue: "#1e66f5", peach: "#fe640b", red: "#d20f39",
  },
  frappe: {
    label: "Frappé",
    base: "#303446", mantle: "#292c3c", crust: "#232634", surface0: "#414559",
    text: "#c6d0f5", subtext0: "#a5adce", mauve: "#ca9ee6", green: "#a6d189",
    pink: "#f4b8e4", blue: "#8caaee", peach: "#ef9f76", red: "#e78284",
  },
  macchiato: {
    label: "Macchiato",
    base: "#24273a", mantle: "#1e2030", crust: "#181926", surface0: "#363a4f",
    text: "#cad3f5", subtext0: "#a5adcb", mauve: "#c6a0f6", green: "#a6da95",
    pink: "#f5bde6", blue: "#8aadf4", peach: "#f5a97f", red: "#ed8796",
  },
  mocha: {
    label: "Mocha",
    base: "#1e1e2e", mantle: "#181825", crust: "#11111b", surface0: "#313244",
    text: "#cdd6f4", subtext0: "#a6adc8", mauve: "#cba6f7", green: "#a6e3a1",
    pink: "#f5c2e7", blue: "#89b4fa", peach: "#fab387", red: "#f38ba8",
  },
};
const $ = (selector) => document.querySelector(selector);
const historySamples = [];
const metricNames = { cpu: "", gpu: "" };
const defaults = {
  displayTemplate: "reference",
  brightness: 80,
  rotation: 0,
  mirror: false,
  accentColor: "#35d7e9",
  cpuCustom: "",
  gpuCustom: "",
  ramCustom: "",
  sensor: "CPU (Tctl/Tdie)",
  network: "Detecção automática",
  city: "São Paulo, São Paulo",
  palette: "mocha",
  startWithWindows: false,
};
let settings = { ...defaults };
let displayTemplates = [];
let selectedTemplate = settings.displayTemplate;
let metricsRequestRunning = false;
let weatherRequestRunning = false;
let queuedWeatherCity = null;
let latestMetrics = null;
let currentWeather = null;
let displayClock = { time: "--:--", date: "" };
let templateLoadRequest = 0;

function formatGb(bytes) {
  return (bytes / (1024 ** 3)).toFixed(1);
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

function sendDisplayState() {
  const frame = $("#display-frame");
  if (!frame?.contentWindow) return;
  frame.contentWindow.postMessage({
    type: "usbmonitor:state",
    payload: {
      metrics: latestMetrics || {},
      weather: currentWeather,
      settings,
      clock: displayClock,
      palette: settings.palette,
    },
  }, window.location.origin);
}

async function chooseTheme(id) {
  const template = displayTemplates.find((item) => item.id === id);
  selectedTemplate = template ? template.id : displayTemplates[0]?.id;
  if (!selectedTemplate) return;
  settings.displayTemplate = selectedTemplate;
  $("#quick-theme").value = selectedTemplate;
  document.querySelectorAll(".theme-card").forEach((card) => card.classList.toggle("selected", card.dataset.template === selectedTemplate));
  const frame = $("#display-frame");
  if (frame.dataset.template !== selectedTemplate) {
    const request = ++templateLoadRequest;
    const path = `./displays/${encodeURIComponent(selectedTemplate)}.html`;
    try {
      const response = await fetch(path);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      if (request !== templateLoadRequest) return;
      frame.dataset.template = selectedTemplate;
      frame.src = path;
    } catch (error) {
      notice(`Não foi possível carregar o layout “${template.name}”: ${error}`, "error");
      return;
    }
  }
  sendDisplayState();
}

function applyPalette(name) {
  const selected = Object.hasOwn(palettes, name) ? name : "mocha";
  const palette = palettes[selected];
  settings.palette = selected;
  document.documentElement.dataset.palette = selected;
  for (const [variable, value] of Object.entries({
    "--bg": palette.base,
    "--panel": palette.mantle,
    "--border": palette.surface0,
    "--text": palette.text,
    "--muted": palette.subtext0,
    "--accent": settings.accentColor || palette.mauve,
    "--green": palette.green,
    "--screen-base": palette.base,
    "--screen-surface": palette.mantle,
    "--screen-border": palette.blue,
    "--screen-blue": palette.blue,
    "--screen-text": palette.text,
    "--screen-muted": palette.subtext0,
    "--screen-cpu": palette.green,
    "--screen-ram": palette.pink,
    "--screen-gpu": palette.blue,
    "--screen-warm": palette.peach,
  })) {
    document.documentElement.style.setProperty(variable, value);
  }
  $("#palette").value = selected;
  $("#bar-color").value = settings.accentColor || palette.mauve;
  sendDisplayState();
}

async function renderThemes() {
  const response = await fetch("./displays/templates.json");
  if (!response.ok) throw new Error(`Não foi possível carregar a lista de layouts (${response.status}).`);
  const templates = await response.json();
  if (!Array.isArray(templates) || templates.length === 0) {
    throw new Error("A lista de layouts HTML está vazia ou inválida.");
  }
  const ids = new Set();
  displayTemplates = templates.map((template) => {
    if (typeof template.id !== "string" || !templateIdPattern.test(template.id) || ids.has(template.id)
        || typeof template.name !== "string" || typeof template.description !== "string") {
      throw new Error("A lista de layouts contém uma entrada inválida ou duplicada.");
    }
    ids.add(template.id);
    return template;
  });
  $("#quick-theme").replaceChildren(...displayTemplates.map(({ id, name }) => new Option(name, id)));
  const grid = $("#theme-grid");
  grid.replaceChildren();
  for (const template of displayTemplates) {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "theme-card";
    card.dataset.template = template.id;
    const image = document.createElement("span");
    image.className = "theme-image";
    image.classList.add(`template-${template.id}`);
    const badge = document.createElement("span");
    badge.className = "badge";
    badge.textContent = "HTML · 800 × 450";
    image.append(badge);
    const title = document.createElement("strong");
    title.textContent = template.name;
    const hint = document.createElement("small");
    hint.textContent = template.description;
    card.append(image, title, hint);
    card.addEventListener("click", () => chooseTheme(template.id));
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
  $("#connect").textContent = data.port_open ? "Fechar porta COM" : "Abrir display (COM)";
  const select = $("#port-select");
  select.replaceChildren(new Option(data.port || "Dispositivo não encontrado", data.port || ""));
  select.disabled = !connected || data.port_open;
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
  sendDisplayState();
}

function setProgress(element, value) {
  element.style.width = Number.isFinite(value) ? `${Math.max(0, Math.min(100, value))}%` : "0%";
}

function updateMetrics(metrics) {
  if (!metrics.timestamp_unix_ms) return;
  latestMetrics = metrics;
  const cpuUsage = metrics.cpu_usage_percent === null ? null : Math.max(0, Math.min(100, metrics.cpu_usage_percent));
  setPercentage($("#home-cpu-usage"), cpuUsage);
  if (cpuUsage !== null) setProgress($("#home-cpu-bar"), cpuUsage);

  const frequency = metrics.cpu_frequency_mhz === null ? "N/D" : `${(metrics.cpu_frequency_mhz / 1000).toFixed(1)} GHz`;
  $("#home-cpu-frequency").textContent = frequency;
  metricNames.cpu = metrics.cpu_name || "";
  metricNames.gpu = metrics.gpu_name || "";
  $("#home-cpu-name").textContent = metricNames.cpu || "Nome indisponível";

  const ramUsage = metrics.ram_total_bytes > 0
    ? metrics.ram_used_bytes / metrics.ram_total_bytes * 100
    : null;
  if (ramUsage === null) {
    $("#home-ram-total").textContent = "Memória indisponível";
  } else {
    $("#home-ram-total").textContent = `${formatGb(metrics.ram_used_bytes)} / ${formatGb(metrics.ram_total_bytes)} GB`;
    setProgress($("#home-ram-bar"), ramUsage);
  }
  setPercentage($("#home-ram-usage"), ramUsage);

  const gpuUsage = metrics.gpu_usage_percent;
  setPercentage($("#home-gpu-usage"), gpuUsage);
  if (gpuUsage === null) {
    setProgress($("#home-gpu-bar"), NaN);
  } else {
    setProgress($("#home-gpu-bar"), gpuUsage);
  }

  const gpuTemperature = metrics.gpu_temperature_c === null ? "N/D" : `${metrics.gpu_temperature_c}°C`;
  $("#home-gpu-temperature").textContent = gpuTemperature;
  $("#home-gpu-name").textContent = metricNames.gpu || "GPU NVIDIA indisponível";
  $("#home-vram-total").textContent = metrics.vram_used_bytes !== null && metrics.vram_total_bytes
    ? `VRAM: ${formatGb(metrics.vram_used_bytes)} / ${formatGb(metrics.vram_total_bytes)} GB`
    : "VRAM: indisponível";

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

async function refreshWeather(city = settings.city) {
  queuedWeatherCity = city.trim();
  if (weatherRequestRunning) return;
  weatherRequestRunning = true;
  try {
    while (queuedWeatherCity !== null) {
      const requestedCity = queuedWeatherCity;
      queuedWeatherCity = null;
      if (!requestedCity) continue;
      try {
        const weather = await invoke("get_weather", { city: requestedCity });
        currentWeather = weather;
        $("#weather-city-label").textContent = weather.city.toLocaleUpperCase("pt-BR");
        $("#city-result").textContent = `Atualizado às ${new Date(weather.observedAtUnixMs).toLocaleTimeString("pt-BR")} · Open-Meteo`;
        sendDisplayState();
      } catch (error) {
        $("#city-result").textContent = `Falha ao consultar o clima: ${error}`;
        notice(`Não foi possível obter o clima para “${requestedCity}”: ${error}`, "error");
      }
    }
  } finally {
    weatherRequestRunning = false;
    if (queuedWeatherCity !== null) refreshWeather(queuedWeatherCity);
  }
}

async function togglePort() {
  const closing = $("#connect").textContent === "Fechar porta COM";
  try {
    const data = closing
      ? await invoke("disconnect_serial")
      : await invoke("connect_serial", {
        port: $("#port-select").value,
        baudRate: Number($("#baud").value),
      });
    updateStatus(data);
    notice(closing
      ? "Porta COM fechada."
      : "Porta COM aberta. Isso não envia dados nem acende a tela física; o protocolo ainda não foi confirmado.");
  } catch (error) {
    notice(String(error), "error");
  }
}

function readSettings() {
  return {
    displayTemplate: selectedTemplate,
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
    palette: $("#palette").value,
    startWithWindows: $("#start-with-windows").checked,
  };
}

function applySettings(config) {
  settings = { ...defaults, ...config };
  selectedTemplate = displayTemplates.some((template) => template.id === settings.displayTemplate)
    ? settings.displayTemplate
    : displayTemplates[0].id;
  settings.displayTemplate = selectedTemplate;
  $("#quick-theme").value = selectedTemplate;
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
  $("#palette").value = settings.palette;
  $("#start-with-windows").checked = settings.startWithWindows;
  applyPalette(settings.palette);
  document.documentElement.style.setProperty("--accent", settings.accentColor);
  $(".screen").style.transform = settings.rotation === 180 || settings.mirror ? "rotate(180deg)" : "";
  $(".screen").style.filter = `brightness(${settings.brightness}%)`;
  chooseTheme(selectedTemplate);
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
  displayClock.time = new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(now);
  displayClock.date = new Intl.DateTimeFormat("pt-BR", {
    weekday: "short",
    day: "2-digit",
    month: "short",
    year: "numeric",
  }).format(now).replaceAll(".", "").toUpperCase();
  sendDisplayState();
}

$("#display-frame").addEventListener("load", sendDisplayState);
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
$("#palette").addEventListener("change", (event) => {
  settings.accentColor = palettes[event.target.value]?.mauve || palettes.mocha.mauve;
  applyPalette(event.target.value);
});
["cpu-custom", "gpu-custom", "ram-custom"].forEach((id) => $(`#${id}`).addEventListener("input", () => {
  settings.cpuCustom = $("#cpu-custom").value.trim();
  settings.gpuCustom = $("#gpu-custom").value.trim();
  settings.ramCustom = $("#ram-custom").value.trim();
  updateCustomInfo();
}));
$("#search-city").addEventListener("click", () => refreshWeather($("#city").value.trim()));
$("#save").addEventListener("click", async () => {
  try {
    const saved = await invoke("save_config", { config: readSettings() });
    applySettings(saved);
    notice("Preferências salvas no computador.");
    refreshWeather(saved.city);
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
  try {
    await renderThemes();
  } catch (error) {
    notice(`Não foi possível carregar os layouts HTML: ${error}`, "error");
    return;
  }
  tick();
  try {
    applySettings(await invoke("load_config"));
  } catch (error) {
    applySettings(defaults);
    notice(`Não foi possível carregar as preferências: ${error}`, "error");
  }
  try {
    await restoreHistory();
  } catch (error) {
    notice(`Não foi possível carregar o histórico: ${error}`, "error");
  }
  refreshStatus();
  refreshMetrics();
  refreshWeather(settings.city);
  setInterval(refreshStatus, 5000);
  setInterval(refreshMetrics, 1000);
  setInterval(() => refreshWeather(settings.city), 15 * 60 * 1000);
  setInterval(tick, 1000);
}

start();
