; Hooks do instalador NSIS do UsbMonitorRust.
;
; É aqui que o serviço do LibreHardwareMonitor — o que dá a temperatura da CPU —
; é instalado junto com o aplicativo, a partir do MSI que viaja no pacote
; ($INSTDIR\lhm-service.msi, veja THIRD-PARTY-NOTICES.md).
;
; Isso funciona aqui e não funcionaria em um MSI: o Windows Installer mantém o
; mutex _MSIExecute durante toda a instalação, e um custom action que chamasse o
; msiexec falharia com o erro 1618 ("outra instalação está em andamento"). Como o
; NSIS não é uma sessão do Windows Installer, ele pode executar o MSI do serviço
; — foi por isso que a entrega ficou só com o instalador NSIS.
;
; A elevação é pedida na hora (`runas`): o instalador NSIS do Tauri é por
; usuário e roda sem privilégios, enquanto o serviço é instalado para todos os
; usuários. Assim aparece um pedido de administrador; se for recusado, o
; aplicativo continua funcionando com a temperatura em N/D (dá para instalar
; depois pela opção da bandeja).
;
; O serviço não é tocado quando já existe, e uma falha aqui não desfaz a
; instalação do aplicativo.

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Services\LibreHardwareMonitorService" "ImagePath"
  StrCmp $0 "" 0 lhm_service_done
    IfFileExists "$INSTDIR\lhm-service.msi" 0 lhm_service_done
      DetailPrint "Instalando o suporte de temperatura da CPU (LibreHardwareMonitor)..."
      ExecShell "runas" "$SYSDIR\msiexec.exe" '/i "$INSTDIR\lhm-service.msi" /qb /norestart'
  lhm_service_done:
!macroend
