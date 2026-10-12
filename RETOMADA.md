# Registro de retomada — UsbMonitor Rust

Atualizado em 2026-10-11. Este documento é o contexto durável para outra IA ou
desenvolvedor retomar o projeto. Atualize-o quando fatos, escopo, decisões,
implementação ou testes mudarem.

## Objetivo do usuário

Fazer engenharia reversa de `UsbMonitor.exe` e criar uma versão parecida em Rust
com HTML para o layout das telas. O usuário confirmou que quer também operar o
display físico (escopo `hardware`) e forneceu o identificador USB observado:

```text
USB\VID_1A86&PID_5722&REV_0100
USB\VID_1A86&PID_5722
```

No Windows, o dispositivo correspondente foi enumerado como:

```text
COM3 — Dispositivo Serial USB (COM3)
USB\VID_1A86&PID_5722\USB35INCHIPSV2
```

Identidade USB indica um adaptador serial USB, provavelmente CH340 pela família
VID/PID; isto **não** prova protocolo, baud rate ou modelo interno do display.

## Localização e conteúdo da entrega

Projeto principal: `C:\Users\rebou\Projetos\UsbMonitorRust`.
Material original do usuário (somente referência): 
`C:\Users\rebou\Downloads\35inchENG\35inchENG`.

- `Cargo.toml`: servidor Rust Axum, `serialport`, Tokio, arquivos estáticos.
- `src\main.rs`: API local, detecção VID/PID e abertura/fechamento serial.
- `web\index.html`, `web\styles.css`, `web\app.js`: telas e prévia.
- `assets\`: cópia das 38 imagens da pasta original `3.5`.
- `README.md`: como executar e escopo implementado.

## Evidência examinada do pacote

- `UsbMonitor.exe`: 1.609.848 bytes, PE x86-64/.NET/CIL.
- A inspeção estática managed identificou aplicação gerenciada, mas cobertura
  parcial: metadados/resources têm índices/bytes inválidos e excedem limites.
  Nenhum método CIL ficou disponível no `inspect-managed-members`.
- A análise estática explicitamente não executa o binário nem recupera código
  C#; portanto não foi observada execução nem comportamento de tela ao vivo.
- Strings embutidas indicam WinForms e componentes como `SerialPort`, uso de
  `BaudRate`, `DtrEnable`, `RtsEnable`, `Write`, USB HID e chamadas de sensores.
  Strings não estabelecem a sequência real de comandos nem qual transporte
  comanda o display.
- `guide.jpg`: documenta rotação 0/180°, seleção de tema, brilho, cores das
  barras de estado e campo customizado (exemplo 9900K). As telas de configurações
  incluem sensor CPU, placa de rede, cidade/clima, início automático,
  desativação de atualização e reset de sensor.
- `Theme Notes.jpg`: exemplos de telas com CPU/GPU (temperatura, uso,
  frequência), memória/VRAM, disco, tráfego de rede, data/hora e volume.
- `3.5\` contém 38 imagens, copiadas para `assets\`.
- `config\` contém temas `.data` binários/serializados; não foram decodificados
  nem alterados. `AppConfig.data` parece conter serialização .NET.
- `Driver\usbser\cdc.inf` documenta um INF genérico de USB Serial Device;
  não especifica protocolo do display.
- `DriverinStallationGuide.txt`: diz que em Windows 10 normalmente não é
  necessário instalar o driver; em certos Windows 7 deve ser indicado o driver.
- `Troubleshooting.txt`: fala de ajustar `code.ini` e instalar componente .NET
  ausente. O pacote atual tem `code.ini` com conteúdo `64`.
- Ferramenta de captura de UI nativa REA indisponível no host Windows em que foi
  analisado (perfil nativo citado pelo CLI exige macOS).
- Em 2026-10-11, o usuário enviou uma foto do próprio display para referência
  visual: tela paisagem azul, relógio/data e identidade no bloco esquerdo,
  previsão do tempo abaixo, cartões CPU/RAM/GPU empilhados à direita e uma
  barra inferior com uptime, tráfego de rede, Ethernet, temperatura e VRAM.

## Implementação atual

Servidor em `127.0.0.1:47825`, não exposto na rede:

- `GET /api/status`: enumera portas seriais e procura especificamente VID/PID
  `1A86:5722`; devolve porta, descrição e flag de abertura.
- `POST /api/connect`: valida que o nome da porta solicitado corresponde
  àquele VID/PID, valida baud rate positivo e abre a serial com timeout de
  500 ms.
- `POST /api/disconnect`: fecha a porta descartando o handle.
- `GET /assets/*`: serve as artes locais.
- arquivos HTML são servidos pela raiz.
- `GET /api/metrics`: fornece amostras locais do sistema.
- Coletor em thread atualiza a cada segundo CPU, RAM, frequência, interfaces
  de rede e uptime por `sysinfo`; NVML de `C:\Windows\System32\nvml.dll` fornece
  nome/utilização/temperatura/VRAM da GPU NVIDIA.
- A prévia segue a foto de referência; CPU, RAM, GPU, rede e uptime são
  alimentados pela API de métricas, e gráfico/barras atualizam com as amostras.
- Identidade do computador e versão do Windows são obtidas das APIs locais.
- Clima fica sem valor até configurar cidade e fonte. Temperatura da CPU não
  tem sensor consultado nesta implementação; GPU aparece como `N/D` se NVML não
  estiver disponível.
- A tela Configurações possui campos de identificação personalizada
  independentes para CPU, GPU e RAM (até 24 caracteres). Os textos são salvos
  em `localStorage` e aparecem como legenda dos respectivos cartões; isso muda
  apenas o rótulo.

**Segurança/limite deliberado:** nenhuma leitura ou escrita serial além da
abertura da porta está implementada. Não enviar quadros aleatórios ao display.
Baud rate inicial no formulário (115200) é apenas uma opção de UI, não foi
verificado contra o aparelho. CPU/RAM/GPU/rede/uptime são lidos localmente, mas
não são enviados ao display. Clima, brilho e rotação ainda não controlam o
hardware. Inicialização do Windows está desativada e preferências persistem
somente no navegador.

## Como executar/testar

Rust/Cargo foram encontrados no ambiente do usuário (`C:\Users\rebou\.cargo\bin`
está no PATH na sessão). Estado da verificação em 2026-10-11:

```powershell
cd C:\Users\rebou\Projetos\UsbMonitorRust
cargo fmt --check
cargo test
cargo run
```

Executados com sucesso: `cargo fmt --check`, `cargo test` (2 testes passaram),
`node --check web\app.js`. Smoke test HTTP serviu a tela e assets. `/api/status`
detectou COM3, porta fechada. `/api/metrics` foi verificada na máquina:
Ryzen 7 5700X, RAM 10,7/15,9 GB, NVIDIA GeForce RTX 5060 Ti, GPU 6–7%,
53–55°C, VRAM ~2,3/8 GB, interface de rede presente. Valores variam em tempo
real. Nenhum byte foi enviado ao display.

Uma primeira instância antiga não tinha a rota nova e manteve o EXE bloqueado;
foi encerrada por PID e o executável recompilado. Instância nova iniciada em
`target\debug\usb-monitor-rust.exe`, escutando em `127.0.0.1:47825`; página
aberta no navegador. Encerrar o terminal/processo mata o servidor, então se a
interface desaparecer verifique a porta e execute novamente `cargo run`.

Quando o usuário reportou os números vazios, a API ainda fornecia métricas
reais, mas o browser podia manter JavaScript anterior em cache. A página passou
a usar uma URL versionada para carregar JS/CSS novos. O status `DADOS AO VIVO`
mostra o instante da última atualização; falhas da API viram mensagem de erro.
O botão serial informa que a métrica do PC funciona sem abrir COM3. Na captura
do usuário, a tentativa de abrir COM3 resultou em `Acesso negado`; outro
processo pode estar usando a porta. Essa situação não bloqueia as métricas e
não é necessário abrir serial para visualizá-las.

Em resposta ao usuário não conseguir localizar os valores na tela, a página
Monitor agora mostra cartões destacados para CPU, RAM e GPU imediatamente após
o título, antes da configuração da porta serial. Cada cartão apresenta valor
grande, barra de uso, nome/frequência ou memória/temperatura e estado ao vivo.
Leituras continuam vindo de `/api/metrics`; não usam valores fictícios. O HTML
e JS usam cache-buster novo `?v=home-metrics-20261011-3`.
Verificado após a alteração: HTML/JS/CSS atualizados são servidos por HTTP 200;
os três cartões e a consulta de métricas estão presentes, e os testes Rust (2),
formatação e sintaxe JavaScript passam. A API retornou amostras reais (~21% CPU,
68% RAM, GPU RTX 5060 Ti ~6%, 55°C). Página reaberta em
`http://127.0.0.1:47825/?home-metrics=20261011-3`.

### Última ocorrência — mouse sem fio (2026-10-11)

O usuário relatou que o mouse via dongle parou após o início das alterações.
Por precaução, o processo `usb-monitor-rust.exe` (PID 17428, porta local
47825) foi encerrado. A API indicava `COM3` detectada, mas `port_open: false`.
O código abre apenas a serial identificada por VID/PID `1A86:5722` (display
CH340); não acessa dispositivos HID nem o dongle do mouse. A enumeração PnP
mostrou o mouse Logitech `046D:C077` e dongle `2DC8:3109` presentes/OK, mas o
dispositivo HID “Gaming Optical Mouse” (`1D57:FA60`) estava com status `Error`.
Isso é uma pista, não evidência de que o app causou a falha. Orientado reconectar
o dongle em outra USB e desligar/ligar o mouse. Servidor permanece parado até
nova decisão; confirmar o funcionamento do mouse antes de reiniciá-lo.

Para repetir, abra `http://127.0.0.1:47825` após `cargo run`. O hardware pode
deixar de ser enumerado se o UsbMonitor original já abriu COM3; feche o processo
original antes de abrir a mesma porta. Confirme erros na UI/console, não force
transmissão.

## Próximas tarefas sugeridas

1. Capturar tráfego USB/serial produzido pelo programa original quando aplica um
   tema (USBPcap/Wireshark ou ferramenta serial equivalente), guardando captura
   local e identificando enquadramento, baud, comandos, handshake e payload.
   Informar o usuário antes de instalar captura/drivers/privilégios; não mandar
   dados experimentais ao display.
2. Buscar decompilador .NET local ou disponível e tentar análise focalizada de
   `SerialPort`, `Write`, classes de tela, geração/criptografia dos `.data`.
   Não baixar nem distribuir conteúdo/segredos proprietários do binário.
3. Só após protocolo corroborado, escrever testes de parser/encoder e enviar
   primeiro um caso benigno conhecido; manter porta fechada e erros explícitos.
4. Configurar clima apenas após obter consentimento para serviço externo e a
   cidade desejada; nunca enviar localização sem autorização.
5. Implementar persistência/configuração do app e inicialização do Windows em
   mudanças separadas, com confirmação.
6. Atualizar este documento e README junto com cada mudança que altere estado.

## Restrições e decisões

- Projeto pedido em `C:\Users\rebou\Projetos`, não dentro da pasta do pacote.
- Telas originais não foram capturadas diretamente; layout é uma interpretação
  visual baseada nas imagens/documentação fornecidas.
- Não declarar uma réplica completa ou funcional no dispositivo até descobrir
  e testar protocolo, métricas e fluxo de temas.
- Não instalar driver ou ferramenta de captura sem necessidade/consentimento.
