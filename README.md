# UsbMonitor Rust

Aplicativo desktop para Windows que acompanha o computador e transmite um painel
HTML para um display USB de 3,5″ (480 × 320). O backend é Rust com Tauri 2, em
`src-tauri`; a interface é HTML, CSS e JavaScript sem framework, em `frontend`,
carregada no WebView2. Tudo roda na máquina: a única consulta de rede é a
previsão do tempo.

A interface usa a marca "TURING SMART SCREEN" no cabeçalho; o nome de exibição do
aplicativo e do executável é UsbMonitorRust.

O aplicativo:

- coleta CPU, memória, rede e uptime a cada segundo e mantém o histórico em
  memória;
- lê a temperatura da CPU pela zona térmica ACPI via WMI
  (`MSAcpi_ThermalZoneTemperature`) e, quando a placa não publica uma zona
  utilizável, pelo serviço do LibreHardwareMonitor;
- lê uso, temperatura e VRAM da GPU por NVML (NVIDIA), ADL (AMD) e serviço do
  LibreHardwareMonitor, campo a campo, na ordem em que cada fonte responde;
- mostra a previsão atual do tempo via Open-Meteo, com a cidade configurável;
- exibe a prévia do layout em 480 × 320 e envia os quadros pela porta serial;
- desenha um gráfico de CPU, memória e GPU e exporta o histórico em CSV;
- fica na bandeja do sistema, com as métricas no menu e a opção de iniciar com o
  Windows.

## Requisitos

Para executar o aplicativo já instalado:

- Windows 10 ou 11, 64 bits;
- WebView2 Runtime (já vem no Windows 11 e na maioria dos Windows 10
  atualizados);
- adaptador USB serial `VID_1A86:PID_5722` para o display físico. Sem ele, o
  restante do aplicativo funciona normalmente.

Para compilar:

- Rust stable com o alvo MSVC (`x86_64-pc-windows-msvc`);
- Microsoft C++ Build Tools (linker `link.exe`);
- o serviço do LibreHardwareMonitor é opcional: o instalador já traz
  `lhm-service.msi` e o aplicativo funciona com `CPU: N/D` sem ele.

## Instalação

### Pelo instalador

Execute `UsbMonitorRust_1.2.0_x64-setup.exe`. A instalação é por usuário e não
exige privilégios. No fim do setup, o hook `src-tauri/nsis/hooks.nsh` chama
`msiexec /i lhm-service.msi /qb /norestart` com elevação para instalar o serviço
de temperatura, que vale para todos os usuários; é por isso que aparece um
pedido de administrador. Recusar não quebra a instalação, e o aplicativo segue
com a temperatura em `CPU: N/D`.

O instalador copia para a pasta de instalação o executável, o `lhm-service.msi`
e o `THIRD-PARTY-NOTICES.md`.

Desinstalar o UsbMonitor não remove o serviço do LibreHardwareMonitor: ele é um
produto à parte, pode ter sido instalado antes e tem a própria entrada em
"Aplicativos e recursos".

### Sem instalador

O `usb-monitor-rust.exe` pode ser executado direto, de qualquer pasta. Nesse
caso o serviço de temperatura não é instalado junto; se o `lhm-service.msi` for
copiado para a mesma pasta do executável, a opção **"Instalar suporte de
temperatura da CPU…"** do menu da bandeja instala o serviço depois.

## Configuração

As preferências ficam em `%APPDATA%\br.com.usbmonitorrust\settings.json`. A
gravação é atômica (arquivo temporário e `.bak`) e os valores passam por
validação antes de serem salvos.

| Campo | Efeito |
| --- | --- |
| `displayTemplate` | layout HTML da prévia e da transmissão (letras minúsculas, números e hífen, até 48 caracteres) |
| `palette` | tema da interface: `latte`, `frappe`, `macchiato`, `mocha`, `console` ou `retro` |
| `brightness` | brilho de 0 a 100% da prévia e da imagem enviada |
| `rotation` | 0° ou 180° na prévia e no quadro transmitido |
| `mirror` | inverte a orientação da prévia e do quadro transmitido |
| `accentColor` | cor de destaque no formato `#RRGGBB` |
| `cpuCustom` `gpuCustom` `ramCustom` | rótulos dos cartões, até 24 caracteres cada |
| `city` | cidade da previsão, no formato `Cidade, Estado, País` (até 100 caracteres) |
| `startWithWindows` | inicia o aplicativo ao entrar na conta |
| `refreshIntervalSecs` | intervalo de envio ao display: 3, 5, 10, 15, 30 ou 60 segundos |

O layout padrão é o `console`, e a paleta padrão é `mocha`.

