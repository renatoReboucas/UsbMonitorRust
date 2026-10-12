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
registravam uma inspeção estática incompleta: metadados/recursos inconsistentes
impediram recuperar métodos CIL. Strings apontavam para WinForms, `SerialPort`,
USB HID, sensores e configurações de baud rate, mas não provavam o fluxo real,
o transporte usado para comandar o display ou o protocolo. A análise anotada
não executou o programa e não observou sua interface em tempo real.

O material de referência descrito nas notas incluía 38 artes de tema, imagens
de guia/configuração, opções de rotação, brilho, sensores e inicialização
automática. Essas fontes orientam a prévia, mas não comprovam os comandos
enviados ao hardware. O adaptador observado no Windows era `COM3`, com
`VID_1A86&PID_5722`; isso não comprova baud rate ou protocolo.

## Arquitetura atual

- `src-tauri\src\main.rs`: inicialização Tauri, comandos e estado compartilhado.
- `cpu.rs`, `ram.rs`, `gpu.rs`, `telemetry.rs`: coleta local de métricas e
  histórico circular de até 3.600 amostras (uma hora).
- `serial.rs`: enumeração, validação e abertura/fechamento explícitos da porta;
  nenhuma transmissão ou leitura serial.
- `config.rs`: validação e persistência de preferências JSON no diretório de
  configuração do aplicativo, com arquivo temporário e backup de recuperação.
- `startup.rs`: integração de inicialização automática do Windows.
- `tray.rs`: ícone e menu da bandeja; fechar a janela oculta o app.
- `frontend`: telas HTML/CSS/JavaScript e layouts 800 × 450 carregados em
  `iframe`; os layouts estão em `frontend\displays`, registrados em
  `templates.json` e recebem as métricas via `postMessage`.
- `weather.rs`: consulta Open-Meteo para geocodificação e previsão atual, com
  cache de 15 minutos e cidade/região editáveis; o padrão é São Paulo, SP.
- `tray.rs` também atualiza CPU, RAM, GPU, temperatura disponível, rede e uptime
  no menu/tooltip da bandeja, a cada cinco segundos.
- Catppuccin Latte, Frappé, Macchiato e Mocha são paletas persistidas. Para
  adicionar um layout HTML, crie `<id>.html` em `frontend\displays` e inclua o
  ID e metadados em `templates.json`; `runtime.js` fornece os dados dinâmicos.

## Limites ainda abertos

- O protocolo, o baud rate, o enquadramento e o transporte efetivo do display
  continuam desconhecidos. Não enviar comandos experimentais.
- A GPU é consultada por NVML NVIDIA; métricas de GPU AMD/Intel e temperatura
  da CPU ainda não estão disponíveis nesta versão. A previsão é de fonte
  pública Open-Meteo; a cidade configurada é enviada para geocodificação e
  previsão, mas nenhuma telemetria do PC é transmitida.
- O gráfico mantém amostras em memória por até uma hora e mostra no máximo os
  dois minutos mais recentes. O histórico é reiniciado ao fechar o aplicativo.
- Tema, brilho, rotação e rótulos personalizados alteram a prévia/configuração,
  não o estado físico do display.
- Os layouts HTML são prévias no WebView. Eles não substituem nem são
  transmitidos ao protocolo do display USB.

## Verificação

Executar `cargo fmt --check`, `cargo test` e `node --check frontend\app.js`.
Os testes cobrem validação de configuração, identidade USB observada, cálculo
de taxa de rede e escape CSV. O build/execução do app Tauri requer o toolchain
Windows e WebView2 Runtime.
