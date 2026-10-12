const byId = (id) => document.getElementById(id);
const paletteColors = {
  latte: { base: "#eff1f5", surface: "#e6e9ef", border: "#ccd0da", text: "#4c4f69", muted: "#6c6f85", accent: "#8839ef", cpu: "#40a02b", ram: "#ea76cb", gpu: "#1e66f5", warm: "#fe640b" },
  frappe: { base: "#303446", surface: "#292c3c", border: "#51576d", text: "#c6d0f5", muted: "#a5adce", accent: "#ca9ee6", cpu: "#a6d189", ram: "#f4b8e4", gpu: "#8caaee", warm: "#ef9f76" },
  macchiato: { base: "#24273a", surface: "#1e2030", border: "#494d64", text: "#cad3f5", muted: "#a5adcb", accent: "#c6a0f6", cpu: "#a6da95", ram: "#f5bde6", gpu: "#8aadf4", warm: "#f5a97f" },
  mocha: { base: "#1e1e2e", surface: "#181825", border: "#45475a", text: "#cdd6f4", muted: "#a6adc8", accent: "#cba6f7", cpu: "#a6e3a1", ram: "#f5c2e7", gpu: "#89b4fa", warm: "#fab387" },
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
  if (element) element.style.width = `${Math.max(0, Math.min(100, Number(value) || 0))}%`;
}

function formatGb(bytes) {
  return Number.isFinite(bytes) ? `${(bytes / (1024 ** 3)).toFixed(1)} GB` : "N/D";
}

function render({ metrics = {}, weather = null, settings = {}, clock = {}, palette = "mocha" }) {
  const colors = paletteColors[palette] || paletteColors.mocha;
  for (const [name, value] of Object.entries(colors)) {
    document.documentElement.style.setProperty(`--${name}`, value);
  }
  const cpu = metrics.cpu_usage_percent;
  const ram = metrics.ram_total_bytes > 0
    ? metrics.ram_used_bytes / metrics.ram_total_bytes * 100
    : null;
  const gpu = metrics.gpu_usage_percent;
  const frequency = metrics.cpu_frequency_mhz == null
    ? "N/D"
    : `${(metrics.cpu_frequency_mhz / 1000).toFixed(1)} GHz`;

  setText("clock", clock.time, "--:--");
  setText("date", clock.date, "");
  setText("computer", metrics.computer_name, "Este computador");
  setText("os", metrics.os_version, "Windows");
  setText("weather-city", weather?.city || settings.city, "São Paulo");
  setText("weather-icon", weather?.icon, "☁");
  setText("weather-temperature", weather ? `${Math.round(weather.temperatureC)}°C` : "—°C");
  setText("weather-description", weather?.description, "CLIMA");
  setText("weather-high", weather ? `↑ ${Math.round(weather.highC)}°` : "↑ —°");
  setText("weather-low", weather ? `↓ ${Math.round(weather.lowC)}°` : "↓ —°");
  setText("weather-humidity", weather ? `◉ ${weather.humidityPercent}%` : "◉ —%");

  setText("cpu-usage", percentage(cpu));
  setText("cpu-frequency", frequency);
  setText("cpu-name", settings.cpuCustom || metrics.cpu_name || "CPU");
  setMeter("cpu-meter", cpu);
  setText("ram-usage", percentage(ram));
  setText("ram-total", metrics.ram_total_bytes > 0
    ? `${formatGb(metrics.ram_used_bytes)} / ${formatGb(metrics.ram_total_bytes)}`
    : "N/D");
  setText("ram-name", settings.ramCustom || "Memória do sistema");
  setMeter("ram-meter", ram);
  setText("gpu-usage", percentage(gpu));
  setText("gpu-temperature", metrics.gpu_temperature_c == null
    ? "TEMP N/D"
    : `TEMP ${metrics.gpu_temperature_c}°C`);
  setText("gpu-name", settings.gpuCustom || metrics.gpu_name || "GPU indisponível");
  setMeter("gpu-meter", gpu);

  const uptime = metrics.uptime_seconds || 0;
  setText("uptime", `${Math.floor(uptime / 86400)}d ${Math.floor(uptime % 86400 / 3600)}h ${Math.floor(uptime % 3600 / 60)}m`);
  setText("upload", `↑ ${((metrics.upload_bytes_per_sec || 0) / 1048576).toFixed(1)} MB/s`);
  setText("download", `↓ ${((metrics.download_bytes_per_sec || 0) / 1048576).toFixed(1)} MB/s`);
  setText("network", metrics.network_interfaces > 0 ? "ATIVA" : "OFFLINE");
  setText("vram", metrics.vram_used_bytes != null && metrics.vram_total_bytes
    ? `${formatGb(metrics.vram_used_bytes)} / ${formatGb(metrics.vram_total_bytes)}`
    : "N/D");
}

window.addEventListener("message", (event) => {
  if (event.source !== window.parent || event.data?.type !== "usbmonitor:state") return;
  render(event.data.payload || {});
});