### Temperatura da CPU

A primeira tentativa é a zona térmica ACPI (`sysinfo::Components` sobre WMI),
que não exige nada instalado. Muitas placas desktop, em especial AM4/Ryzen, não
publicam uma zona utilizável; nesses casos a leitura passa a vir do serviço do
LibreHardwareMonitor, que carrega o driver de kernel e responde pelo named pipe
`\\.\pipe\LHMLibreHardwareMonitorService`. O aplicativo não precisa de
privilégios: quem lê os sensores é o serviço.

Para conferir se o serviço está no ar:

```powershell
Get-Service LibreHardwareMonitorService | Select-Object Name, Status, StartType
```

Sem o serviço o valor aparece como `CPU: N/D`. Se ele for instalado com o
aplicativo aberto, não é preciso reiniciar: a conexão é tentada de novo a cada
30 segundos. Quando a leitura vem do serviço, o valor exibido é a maior
temperatura válida entre os sensores da CPU; leituras ausentes (`NaN`), zeradas
ou negativas são descartadas, e o `Distance to TjMax` fica de fora por ser uma
distância até o limite.

### GPU

Cada campo (nome, uso, temperatura, VRAM usada e VRAM total) é preenchido pela
primeira fonte que responder:

1. **NVML** (`nvml.dll`), em máquinas com driver NVIDIA;
2. **ADL** (`atiadlxx.dll`), em máquinas com driver AMD;
3. **serviço do LibreHardwareMonitor**, para o que as fontes nativas não
   cobrirem. A GPU pelo serviço só é pedida quando o NVML não está disponível,
   para manter a consulta leve.

O que nenhuma fonte informar aparece como `N/D`.

### Clima

A previsão usa as APIs públicas do Open-Meteo (geocodificação e previsão
atual), com cache de 15 minutos. O formato aceito é `Cidade, Estado, País`; só o
nome da cidade é enviado ao serviço, e nenhuma métrica do computador sai da
máquina.

## Execução

Rodar em modo de desenvolvimento, a partir da raiz do repositório:

```powershell
cargo run
```

Compilar a versão otimizada:

```powershell
cargo build --release
```

O binário fica em `target\release\usb-monitor-rust.exe` e pode ser executado
direto:

```powershell
.\target\release\usb-monitor-rust.exe
```

Gerar o instalador NSIS (o bundler baixa o NSIS na primeira vez):

```powershell
cargo tauri build
```

A saída é `target\release\usb-monitor-rust.exe` e
`target\release\bundle\nsis\UsbMonitorRust_1.2.0_x64-setup.exe`. Se o comando
`cargo tauri` não existir, instale a CLI com
`cargo install tauri-cli --version "^2"`.

### Bandeja

Fechar a janela apenas a oculta. O menu da bandeja mostra CPU, RAM, GPU, CPU
TEMP, GPU TEMP, rede e uptime, atualizados a cada cinco segundos, e oferece
"Abrir UsbMonitor", "Instalar suporte de temperatura da CPU…", "Liberar porta
serial" e "Encerrar". O clique esquerdo no ícone abre a janela.

### Transmissão para o display

O protocolo observado no programa original é usado sem alteração: 115200 baud,
8N1, DTR e RTS ativos e sem handshake. Ao iniciar, o aplicativo envia a
orientação horizontal (comando 121) e depois cada quadro com o comando 197, com
pixels RGB565 em little-endian, em 480 × 320 (307.200 bytes de pixel por
quadro).

Para transmitir, selecione a porta e clique em "Transmitir para display". Use
"Enviar agora" para antecipar uma atualização e "Parar transmissão" para fechar
a porta. Como a COM é exclusiva no Windows, feche o programa original do display
e terminais seriais antes de transmitir. Se aparecer acesso negado na porta,
verifique também outra instância deste aplicativo: fechar a janela só a oculta
na bandeja, então use "Liberar porta serial" ou "Encerrar".

## Estrutura do Projeto

