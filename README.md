# UsbMonitor Rust

Aplicativo desktop para Windows, implementado com Rust + Tauri 2. A interface é
HTML/CSS/JavaScript em `frontend`; telemetria, serial, persistência, inicialização
automática, bandeja e exportação são implementadas em `src-tauri`.

## Executar

Requisitos: Rust stable, Microsoft C++ Build Tools e WebView2 Runtime
(normalmente presente no Windows 10/11).

### Rodar localmente em modo de desenvolvimento

No PowerShell:

```powershell
cd C:\Users\rebou\Projetos\UsbMonitorRust
cargo run
```

O comando compila e abre a janela desktop; não inicia servidor HTTP nem porta
de rede.

### Gerar e executar o `.exe` localmente

> [!TIP]
> **Atenção:** `cargo build --release` e a execução do binário são **dois comandos separados**. Não execute os dois na mesma linha sem separador (ou use `;` no PowerShell).

Para compilar a versão otimizada `.exe`:

```powershell
cd C:\Users\rebou\Projetos\UsbMonitorRust
cargo build --release
```

Depois de concluída a compilação, para rodar o `.exe`:

```powershell
.\target\release\usb-monitor-rust.exe
```

Ou em um único comando no PowerShell (separados por `;`):

```powershell
cargo build --release; .\target\release\usb-monitor-rust.exe
```

O executável gerado fica em:
`C:\Users\rebou\Projetos\UsbMonitorRust\target\release\usb-monitor-rust.exe`.
Ele pode ser iniciado pelo PowerShell, pelo Explorador de Arquivos (dando dois cliques) ou por um atalho na Área de Trabalho. O WebView2 Runtime precisa estar instalado no computador em que o app será executado. As configurações ficam no diretório de configuração do usuário (`%APPDATA%`) e os CSVs são exportados para `Downloads\UsbMonitorRust`.

Para gerar o instalador do Windows (NSIS/MSI), execute:

```powershell
npx @tauri-apps/cli build
```
*(ou instale via Cargo: `cargo install tauri-cli --version "^2"` e use `cargo tauri build`)*

Os instaladores serão gravados em:
- `target\release\bundle\nsis\UsbMonitorRust_0.2.0_x64-setup.exe` (Instalador padrão executável)
- `target\release\bundle\msi\UsbMonitorRust_0.2.0_x64_en-US.msi` (Pacote MSI do Windows)

## Recursos

- CPU, memória, rede e uptime coletados localmente a cada segundo.
- Temperatura da CPU pela zona térmica ACPI via WMI
  (`MSAcpi_ThermalZoneTemperature`). Em placas que não publicam uma zona
  utilizável, o valor é apresentado como `CPU: N/D`.
- Utilização, temperatura e VRAM NVIDIA via NVML, quando o driver NVIDIA
  disponibiliza `nvml.dll`; métricas indisponíveis são apresentadas como N/D.

> [!IMPORTANT]
> A leitura da GPU é compatível **apenas com placas NVIDIA**, pois depende da
> NVML (`nvml.dll`). GPU Intel/AMD não é suportada e aparece como N/D.
- Gráfico com até dois minutos visíveis; o buffer em memória e a exportação
  guardam até uma hora de amostras.
- Preferências validadas e persistidas em JSON no diretório de configuração do
  aplicativo; inicialização do Windows configurável.
- Bandeja do sistema: clique ou “Abrir UsbMonitor” mostra a janela; fechar a
  janela a oculta; “Encerrar” finaliza o aplicativo. A bandeja usa o mesmo
  ícone do aplicativo (`src-tauri/icons`), e o menu e a dica exibem CPU, RAM,
  GPU, CPU TEMP, GPU TEMP, rede e uptime. “Liberar porta
  serial” fecha uma COM aberta por esta instância, mesmo com a janela oculta.
- Exportação das amostras para CSV em `Downloads\UsbMonitorRust`.
- Detecção do adaptador USB serial `VID_1A86:PID_5722`, com transmissão real
  do layout selecionado para o display.
