# Turing Smart Screen — Rust + HTML

Versão inicial independente para o display Turing de 3,5 polegadas. O backend
é Rust; as telas são HTML/CSS/JavaScript. O projeto contém cópias das artes
disponíveis em `assets`.

## Executar no Windows

Com Rust stable instalado, abra o PowerShell:

```powershell
cd C:\Users\rebou\Projetos\UsbMonitorRust
cargo run
```

Abra <http://127.0.0.1:47825>. O servidor escuta apenas em loopback. O
aplicativo identifica o adaptador USB por `VID_1A86` e `PID_5722`; conectado
nesta máquina, ele apareceu como `COM3`.

## Implementado

- Tela Monitor, Temas e Configurações, responsivas e em português.
- Prévia local usando imagens fornecidas pelo usuário.
- Identificações personalizadas e independentes para CPU, GPU e RAM, refletidas
  na prévia e salvas no navegador.
- Detecção periódica do adaptador USB-serial e exibição da porta COM.
- Abrir/fechar a porta serial com seleção explícita de velocidade.
- Erros de enumeração/abertura da porta apresentados à interface.
- Métricas reais de CPU, RAM, frequência da CPU, tráfego da rede e uptime.
- Utilização, temperatura e VRAM da GPU NVIDIA via NVML.
- Cartões destacados de CPU, RAM e GPU no topo da página Monitor, visíveis
  antes dos controles seriais e da prévia do display.
- Indicador de status/horário da última atualização das métricas ao vivo.

As métricas aparecem automaticamente; não é necessário abrir a serial. A porta
serial só precisa ser aberta para uma integração futura, e abrir a porta
**não transmite nenhum byte**. Temperatura da CPU e clima ainda não são obtidos;
clima exige configurar uma cidade e um provedor. Métricas de GPU ficam
indisponíveis em computadores sem driver NVIDIA/NVML. Preferências visuais ficam
no navegador; inicialização do Windows e envio das telas ao display não estão
implementados enquanto o protocolo serial continuar desconhecido.

Leia `RETOMADA.md` para o registro da engenharia reversa, evidências, limitações
e próximos passos; mantenha esse documento atualizado ao alterar o projeto.