```
UsbMonitorRust/
├─ Cargo.toml                  workspace; o único membro é src-tauri
├─ Cargo.lock
├─ README.md
├─ RETOMADA.md                 engenharia reversa, decisões e limites conhecidos
├─ THIRD-PARTY-NOTICES.md      licenças do que é redistribuído
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ build.rs
│  ├─ tauri.conf.json          janela, bundle NSIS e resources
│  ├─ nsis/hooks.nsh           instala o lhm-service.msi ao fim do setup
│  ├─ icons/                   ícone do aplicativo e da bandeja
│  ├─ lhm/lhm-service.msi      serviço de temperatura redistribuído (release 0.0.0)
│  └─ src/
│     ├─ main.rs               builder do Tauri, comandos e estado compartilhado
│     ├─ telemetry.rs          coleta de 1 em 1 segundo, histórico e exportação CSV
│     ├─ cpu.rs ram.rs         leitura de CPU e memória (sysinfo)
│     ├─ gpu.rs amd.rs         GPU nativa por NVML e por ADL
│     ├─ lhm.rs                serviço do LibreHardwareMonitor: CPU (e GPU de reserva)
│     ├─ serial.rs             porta COM, PNG para RGB565 e envio dos quadros
│     ├─ config.rs             validação e persistência das preferências
│     ├─ startup.rs            inicialização automática com o Windows
│     ├─ tray.rs               ícone, menu e métricas da bandeja
│     └─ weather.rs            Open-Meteo, com cache
└─ frontend/
   ├─ index.html styles.css    interface do aplicativo
   ├─ app.js                   comandos Tauri, prévia, gráfico e captura do quadro
   ├─ icons.js                 ícones Lucide embutidos (offline)
   ├─ fonts/                   VT323 e Press Start 2P (offline)
   └─ displays/                layouts 480 × 320 (HTML, CSS, runtime.js e templates.json)
```

Para criar um layout, copie `frontend/displays/reference.html` e
`reference.css`, renomeie os dois para o mesmo `<id>` e registre ID, nome e
descrição em `frontend/displays/templates.json`. Mantenha o `runtime.js`, que
recebe as métricas por `postMessage`. O ID aceita letras minúsculas, números e
hífen.

## Tecnologias Utilizadas

| Dependência | Uso no projeto |
| --- | --- |
| `tauri` 2 (`tray-icon`, `image-png`) | janela, comandos, bandeja e ícones |
| `tauri-build` 2 | build script do Tauri |
| `tauri-plugin-autostart` 2 | iniciar com o Windows |
| `sysinfo` 0.33 | CPU, memória, rede, uptime e sensores ACPI |
| `serialport` 4 | porta COM do display |
| `png` 0.17 | decodificação do quadro capturado em PNG |
| `libloading` 0.8 | carrega `nvml.dll` e `atiadlxx.dll` em tempo de execução |
| `lhm-client` 0.0.0 | cliente do serviço do LibreHardwareMonitor |
| `tokio` 1 (`rt`, `net`, `io-util`, `time`) | runtime usado pelo cliente do serviço |
| `reqwest` 0.12 (`json`, `rustls-tls`) | Open-Meteo |
| `serde` 1 e `serde_json` 1 | serialização das métricas e das preferências |

No frontend não há gerenciador de pacotes nem etapa de build: os arquivos são
consumidos como estão. Os ícones do Lucide (`lucide-static` 0.460.0) estão
embutidos em `frontend/icons.js`, e as fontes VT323 e Press Start 2P (desktop) e
Doto (layout `console`) são arquivos WOFF2 locais, para o aplicativo funcionar
sem rede.

## Observações Importantes

- A via ADL (AMD) foi escrita a partir do cabeçalho público da biblioteca e
  ainda não foi validada em hardware AMD; o serviço do LibreHardwareMonitor
  cobre as GPUs que ela não responder.
- A versão `0.0.0` do `lhm-client` é intencional: é a que fala o protocolo do
  instalador publicado pelo projeto. Da `0.1.0` em diante o serviço trocou as
  mensagens do pipe e não funciona com este aplicativo.
- Só o instalador NSIS é gerado. Um MSI não consegue instalar outro MSI (o
  Windows Installer mantém o mutex `_MSIExecute` e o `msiexec` filho falha com o
  erro 1618), e é o NSIS que instala o serviço de temperatura junto.
- Brilho, rotação e inversão alteram a imagem enviada, não a iluminação física
  do painel.
- O histórico fica em memória (até 3.600 amostras, uma hora) e é zerado ao
  fechar o aplicativo. O gráfico desenha no máximo os dois minutos mais
  recentes. A exportação CSV vai para `Downloads\UsbMonitorRust`.
- Os seletores "Sensor de temperatura da CPU" e "Adaptador de rede" são salvos
  com as preferências, mas ainda não alteram a coleta: a temperatura usa a maior
  leitura válida disponível e a rede soma todas as interfaces.
- Os layouts em `frontend/displays` são HTML e JavaScript executados no WebView
  local. Use apenas layouts que você mesmo criou ou revisou.
- Créditos e licenças do serviço redistribuído, das fontes e dos ícones estão em
  [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md). Os achados da engenharia
  reversa do programa original e os limites ainda abertos estão em
  [RETOMADA.md](RETOMADA.md).
