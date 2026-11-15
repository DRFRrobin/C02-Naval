# Clics et saisie dans la fenêtre « C02 Naval » (outil de test).
# Coordonnées = pixels de la capture faite par shot.ps1.
# Commandes séparées par « ; » : click X Y ; rclick X Y ; drag X1 Y1 X2 Y2 ; type texte ; key ENTER ; wait MS
param([string]$Do)
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Runtime.InteropServices;
public class I {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(int f, int x, int y, int d, int e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public struct RECT { public int L, T, R, B; }
}
"@
[I]::SetProcessDPIAware() | Out-Null
$h = (Get-Process c02-naval | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1).MainWindowHandle
[I]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 150
$r = New-Object I+RECT
[I]::GetWindowRect($h, [ref]$r) | Out-Null
foreach ($cmd in ($Do -split ";")) { $cmd = $cmd.Trim()
  $a = $cmd -split ' ', 2
  switch ($a[0]) {
    'click' { $p = $a[1] -split ' '; [I]::SetCursorPos($r.L + [int]$p[0], $r.T + [int]$p[1]) | Out-Null; Start-Sleep -Milliseconds 60; [I]::mouse_event(2,0,0,0,0); Start-Sleep -Milliseconds 40; [I]::mouse_event(4,0,0,0,0) }
    'rclick' { $p = $a[1] -split ' '; [I]::SetCursorPos($r.L + [int]$p[0], $r.T + [int]$p[1]) | Out-Null; Start-Sleep -Milliseconds 60; [I]::mouse_event(8,0,0,0,0); Start-Sleep -Milliseconds 40; [I]::mouse_event(16,0,0,0,0) }
    'drag' { $p = $a[1] -split ' '; [I]::SetCursorPos($r.L + [int]$p[0], $r.T + [int]$p[1]) | Out-Null; Start-Sleep -Milliseconds 80; [I]::mouse_event(2,0,0,0,0);
             for ($i = 1; $i -le 10; $i++) { $x = [int]$p[0] + ([int]$p[2] - [int]$p[0]) * $i / 10; $y = [int]$p[1] + ([int]$p[3] - [int]$p[1]) * $i / 10; [I]::SetCursorPos($r.L + $x, $r.T + $y) | Out-Null; Start-Sleep -Milliseconds 40 }
             [I]::mouse_event(4,0,0,0,0) }
    'type' { [System.Windows.Forms.SendKeys]::SendWait($a[1]) }
    'key' { [System.Windows.Forms.SendKeys]::SendWait("{" + $a[1] + "}") }
    'wait' { Start-Sleep -Milliseconds ([int]$a[1]) }
  }
  Start-Sleep -Milliseconds 120
}
