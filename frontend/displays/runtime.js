const byId = (id) => document.getElementById(id);
const paletteColors = {
  latte: {
    base: "#eff1f5",
    surface: "#e6e9ef",
    border: "#bcc0cc",
    text: "#1e2030",
    muted: "#5c5f77",
    accent: "#8839ef",
    cpu: "#258619",
    ram: "#d2409c",
    gpu: "#1e66f5",
    warm: "#fe640b",
  },
  frappe: {
    base: "#232634",
    surface: "#292c3c",
    border: "#51576d",
    text: "#f0f4fc",
    muted: "#b5bfe2",
    accent: "#ca9ee6",
    cpu: "#a6d189",
    ram: "#f4b8e4",
    gpu: "#8caaee",
    warm: "#ef9f76",
  },
  macchiato: {
    base: "#181926",
    surface: "#1e2030",
    border: "#494d64",
    text: "#f2f5fd",
    muted: "#b8c0e0",
    accent: "#c6a0f6",
    cpu: "#a6da95",
    ram: "#f5bde6",
    gpu: "#8aadf4",
    warm: "#f5a97f",
  },
  mocha: {
    base: "#11111b",
    surface: "#181825",
    border: "#45475a",
    text: "#f5f5f7",
    muted: "#bac2de",
    accent: "#cba6f7",
    cpu: "#a6e3a1",
    ram: "#f5c2e7",
    gpu: "#89b4fa",
    warm: "#fab387",
  },
};

function setText(id, value, fallback = "N/D") {
  const element = byId(id);
  if (element) element.textContent = value ?? fallback;
}

function percentage(value) {
  return Number.isFinite(value) ? `${Math.round(value)}%` : "N/D";
}

function setMeter(id, value) {
  const element = byId(id);
  if (element) {
    const clamped = Math.max(0, Math.min(100, Number(value) || 0));
    element.style.width = `${clamped}%`;
  }
}

function formatGb(bytes) {
  return Number.isFinite(bytes) ? `${(bytes / (1024 ** 3)).toFixed(1)} GB` : "N/D";
}

function formatNet(bytes) {
  const b = Number(bytes) || 0;
  if (b >= 1048576) return `${(b / 1048576).toFixed(1)}M`;
  if (b >= 1024) return `${Math.round(b / 1024)}K`;
  return "0K";
}

function render({ metrics = {}, weather = null, settings = {}, clock = {}, palette = "mocha" }) {
  const colors = paletteColors[palette] || paletteColors.mocha;
  for (const [name, value] of Object.entries(colors)) {
    document.documentElement.style.setProperty(`--${name}`, value);
  }

  const cpu = metrics.cpu_usage_percent;
  const ram = metrics.ram_total_bytes > 0
    ? (metrics.ram_used_bytes / metrics.ram_total_bytes) * 100
    : null;
  const gpu = metrics.gpu_usage_percent;

  const frequency = metrics.cpu_frequency_mhz == null
    ? "N/D"
    : `${(metrics.cpu_frequency_mhz / 1000).toFixed(1)} GHz`;

  document.documentElement.style.setProperty("--cpu-pct", `${Math.round(cpu || 0)}%`);
  document.documentElement.style.setProperty("--ram-pct", `${Math.round(ram || 0)}%`);
  document.documentElement.style.setProperty("--gpu-pct", `${Math.round(gpu || 0)}%`);

  setText("clock", clock.time, "--:--");
  setText("date", clock.date, "");
  setText("computer", metrics.computer_name, "Este computador");
  setText("os", metrics.os_version, "Windows");

  const rawCity = weather?.city || settings.city || "São Paulo";
  const cleanCity = rawCity.split(",")[0].trim();
  setText("weather-city", cleanCity);
  setText("weather-icon", weather?.icon, "☁");
  setText("weather-temperature", weather ? `${Math.round(weather.temperatureC)}°C` : "—°C");
  setText("weather-description", weather?.description, "CLIMA");
  setText("weather-high", weather ? `↑ ${Math.round(weather.highC)}°` : "↑ —°");
  setText("weather-low", weather ? `↓ ${Math.round(weather.lowC)}°` : "↓ —°");
  setText("weather-humidity", weather ? `◉ ${weather.humidityPercent}%` : "◉ —%");

  setText("cpu-usage", percentage(cpu));
  setText("cpu-frequency", frequency);
  setText("cpu-name", settings.cpuCustom || metrics.cpu_name || "Processador");
  setMeter("cpu-meter", cpu);

  setText("ram-usage", percentage(ram));
  setText("ram-total", metrics.ram_total_bytes > 0
    ? `${(metrics.ram_used_bytes / (1024 ** 3)).toFixed(1)} / ${(metrics.ram_total_bytes / (1024 ** 3)).toFixed(0)} GB`
    : "N/D");
  setText("ram-name", settings.ramCustom || "Memória do sistema");
  setMeter("ram-meter", ram);

  setText("gpu-usage", percentage(gpu));
  setText("gpu-temperature", metrics.gpu_temperature_c == null
    ? "N/D"
    : `${metrics.gpu_temperature_c}°C`);
  setText("gpu-name", settings.gpuCustom || metrics.gpu_name || "GPU indisponível");
  setMeter("gpu-meter", gpu);

  const uptime = metrics.uptime_seconds || 0;
  const days = Math.floor(uptime / 86400);
  const hours = Math.floor((uptime % 86400) / 3600);
  const minutes = Math.floor((uptime % 3600) / 60);
  const uptimeStr = days > 0 ? `${days}d ${hours}h` : `${hours}h ${minutes}m`;
  setText("uptime", uptimeStr);

  const upSpeed = formatNet(metrics.upload_bytes_per_sec);
  const downSpeed = formatNet(metrics.download_bytes_per_sec);
  setText("upload", `↑ ${upSpeed}`);
  setText("download", `↓ ${downSpeed}`);
  setText("network", metrics.network_interfaces > 0 ? "ONLINE" : "OFFLINE");
  setText("vram", metrics.vram_used_bytes != null && metrics.vram_total_bytes
    ? `${(metrics.vram_used_bytes / (1024 ** 3)).toFixed(1)} / ${(metrics.vram_total_bytes / (1024 ** 3)).toFixed(0)} GB`
    : "N/D");
}

window.addEventListener("message", (event) => {
  if (event.source !== window.parent || event.data?.type !== "usbmonitor:state") return;
  render(event.data.payload || {});
});
