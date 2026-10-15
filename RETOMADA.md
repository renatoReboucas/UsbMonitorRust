# Engenharia reversa e estado do projeto

Atualizado em 2026-10-15.

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
- `lhm.rs`: temperatura da CPU pelo serviço do LibreHardwareMonitor
  (`lhm-client`), usado só quando a zona térmica ACPI não devolve nada. Mantém
  um runtime tokio próprio, limite de tempo por requisição e reconexão periódica.
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
  oculta, e “Instalar suporte de temperatura da CPU…” abre o `lhm-service.msi`
  que acompanha o aplicativo (veja a seção sobre o instalador).
- Catppuccin Latte, Frappé, Macchiato e Mocha, mais a paleta própria Console
  (verde fósforo, definida no tema `console`), são paletas persistidas. Para
  adicionar um layout HTML, crie `<id>.html` em `frontend\displays` e inclua o
  ID e metadados em `templates.json`; `runtime.js` fornece os dados dinâmicos.
  O tema `console` usa a fonte local `frontend\displays\fonts\doto.woff2` em
  todos os textos para manter o visual de terminal sem depender de rede. Na
  captura para o display físico, `app.js` reescreve as URLs de fonte locais como
  data URI (para o `@font-face` resolver dentro do `foreignObject` do SVG) e
  desativa transições antes de ler os estilos, senão os medidores saem vazios.
  O botão de transmissão tem largura fixa (158 px) em todos os estados, com o
  ícone (play, stop, fechar, sincronizar) sempre visível; rótulo e ícone trocam
  sem redimensionar o cartão. Os rodapés dos quatro layouts antigos usam o
  formato literal `CPU: XX°C`, rede com uma casa decimal (`↑12.3M`) e valores
  encurtados de uptime (`3d12h`) e VRAM (`10/24G`), sem reticências.

## Serviço de temperatura embutido no instalador

O `lhm-service.msi` (release `0.0.0`, licença MIT) é versionado em
`src-tauri\lhm\` e copiado para a pasta de instalação pelo `bundle.resources` do
`tauri.conf.json`, junto com o aviso de terceiros (`THIRD-PARTY-NOTICES.md`, que
traz também o texto da MPL-2.0 da LibreHardwareMonitorLib embutida no serviço).
Assim não é preciso baixar nada depois de instalar.

Quem instala o serviço é o instalador **NSIS**, pelo hook
`src-tauri\nsis\hooks.nsh` (`NSIS_HOOK_POSTINSTALL`): o NSIS não é uma sessão do
Windows Installer, então pode executar o outro MSI. Como o instalador do Tauri é
por usuário (`installMode` padrão, `RequestExecutionLevel user`) e o serviço é
instalado para todos, o hook usa `ExecShell "runas" ... /qb`, que pede elevação
na hora; recusar não desfaz a instalação do aplicativo.

O instalador NSIS é o único gerado (`bundle.targets` no `tauri.conf.json`).
Antes saía também um MSI, e o encadeamento **não** funciona nele — isso foi
medido, não suposto: um custom action diferido (`Directory="INSTALLDIR"`,
`Impersonate="no"`) que executa `msiexec /i child.msi /qn` devolve **1618**
(`ERROR_INSTALL_ALREADY_RUNNING`) e o MSI filho não é instalado, porque o Windows
Installer mantém o mutex `_MSIExecute` durante toda a sequência de execução. O
mesmo teste confirmou que `[INSTALLDIR]` **é** expandido em ação diferida; a
limitação é só a instalação aninhada. Como o MSI deixava o serviço dependente de
um clique manual, a entrega ficou só com o NSIS.

A opção “Instalar suporte de temperatura da CPU…”, na bandeja, é o caminho de
conserto: abre o MSI que está ao lado do executável através do shell
(`cmd /c start ...`), o que faz o Windows pedir elevação como se fosse um duplo
clique. Serve quando a elevação foi recusada no setup ou quando o serviço foi
removido depois.

Desinstalar o UsbMonitor não remove o serviço: ele é um produto independente,
pode ter sido instalado antes pelo usuário e tem entrada própria em “Aplicativos
e recursos”. Os dois instaladores apagam apenas os arquivos que copiaram para a
pasta de instalação.

## Limites ainda abertos

- O protocolo foi confirmado pela descompilação estática do programa original,
  mas ainda falta validar um quadro no hardware físico. A transmissão usa o
  perfil paisagem 480 × 320 do layout atual.
- A GPU é consultada por NVML NVIDIA, então o app é compatível apenas com
  placas NVIDIA; métricas de GPU AMD/Intel aparecem como N/D. A temperatura da
  CPU é lida da zona térmica ACPI via WMI (`MSAcpi_ThermalZoneTemperature`,
  exposta por `sysinfo::Components`). Em placas que não publicam uma zona
  utilizável — confirmado nesta máquina (Ryzen 7 5700X/AM4: o namespace
  `MSAcpi_ThermalZoneTemperature` responde "não há suporte à operação") — a
  leitura passa a vir do **serviço** do LibreHardwareMonitor
  (`lhm-service`, cliente `lhm-client 0.0.0`, pipe
  `\\.\pipe\LHMLibreHardwareMonitorService`), que é quem carrega o driver de
  kernel; assim o app segue sem exigir administrador. Sem o serviço instalado o
  valor fica `CPU: N/D`. A versão 0.0.0 do cliente é deliberada: o único
  instalador publicado pelo projeto (release `0.0.0`) fala o protocolo
  `Update`/`GetHardware`; a partir de `0.1.0` o serviço passou a usar
  `UpdateAll`/`QueryHardware` e deixou de ser compatível. A previsão é de fonte
  pública Open-Meteo; a cidade configurada é enviada para geocodificação e
  previsão, mas nenhuma telemetria do PC é transmitida.
- O gráfico mantém amostras em memória por até uma hora e mostra no máximo os
  dois minutos mais recentes. O histórico é reiniciado ao fechar o aplicativo.
- Brilho é aplicado aos pixels, não à iluminação do painel; a rotação de 180°
  também é aplicada à imagem transmitida.

## Verificação

Executar `cargo fmt --check`, `cargo test`, Clippy e `node --check
frontend\app.js`; para os layouts, `node --check frontend\displays\runtime.js`.
Os testes cobrem validação de configuração, identidade USB, cabeçalho de quadro,
orientação, conversão RGB565, validação da imagem PNG, cálculo de taxa de rede,
escape CSV e a escolha da temperatura da CPU entre os sensores do
LibreHardwareMonitor (maior leitura válida, ignorando `Distance to TjMax`,
`NaN`/zero e sensores de hardware alheio, como a GPU). A integração com o serviço
foi validada de ponta a ponta contra um named pipe falso que responde com o
protocolo publicado (`SetOptions`, `Update`, `GetHardware`), sem instalar o
serviço. Nos instaladores gerados foram conferidas as tabelas `File`,
`Component` e `Directory` do MSI pelo objeto COM `WindowsInstaller.Installer`
(`lhm-service.msi`, 2.641.920 bytes, e `THIRD-PARTY-NOTICES.md` ficam em
`INSTALLDIR`) e o script NSIS compilado (`File /oname=...` em `$INSTDIR` e o
hook `NSIS_HOOK_POSTINSTALL` inserido ao fim da seção de instalação). O build e a
execução do app Tauri requerem o toolchain Windows e o WebView2 Runtime.