- Prévia e envio do dashboard em 480 × 320, com CPU, RAM, GPU, temperatura da
  CPU, rede e clima; a cidade é editável no formato `Cidade, Estado, País` e o
  rodapé de cada layout exibe `CPU: XX°C`.
- Layouts da prévia definidos por HTML/CSS em `frontend\displays`; selecione
  um layout na página **Layouts HTML** ou na configuração rápida do Monitor.
  Os layouts são `reference`, `compact`, `minimal`, `hud` e `console`; o
  `console` carrega a fonte Doto de `frontend\displays\fonts` (matriz de pontos)
  e usa a mesma fonte em todos os textos, então funciona offline. Ao transmitir
  para o display físico, o aplicativo embute a fonte como data URI e congela
  transições, para o quadro sair idêntico à prévia.
  Para criar outro, copie `reference.html` e `reference.css` dentro de
  `frontend\displays`, renomeie os arquivos para o mesmo `<id>` e ajuste o link
  da folha de estilos e o título. Registre ID, nome e descrição em
  `frontend\displays\templates.json`. IDs aceitam letras minúsculas, números e
  hífens (até 48 caracteres); mantenha o script `runtime.js` para receber
  métricas por `postMessage` usando os IDs de elementos do exemplo.
- Previsão atual via Open-Meteo, iniciando em São Paulo, SP. A consulta envia o
  nome da cidade ao serviço público de geocodificação/previsão; nenhum dado de
  telemetria do computador é transmitido.
- Temas de interface Catppuccin Latte, Frappé, Macchiato e Mocha, mais a paleta
  própria Console (verde fósforo), com seleção persistente nas configurações.

## Display físico e transmissão serial

A rotina serial do programa original foi decompilada localmente para confirmar
seu comportamento: 115200 baud, 8N1, DTR/RTS ativados e sem handshake; a
orientação usa o comando 121 e a imagem usa o comando 197, com pixels RGB565 em
little-endian. A resolução transmitida por esta interface é 480 × 320, igual
ao layout HTML selecionado. Entre o comando de imagem e os pixels são mantidos
os intervalos de 10 ms observados no programa original.

Ao iniciar a transmissão, o aplicativo configura a orientação horizontal,
rasteriza o HTML selecionado na resolução nativa e envia um quadro completo.
Depois, atualiza a tela no intervalo configurado (padrão de 5 segundos, ajustável
diretamente na tela inicial ou nas configurações para 3s, 5s, 10s, 15s, 30s ou 1 min).
Um quadro completo contém 307.200 bytes de pixel. Use **Enviar agora** para antecipar
uma atualização ou **Parar transmissão** para fechar a porta COM. Como a COM é
exclusiva no Windows, feche o programa original e terminais seriais antes de transmitir.

Os templates HTML executam no WebView local. Para recursos externos usados no
display, prefira imagens embutidas no próprio HTML/CSS; scripts de templates
customizados são executados localmente no aplicativo.

Se o Windows informar que COM3 está ocupada/acesso negado, verifique também o
programa original do display, terminais seriais e outras instâncias deste app.
Fechar a janela do UsbMonitor apenas o oculta na bandeja; use “Liberar porta
serial” para fechar a conexão da instância atual ou “Encerrar” para sair.

Esta versão obtém métricas de GPU pela NVML NVIDIA, portanto é compatível
**apenas com GPUs NVIDIA**; métricas equivalentes de GPU Intel/AMD aparecem como
N/D. A temperatura da CPU é lida da zona térmica ACPI via WMI, sem software
externo, mas pode ficar indisponível (`CPU: N/D`) em placas que não publicam
uma zona utilizável. O brilho é aplicado
à imagem transmitida, mas não altera a iluminação física do painel; a rotação
de 180° é aplicada aos pixels enviados.

Os HTMLs em `frontend\displays` são renderizados localmente no WebView e a
imagem resultante é enviada à tela USB; páginas web não são executadas no
firmware do display. Só adicione arquivos HTML/JavaScript de layouts que você
mesmo criou ou revisou.

Consulte `RETOMADA.md` para os achados do código substituído, o estado da
engenharia reversa do programa original e as evidências/limitações conhecidas.
