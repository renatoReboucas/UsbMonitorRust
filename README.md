# UsbMonitor Rust

Aplicativo desktop para Windows, implementado com Rust + Tauri 2. A interface é
HTML/CSS/JavaScript em `frontend`; telemetria, serial, persistência, inicialização
automática, bandeja e exportação são implementadas em `src-tauri`.

## Executar

Requisitos: Rust stable, Microsoft C++ Build Tools e WebView2 Runtime
(normalmente presente no Windows 10/11).

```powershell
cd C:\Users\rebou\Projetos\UsbMonitorRust
cargo run
```

O comando compila e abre a janela desktop; não inicia servidor HTTP nem porta
de rede. Para gerar instaladores MSI/NSIS, instale o Tauri CLI v2 e execute
`cargo tauri build`.

## Recursos

- CPU, memória, rede e uptime coletados localmente a cada segundo.
- Utilização, temperatura e VRAM NVIDIA via NVML, quando o driver NVIDIA
  disponibiliza `nvml.dll`; métricas indisponíveis são apresentadas como N/D.
- Gráfico com até dois minutos visíveis; o buffer em memória e a exportação
  guardam até uma hora de amostras.
- Preferências validadas e persistidas em JSON no diretório de configuração do
  aplicativo; inicialização do Windows configurável.
- Bandeja do sistema: clique ou “Abrir UsbMonitor” mostra a janela; fechar a
  janela a oculta; “Encerrar” finaliza o aplicativo.
- Exportação das amostras para CSV em `Downloads\UsbMonitorRust`.
- Detecção opcional do adaptador USB serial `VID_1A86:PID_5722`, com abertura e
  fechamento explícitos.

## Limites conhecidos da engenharia reversa

O código anterior deste projeto abria a serial, mas não enviava nem recebia
bytes. A interface atual mantém esse limite de segurança: não altera o display
físico. O protocolo, baud rate correto e formato das telas ainda precisam de
captura passiva e confirmação antes de implementar transmissão. A identificação
USB, isoladamente, não comprova o protocolo.

Esta versão obtém métricas de GPU pela NVML NVIDIA. Métricas equivalentes de
GPU Intel/AMD e temperatura da CPU não foram implementadas. Clima também não
tem provedor configurado; cidade/previsão não é consultada nem enviada.
Brilho, tema e rotação são preferências/previsualização da interface, não
comandos enviados ao display.

Consulte `RETOMADA.md` para os achados do código substituído, o estado da
engenharia reversa do programa original e as evidências/limitações conhecidas.
