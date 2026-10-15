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

Para gerar o instalador do Windows (NSIS), execute:

```powershell
npx @tauri-apps/cli build
```
*(ou instale via Cargo: `cargo install tauri-cli --version "^2"` e use `cargo tauri build`)*

O instalador será gravado em:
- `target\release\bundle\nsis\UsbMonitorRust_0.2.0_x64-setup.exe`

Ele também leva, para a pasta de instalação, o `lhm-service.msi` que dá a
temperatura da CPU (veja **Dependência opcional** abaixo). Esse arquivo fica
versionado em `src-tauri/lhm/`, com créditos e licenças em
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Recursos

- CPU, memória, rede e uptime coletados localmente a cada segundo.
- Temperatura da CPU pela zona térmica ACPI via WMI
  (`MSAcpi_ThermalZoneTemperature`). Em placas que não publicam uma zona
  utilizável, a leitura passa a vir do serviço do LibreHardwareMonitor
  (ver **Dependência opcional: temperatura da CPU** abaixo) e, sem ele, o valor
  é apresentado como `CPU: N/D`.
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

## Dependência opcional: temperatura da CPU (LibreHardwareMonitor)

Desktops — em especial placas AM4/Ryzen — normalmente **não publicam** a zona
térmica ACPI, então a leitura direta por WMI não tem o que ler e a temperatura
da CPU fica `CPU: N/D`. Para esses casos o UsbMonitor consulta os sensores do
[LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor)
através do serviço
[LibreHardwareMonitorService](https://github.com/jacobtread/lhm-service),
que publica os dados no named pipe `\\.\pipe\LHMLibreHardwareMonitorService`.

Quem carrega o driver de kernel e faz a leitura é o **serviço**; o UsbMonitor
apenas pede o resultado pelo pipe. Por isso **este aplicativo não precisa rodar
como administrador** — mas o serviço precisa ser instalado uma vez, com
administrador.

### Como instalar

O instalador do serviço viaja **junto** com o instalador do UsbMonitor (o
arquivo `lhm-service.msi` é copiado para a pasta de instalação). Não é preciso
baixar nada: a instalação do serviço é feita no fim do setup.

O instalador do UsbMonitor é por usuário, enquanto o serviço é instalado para
todos; por isso aparece **um pedido de administrador** no fim da instalação. Se
você recusar, nada quebra — o aplicativo funciona com a temperatura em `N/D`.
Para instalar depois (ou para reinstalar o serviço, caso ele seja removido),
clique com o botão direito no ícone da bandeja e escolha **"Instalar suporte de
temperatura da CPU…"**; o Windows pede elevação uma vez.

Em qualquer um dos caminhos, o serviço `LibreHardwareMonitorService` passa a
subir junto com o Windows e fica aguardando consultas em segundo plano, sem
ícone nem janela.

### Compatibilidade

O UsbMonitor embute o cliente `lhm-client 0.0.0`, que fala **exatamente** o
protocolo do instalador publicado hoje (o release `0.0.0` do serviço). As versões
`0.1.0` em diante do serviço trocaram as mensagens do pipe (`Update`/
`GetHardware` viraram `UpdateAll`/`QueryHardware` e o formato mudou), então elas
**não** funcionam com esta versão do UsbMonitor: a temperatura continuaria em
`N/D`. Se um dia o serviço publicar um instalador novo, o cliente embutido
precisa ser atualizado junto.

### Como usar

- **Nada no dia a dia.** Com o serviço rodando, a temperatura da CPU aparece
  sozinha no painel, na dica do ícone da bandeja e na prévia do display.
- Se você instalar o serviço com o UsbMonitor **já aberto**, não precisa
  reiniciar: a conexão é tentada de novo a cada 30 segundos.
- Para conferir se o serviço está no ar:

```powershell
Get-Service LibreHardwareMonitorService | Select-Object Name, Status, StartType
```

### Observações e limitações

- O serviço **não é obrigatório**: sem ele o UsbMonitor funciona normalmente e
  mostra `N/D` na temperatura da CPU.
- É um programa de terceiros, redistribuído **sem nenhuma modificação** dentro do
  nosso instalador. Os créditos e as licenças (MIT do serviço, MPL-2.0 da
  LibreHardwareMonitorLib que ele embute) estão em
  [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md), arquivo que também é copiado
  para a pasta de instalação. O arquivo redistribuído é o release `0.0.0`, com
  SHA-256 `adf740c3…469d`.
- Desinstalar o UsbMonitor **não** remove o serviço: ele é um produto à parte,
  pode ter sido instalado por você antes e tem a própria entrada em
  "Aplicativos e recursos".
- A entrega tem só o instalador NSIS (`bundle.targets` no `tauri.conf.json`).
  Antes havia também um MSI, mas um MSI não consegue instalar outro MSI: o
  Windows Installer mantém o mutex `_MSIExecute` durante toda a instalação e um
  `msiexec` filho falha com o erro 1618 ("outra instalação está em andamento").
  Isso deixava o serviço dependente de um clique manual; com o NSIS os dois são
  instalados de uma vez.
- A zona térmica ACPI (WMI, sem software externo) é tentada primeiro; o serviço
  só é usado quando ela não devolve nada.
- A temperatura exibida é a **maior** leitura válida entre os sensores de
  temperatura da CPU (por exemplo `Core (Tctl/Tdie)` ou `CPU Package`). Leituras
  ausentes (`NaN`), zeradas ou negativas são descartadas, e o `Distance to TjMax`
  — que é uma distância até o limite, não uma temperatura — fica de fora.
- Só a CPU é habilitada no serviço (`cpu`, `motherboard` e `controller`); a GPU
  continua vindo da NVML e o resto do `sysinfo`, o que mantém a consulta leve.
- Se um sensor aparecer como `N/D` e você quiser inspecionar o que a máquina
  expõe, o aplicativo avulso
  <https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases> mostra
  a árvore de sensores. O UsbMonitor conversa com o **serviço**, não com o
  aplicativo avulso — mas os dois usam a mesma biblioteca por baixo.
- A leitura é feita no máximo a cada 5 segundos, junto com as demais
  temperaturas.

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
externo; em placas que não publicam uma zona utilizável ela passa a vir do
serviço do LibreHardwareMonitor (dependência opcional, detalhada acima) e, sem
ele, fica `CPU: N/D`. O brilho é aplicado
à imagem transmitida, mas não altera a iluminação física do painel; a rotação
de 180° é aplicada aos pixels enviados.

Os HTMLs em `frontend\displays` são renderizados localmente no WebView e a
imagem resultante é enviada à tela USB; páginas web não são executadas no
firmware do display. Só adicione arquivos HTML/JavaScript de layouts que você
mesmo criou ou revisou.

Consulte `RETOMADA.md` para os achados do código substituído, o estado da
engenharia reversa do programa original e as evidências/limitações conhecidas.
