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

Para compilar a versão otimizada sem criar instalador:

```powershell
cd C:\Users\rebou\Projetos\UsbMonitorRust
cargo build --release
.\target\release\usb-monitor-rust.exe
```

O executável fica em
`C:\Users\rebou\Projetos\UsbMonitorRust\target\release\usb-monitor-rust.exe`.
Ele pode ser iniciado pelo PowerShell, pelo Explorador de Arquivos ou por um
atalho. O WebView2 Runtime precisa estar instalado no computador em que o app
será executado. As configurações ficam no diretório de configuração do usuário
e os CSVs são exportados para `Downloads\UsbMonitorRust`.

Para gerar um instalador Windows (NSIS/MSI), instale o Tauri CLI v2 e execute:

```powershell
cargo install tauri-cli --version "^2"
cargo tauri build
```

Os instaladores serão gravados em `target\release\bundle`. `target\` é uma
pasta de build e não deve ser copiada nem versionada junto ao código-fonte.

## Recursos

- CPU, memória, rede e uptime coletados localmente a cada segundo.
- Utilização, temperatura e VRAM NVIDIA via NVML, quando o driver NVIDIA
  disponibiliza `nvml.dll`; métricas indisponíveis são apresentadas como N/D.
- Gráfico com até dois minutos visíveis; o buffer em memória e a exportação
  guardam até uma hora de amostras.
- Preferências validadas e persistidas em JSON no diretório de configuração do
  aplicativo; inicialização do Windows configurável.
- Bandeja do sistema: clique ou “Abrir UsbMonitor” mostra a janela; fechar a
  janela a oculta; “Encerrar” finaliza o aplicativo. O menu e a dica da bandeja
  exibem CPU, RAM, GPU, temperatura disponível, rede e uptime.
- Exportação das amostras para CSV em `Downloads\UsbMonitorRust`.
- Detecção opcional do adaptador USB serial `VID_1A86:PID_5722`, com abertura e
  fechamento explícitos.
- Prévia do dashboard em proporção 800 × 450, com CPU, RAM, GPU, rede e clima;
  cidade editável no formato `Cidade, Estado, País`.
- Layouts da prévia definidos por HTML/CSS em `frontend\displays`; selecione
  um layout na página **Layouts HTML** ou na configuração rápida do Monitor.
  Para criar outro, copie `reference.html` e `reference.css` dentro de
  `frontend\displays`, renomeie os arquivos para o mesmo `<id>` e ajuste o link
  da folha de estilos e o título. Registre ID, nome e descrição em
  `frontend\displays\templates.json`. IDs aceitam letras minúsculas, números e
  hífens (até 48 caracteres); mantenha o script `runtime.js` para receber
  métricas por `postMessage` usando os IDs de elementos do exemplo.
- Previsão atual via Open-Meteo, iniciando em São Paulo, SP. A consulta envia o
  nome da cidade ao serviço público de geocodificação/previsão; nenhum dado de
  telemetria do computador é transmitido.
- Temas de interface Catppuccin Latte, Frappé, Macchiato e Mocha, com seleção
  persistente nas configurações.

## Limites conhecidos da engenharia reversa

O código anterior deste projeto abria a serial, mas não enviava nem recebia
bytes. A interface atual mantém esse limite de segurança: não altera o display
físico. O protocolo, baud rate correto e formato das telas ainda precisam de
captura passiva e confirmação antes de implementar transmissão. A identificação
USB, isoladamente, não comprova o protocolo.

Esta versão obtém métricas de GPU pela NVML NVIDIA. Métricas equivalentes de
GPU Intel/AMD e temperatura da CPU não foram implementadas. Brilho, tema,
paleta e rotação são preferências/previsualização da interface, não comandos
enviados ao display.

Os HTMLs em `frontend\displays` são layouts de pré-visualização executados
localmente no WebView; eles não são enviados à tela USB como páginas web. O
protocolo serial do equipamento ainda precisa ser confirmado. Só adicione
arquivos HTML/JavaScript de layouts que você mesmo criou ou revisou.

Consulte `RETOMADA.md` para os achados do código substituído, o estado da
engenharia reversa do programa original e as evidências/limitações conhecidas.
