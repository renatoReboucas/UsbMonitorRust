# Engenharia reversa e estado do projeto

Atualizado em 2026-10-12.

## Escopo e evidência

Esta pasta continha a implementação Rust/Axum anterior, sua interface web e um
executável de desenvolvimento em `target\debug`. A análise desta refatoração
foi feita sobre o código-fonte presente; o programa original de terceiros não
foi executado nem decompilado nesta etapa. As notas históricas abaixo
reproduzem resultados previamente registrados no projeto e não constituem uma
nova análise do binário.

### Implementação Rust substituída

- O backend anterior atendia em `127.0.0.1:47825` por HTTP, com rotas para estado
  USB, métricas e abrir/fechar porta serial.
- A porta serial era filtrada pelo identificador observado `VID_1A86:PID_5722`.
  O código abria a porta, mas não lia nem transmitia bytes.
- `sysinfo` fornecia CPU, memória, frequência, tráfego agregado de rede e
  uptime; GPU NVIDIA era consultada por chamadas NVML carregadas dinamicamente.
- Preferências visuais eram guardadas em `localStorage`; não existiam bandeja,
  inicialização automática ou exportação de logs.
- Foram encontrados dois defeitos no frontend anterior: `updateCustomInfo`
  terminava chamando a si própria recursivamente e a troca de tema tentava
  acessar o elemento inexistente `#screen-theme`.

### Programa original — resultados registrados anteriormente

As anotações anteriores identificavam `UsbMonitor.exe` como PE x86-64/.NET e
registravam uma inspeção estática incompleta. Depois foi instalado o SDK .NET 9
e ILSpy 9.1 no perfil do usuário para decompilar localmente o executável. A
rotina serial recuperada confirma 115200 baud, 8N1, DTR/RTS ativos e ausência
de handshake; o enquadramento de imagem usa o comando 197, com pixels RGB565,
e intervalos de 10 ms antes/depois do payload. A orientação é enviada pelo
comando 121. O binário foi analisado estaticamente; o hardware não foi usado
para validar a transmissão.

O material de referência descrito nas notas incluía 38 artes de tema, imagens
de guia/configuração, opções de rotação, brilho, sensores e inicialização
automática. As artes incluem perfis paisagem e retrato; a interface desta
implementação envia o layout paisagem em 480 × 320. O adaptador observado no
Windows era `COM3`, com `VID_1A86&PID_5722`; o identificador foi usado junto
com os métodos recuperados, não como prova isolada do protocolo.

## Arquitetura atual

- `src-tauri\src\main.rs`: inicialização Tauri, comandos e estado compartilhado.
- `cpu.rs`, `ram.rs`, `gpu.rs`, `telemetry.rs`: coleta local de métricas e
  histórico circular de até 3.600 amostras (uma hora).
- `serial.rs`: enumeração e validação da porta, configuração serial compatível
  com o executável original, rasterização PNG/RGB565 e transmissão de quadros.
- `config.rs`: validação e persistência de preferências JSON no diretório de
  configuração do aplicativo, com arquivo temporário e backup de recuperação.
- `startup.rs`: integração de inicialização automática do Windows.
- `tray.rs`: ícone e menu da bandeja; fechar a janela oculta o app.
- `frontend`: telas HTML/CSS/JavaScript e layouts 480 × 320 carregados em
  `iframe`; os layouts estão em `frontend\displays`, registrados em
  `templates.json`, recebem as métricas via `postMessage` e podem ser enviados
  como quadros RGB565 ao display.
- `weather.rs`: consulta Open-Meteo para geocodificação e previsão atual, com
  cache de 15 minutos e cidade/região editáveis; o padrão é São Paulo, SP.
- `tray.rs` usa o ícone do aplicativo (`icons/32x32.png`) e também atualiza
  CPU, RAM, GPU, CPU TEMP, GPU TEMP, rede e uptime
  no menu/tooltip da bandeja, a cada cinco segundos. A ação “Liberar porta
  serial” desconecta uma porta aberta por esta instância mesmo com a janela
  oculta.
- Catppuccin Latte, Frappé, Macchiato e Mocha, mais a paleta própria Console
  (verde fósforo, definida no tema `console`), são paletas persistidas. Para
  adicionar um layout HTML, crie `<id>.html` em `frontend\displays` e inclua o
  ID e metadados em `templates.json`; `runtime.js` fornece os dados dinâmicos.
  O tema `console` usa a fonte local `frontend\displays\fonts\doto.woff2` em
  todos os textos para manter o visual de terminal sem depender de rede. Na
  captura para o display físico, `app.js` reescreve as URLs de fonte locais como
  data URI (para o `@font-face` resolver dentro do `foreignObject` do SVG) e
  desativa transições antes de ler os estilos, senão os medidores saem vazios.

## Limites ainda abertos

- O protocolo foi confirmado pela descompilação estática do programa original,
  mas ainda falta validar um quadro no hardware físico. A transmissão usa o
  perfil paisagem 480 × 320 do layout atual.
- A GPU é consultada por NVML NVIDIA, então o app é compatível apenas com
  placas NVIDIA; métricas de GPU AMD/Intel aparecem como N/D. A temperatura da
  CPU passa a ser lida da zona térmica ACPI via WMI (`MSAcpi_ThermalZoneTemperature`,
  exposta por `sysinfo::Components`), com fallback `CPU: N/D` quando a placa não
  publica zona utilizável. A previsão é de fonte
  pública Open-Meteo; a cidade configurada é enviada para geocodificação e
  previsão, mas nenhuma telemetria do PC é transmitida.
- O gráfico mantém amostras em memória por até uma hora e mostra no máximo os
  dois minutos mais recentes. O histórico é reiniciado ao fechar o aplicativo.
- Brilho é aplicado aos pixels, não à iluminação do painel; a rotação de 180°
  também é aplicada à imagem transmitida.

## Verificação

Executar `cargo fmt --check`, `cargo test`, Clippy e `node --check
frontend\app.js`. Os testes cobrem validação de configuração, identidade USB,
cabeçalho de quadro, orientação, conversão RGB565 e validação da imagem PNG,
além de cálculo de taxa de rede e escape CSV. O build/execução do app Tauri
requer o toolchain Windows e WebView2 Runtime.
